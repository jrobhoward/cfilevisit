// Copyright (c) 2018 James Howard <jrobhoward@gmail.com>
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
//
// src/lib.rs
use std::fs;
use std::fs::Metadata;
use std::path::Path;
use std::path::PathBuf;
use std::string::String;
use std::sync::{Arc, Condvar, Mutex};
use std::thread;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate time;

/// Simple (example) VisitCallback implementation.
pub mod countingvisitor;

/// Set of functions evaluated during `visit_paths`.
///
/// # Panics
/// If any of the callbacks cause a panic, the release build of this module is configured to abort.
pub trait VisitCallback: Sized + Send + Sync {
    /// Called when filesystem walk encounters a file.
    ///
    /// Must be a traiditonal file (i.e. not symlink, device, or fifo).
    fn on_file_visit(&mut self, PathBuf, &fs::Metadata) -> () {}

    /// Called when filesystem walk encounters a non-file (e.g. symlink, device, fifo, socket).
    fn on_nonfile_visit(&mut self, PathBuf, &fs::Metadata) -> () {}

    /// Called before filesystem walk processes a directory.
    ///
    /// The directory will be processed when `true` is returned.
    /// The directory will be skipped when `false` is returned.
    fn on_dir_enter(&mut self, PathBuf, &fs::Metadata) -> bool {
        true
    }

    /// Called after filesystem walk finishes processing a directory.
    ///
    /// By "finish" we mean that all of its files have been processed and subdirectories have been enqueued as additional work.
    /// This callback is also evaluated when directories are skipped or cannot be read (e.g. permission denied).
    fn on_dir_exit(&mut self, PathBuf, &fs::Metadata) -> () {}
}

fn process_dir<T: VisitCallback + 'static>(
    path: &String,
    vis: &std::sync::Arc<Mutex<T>>,
    workitem_metadata: &Metadata,
    paths: &Mutex<Vec<String>>,
    cond_workpresent: &Condvar,
) -> () {
    let directory = Path::new(path);
    let should_process_dir = vis.lock()
        .unwrap()
        .on_dir_enter(directory.to_path_buf(), &workitem_metadata);

    if !should_process_dir {
        vis.lock()
            .unwrap()
            .on_dir_exit(directory.to_path_buf(), &workitem_metadata);
        return;
    }
    let dir_entries = match fs::read_dir(directory) {
        Result::Ok(val) => val,
        _ => {
            vis.lock()
                .unwrap()
                .on_dir_exit(directory.to_path_buf(), &workitem_metadata);
            return;
        }
    };
    for dir_entry in dir_entries {
        let entry = dir_entry.unwrap();
        if let Ok(direntry_metadata) = entry.metadata() {
            let path = entry.path();
            if direntry_metadata.file_type().is_dir() {
                paths
                    .lock()
                    .unwrap()
                    .push(path.into_os_string().into_string().unwrap());
                cond_workpresent.notify_all();
            } else if direntry_metadata.file_type().is_file() {
                vis.lock().unwrap().on_file_visit(path, &direntry_metadata);
            } else {
                vis.lock()
                    .unwrap()
                    .on_nonfile_visit(path, &direntry_metadata);
            }
        } else {
            debug!("Couldn't get metadata for {:?}", entry.path());
            continue;
        }
        vis.lock()
            .unwrap()
            .on_dir_exit(directory.to_path_buf(), &workitem_metadata);
    }
}

