#![feature(macro_rules, unsafe_destructor)]

extern crate libc;
extern crate raw = "libgit2";

use std::rt;
use std::mem;
use std::sync::{Once, ONCE_INIT};
use std::c_str::CString;

pub use error::Error;
pub use object::Object;
pub use oid::Oid;
pub use refspec::Refspec;
pub use remote::{Remote, Refspecs};
pub use repo::Repository;
pub use revspec::Revspec;
pub use string_array::{StringArray, StringArrayItems, StringArrayBytes};
pub use signature::Signature;

#[cfg(test)]
macro_rules! git( ( $cwd:expr, $($arg:expr),*) => ({
    use std::str;
    let mut cmd = ::std::io::Command::new("git");
    cmd.cwd($cwd)$(.arg($arg))*;
    let out = cmd.output().unwrap();
    if !out.status.success() {
        let err = str::from_utf8(out.error.as_slice()).unwrap_or("<not-utf8>");
        let out = str::from_utf8(out.output.as_slice()).unwrap_or("<not-utf8>");
        fail!("cmd failed: {}\n{}\n{}\n", cmd, out, err);
    }
    str::from_utf8(out.output.as_slice()).unwrap().trim().to_string()
}) )

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

pub enum Direction {
    Fetch, Push,
}

mod error;
mod object;
mod oid;
mod refspec;
mod remote;
mod repo;
mod revspec;
mod signature;
mod string_array;

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

unsafe fn opt_bytes<'a, T>(_: &'a T,
                           c: *const libc::c_char) -> Option<&'a [u8]> {
    if c.is_null() {
        None
    } else {
        let s = CString::new(c, false);
        Some(mem::transmute(s.as_bytes_no_nul()))
    }
}
