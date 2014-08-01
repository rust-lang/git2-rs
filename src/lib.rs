#![feature(macro_rules)]

extern crate libc;
extern crate raw = "libgit2";

use std::rt;
use std::sync::{Once, ONCE_INIT};

pub use oid::Oid;
pub use error::Error;
pub use repo::Repository;
pub use object::Object;
pub use revspec::Revspec;

mod oid;
mod error;
mod repo;
mod object;
mod revspec;

fn doit(f: || -> libc::c_int) -> Result<(), Error> {
    if f() == 0 {
        Ok(())
    } else {
        Err(Error::last_error().unwrap())
    }
}

fn init() {
    static mut INIT: Once = ONCE_INIT;
    unsafe {
        INIT.doit(|| {
            assert!(raw::git_threads_init() == 0,
                    "couldn't initialize the libgit2 library!");
            rt::at_exit(proc() {
                raw::git_threads_shutdown();
            });
        })
    }
}
