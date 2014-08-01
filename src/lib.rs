#![feature(macro_rules, unsafe_destructor)]

extern crate libc;
extern crate raw = "libgit2";

use std::rt;
use std::sync::{Once, ONCE_INIT};

pub use oid::Oid;
pub use error::Error;
pub use repo::Repository;
pub use object::Object;
pub use revspec::Revspec;

/// An enumeration of possible errors that can happen when working with a git
/// repository.
#[deriving(PartialEq, Eq, Clone, Show)]
pub enum ErrorCode {
    /// Generic error
    Error,
    /// Requested object could not be found
    NotFound,
    /// Object exists preventing operation
    Exists,
    /// More than one object matches
    Ambiguous,
    /// Output buffer too short to hold data
    BufSize,
    /// Operation not allowed on bare repository
    User,
    /// Operation not allowed on bare repository
    BareRepo,
    /// HEAD refers to branch with no commits
    UnbornBranch,
    /// Merge in progress prevented operation
    Unmerged,
    /// Reference was not fast-forwardable
    NotFastForward,
    /// Name/ref spec was not in a valid format
    InvalidSpec,
    /// Merge conflicts prevented operation
    MergeConflict,
    /// Lock file prevented operation
    Locked,
    /// Reference value does not match expected
    Modified,
}

#[deriving(PartialEq, Eq, Clone, Show)]
pub enum RepositoryState {
    Clean,
    Merge,
    Revert,
    CherryPick,
    Bisect,
    Rebase,
    RebaseInteractive,
    RebaseMerge,
    ApplyMailbox,
    ApplyMailboxOrRebase,
}

mod oid;
mod error;
mod repo;
mod object;
mod revspec;

fn doit(f: || -> libc::c_int) -> Result<libc::c_int, Error> {
    match f() {
        n if n < 0 => Err(Error::last_error().unwrap()),
        n => Ok(n),
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
