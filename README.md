# Concurrent File Visitor

## Summary

A fast/concurrent file visitor example library written in rust.

This library, by itself, doesn't do anything interesting.  The user (developer) is expected to supply his or her own VisitCallback implementation.  It is expected to perform faster on multicore machines with SSDs.

Nightly rust is required, since the build was altered to use system malloc (this can be changed back).

## Example Usage

* To generate doc: `cargo rustdoc`
* To reeformat code: `cargo fmt`
* To run unit tests (and doc tests): `cargo test`
* To build (debug): `cargo build --examples`
* To build (release): `cargo build --examples --release`
* To run (debug version, trace loglevel): `RUST_LOG=trace RUST_BACKTRACE=1 ./target/debug/examples/do_walk /home /etc`
* To run (release version, 16 threads, debug loglevel): `RUST_LOG=debug RUST_BACKTRACE=1 ./target/release/examples/do_walk -w 16 /`
* To flush filesystem cache on Linux: `sudo bash -c 'free && sync && echo 3 > /proc/sys/vm/drop_caches && free'`

## Changelog

* Added sample unit test
* Added basic rustdoc
* Changed panic behavior from unwind to abort
* Tested on Linux, FreeBSD, MacOS
* Moved countingvisitor impl from example into library
* Added `getopt` command line processing
* Use system malloc instead of jemalloc for smaller binaries
* Converted from executable to library

## Todo

* Test on Windows (may require work or alt metadata impl)
* Benchmark results on different hardware/OS platforms
* Assure there are debug/trace events for unhappy paths (e.g. permissions, file doesn't exist, etc..)
* Add option to follow symlinks, track dev+inode to prevent circular loop
* Add program arg to allow (or prevent) cross-device filesystem walk
* Change from vector to kv store, persist (and optionally continue/resume) filesystem walk
* Add option to tolerate panics that happen in callback functions
  * Will not catch all panics, see: <https://doc.rust-lang.org/std/panic/fn.catch_unwind.html>
