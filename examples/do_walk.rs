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
// examples/do_walk.rs

// Next five lines require nightly rust
// Use system malloc instead of bundling jemalloc
#![feature(global_allocator)]
#![feature(allocator_api)]
use std::heap::System;
#[global_allocator]
static ALLOCATOR: System = System;

use cfilevisit::countingvisitor::CountingVisitor;
use getopts::Options;
use std::env;
use std::sync::{Arc, Mutex};
use time::PreciseTime;

#[macro_use]
extern crate log;
extern crate cfilevisit;
extern crate env_logger;
extern crate getopts;
extern crate time;

const DEFAULT_WORKER_COUNT: usize = 16;

fn print_usage(program: &str, opts: Options) {
    let brief = format!(
        "Usage: RUST_LOG=debug {} [options] PATH [PATH2] [PATH3] [...]",
        program
    );
    print!("{}", opts.usage(&brief));
}

fn main() {
    // enable logging
    env_logger::init();

    // process arguments
    let args: Vec<String> = env::args().collect();
    let program = args[0].clone();

    let mut opts = Options::new();
    opts.optopt("w", "workers", "set worker thread count", "THREAD_COUNT");
    opts.optflag("h", "help", "print this help menu");
    let matches = match opts.parse(&args[1..]) {
        Ok(m) => m,
        Err(f) => panic!(f.to_string()),
    };

    // help requested
    if matches.opt_present("h") {
        print_usage(&program, opts);
        return;
    }

    // assign user-supplied worker count
    let mut worker_count = DEFAULT_WORKER_COUNT;
    if matches.opt_present("w") {
        let w = matches.opt_str("w").unwrap();
        worker_count = w.parse().unwrap();
    }

    // set up initial paths to walk (may be directories or files)
    let paths = if !matches.free.is_empty() {
        // args if supplied
        matches.free
    } else {
        // root filesystem, if not
        let mut p = Vec::new();
        p.push(String::from("/"));
        p
    };

    let original_paths = paths.clone();

    // initialize file counting visitor
    let visitor_callback = Arc::new(Mutex::new(CountingVisitor::new(&paths)));

    // walk the path(s), capture time before and after
    let before_walk = PreciseTime::now();
    cfilevisit::visit_paths(paths, worker_count, visitor_callback.clone());
    let after_walk = PreciseTime::now();

    // report results
    info!(
        "It took {} seconds to walk paths={:?}.",
        before_walk.to(after_walk),
        original_paths
    );
    info!(
        "Files found: {}",
        visitor_callback.lock().unwrap().get_filecount()
    );
}