/// Perform a concurrent filesystem walk over a set of paths.
///
/// * `paths` - A vector containing a set of paths to initiate the filesystem walk.
/// * `num_workers` - The number of concurrent worker threads used to walk the filesystem.
/// * `visitor_callback` - An arc-wrapperd VisitCallback implementation, supplied by the caller.
pub fn visit_paths<T: VisitCallback + 'static>(
    paths: Vec<String>,
    num_workers: usize,
    visitor_callback: Arc<Mutex<T>>,
) -> () {
    debug!(
        "Concurent file visit started: num_workers={}, paths={:?}",
        num_workers, paths
    );
    let thread_args = Arc::new((
        Mutex::new(paths),        // initial list of dirs/files to process
        visitor_callback.clone(), // VisitorCallback impl
        Condvar::new(),           // condition: work present, requires processing
        Mutex::new(num_workers),  // number of active worker threads
        Condvar::new(),           // condition: work finished, check if done
        Mutex::new(false),        // set to true when all work is finished
    ));

    let mut workers = Vec::with_capacity(num_workers);
    for _i in 0..num_workers {
        let mut args = thread_args.clone();
        workers.push(thread::spawn(move || {
            let &(
                ref paths,
                ref vis,
                ref cond_workpresent,
                ref active_workercount,
                ref cond_workfinished,
                ref all_threads_finished,
            ) = &*args;
            loop {
                let result = paths.lock().unwrap().pop();
                match result {
                    Some(ref path) => {
                        // read metadata, do not resolve symlinks
                        if let Ok(workitem_metadata) = fs::symlink_metadata(path) {
                            if workitem_metadata.file_type().is_dir() {
                                process_dir(path, vis, &workitem_metadata, paths, cond_workpresent);
                            } else if workitem_metadata.file_type().is_file() {
                                let file_path = Path::new(path);
                                vis.lock()
                                    .unwrap()
                                    .on_file_visit(file_path.to_path_buf(), &workitem_metadata);
                            } else {
                                debug!(
                                    "Unsupported file type={:?} for path={}",
                                    workitem_metadata.file_type(),
                                    path
                                );
                            }
                        } else {
                            debug!("Unable to read metadata for path={}", path);
                        }
                    }
                    None => {
                        // decrement the number of active workers, going to sleep
                        {
                            let mut active_workers = active_workercount.lock().unwrap();
                            *active_workers = *active_workers - 1;
                            cond_workfinished.notify_all();
                        }

                        {
                            let d = paths.lock().unwrap();
                            {
                                let mut is_finished = all_threads_finished.lock().unwrap();
                                if *is_finished {
                                    return;
                                }
                            }
                            let _unused = cond_workpresent.wait(d);
                        }

                        // woke up, increment the number of active workers
                        {
                            let mut active_workers = active_workercount.lock().unwrap();
                            {
                                let mut is_finished = all_threads_finished.lock().unwrap();
                                if *is_finished {
                                    return;
                                }
                            }
                            *active_workers = *active_workers + 1;
                        }
                    }
                }
            }
        }));
    }

    // Main thread continues here...
    {
        let &(
            ref paths,
            ref _visitor_callback,
            ref cond_workpresent,
            ref active_workercount,
            ref cond_workfinished,
            ref all_threads_finished,
        ) = &*thread_args;
        loop {
            let active_workercount = active_workercount.lock().unwrap();
            {
                let paths = paths.lock().unwrap();
                trace!(
                    "Main thread waiting for workers: path_count={}, active_workcount={}",
                    paths.len(),
                    active_workercount
                );
            }

            // When 0 workers are active, set finished & wake everyone up
            if *active_workercount == 0 {
                {
                    let paths = paths.lock().unwrap();
                    if paths.len() != 0 {
                        warn!(
                            "Unexpected state: zero active workers when work exists, paths={:?}",
                            paths
                        );
                    }
                    let mut is_finished = all_threads_finished.lock().unwrap();
                    *is_finished = true;
                }
                cond_workpresent.notify_all(); // wake all threads up so they may finish
                break;
            }

            // At least one worker is active, wait for more work
            let _unused = cond_workfinished.wait(active_workercount);
        }
    }

    trace!("Main thread waiting to join worker threads");
    let mut joined_count = 0;
    for worker in workers {
        let _ = worker.join();
        joined_count += 1;
        trace!("Joined {}", joined_count);
    }
    trace!("Main thread finished joining worker threads");
}
