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
// src/countingvisitor/mod.rs
use std::fs;
use std::path::PathBuf;
use std::thread;
extern crate time;
use std::collections::HashSet;
use std::os::unix::fs::MetadataExt;

use VisitCallback;

/// An example implementation of VisitCallback that counts the number of files found.
///
/// Each file that is found is also logged at `trace` level, along with its device and inode numbers.
///
/// # Examples
///
/// ```compile_fail
/// use std::sync::{Arc, Mutex};
/// use cfilevisit::countingvisitor::CountingVisitor;
///
/// let visitor_callback = Arc::new(Mutex::new(CountingVisitor { file_count: 0 }));
/// cfilevisit::visit_paths(paths, worker_count, visitor_callback.clone());
/// ```
#[derive(Debug)]
pub struct CountingVisitor {
    /// A count of the files encountered during filesystem walk.
    pub file_count: u64,
    pub root_devnos: HashSet<u64>,
}

impl VisitCallback for CountingVisitor {
    fn on_file_visit(&mut self, p: PathBuf, m: &fs::Metadata) -> () {
        self.file_count += 1;
        trace!(
            "FILE({:?})({},{}):{}",
            thread::current().id(),
            m.dev(),
            m.ino(),
            p.into_os_string().into_string().unwrap()
        );
    }
    fn on_dir_enter(&mut self, p: PathBuf, m: &fs::Metadata) -> bool {
        trace!(
            "ENTERING_DIR({:?})({},{}):{}",
            thread::current().id(),
            m.dev(),
            m.ino(),
            p.into_os_string().into_string().unwrap()
        );

        if self.root_devnos.contains(&m.dev()) {
            return true;
        }

        return false;
    }
}

impl CountingVisitor {
    /// Construct a new CountingVisitor
    pub fn new(paths: &Vec<String>) -> CountingVisitor {
        let mut devnos = HashSet::new();

        for p in paths {
            if let Ok(workitem_metadata) = fs::symlink_metadata(p) {
                devnos.insert(workitem_metadata.dev());
            }
        }

        CountingVisitor {
            file_count: 0,
            root_devnos: devnos,
        }
    }

    /// Return the number of files found during walk
    pub fn get_filecount(&mut self) -> u64 {
        self.file_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_when_called_file_count_equals_zero() {
        let mut sut = CountingVisitor::new(&Vec::new());

        let result = sut.get_filecount();

        assert_eq!(0, result);
    }
}
