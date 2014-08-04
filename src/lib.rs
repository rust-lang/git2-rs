//! # libgit2 bindings for Rust
//!
//! This library contains bindings to the [libgit2][1] C library which is used
//! to manage git repositories. The library itself is a work in progress and is
//! likely lacking some bindings here and there, so be warned.
//!
//! [1]: https://libgit2.github.com/
//!
//! The git2-rs library strives to be as close to libgit2 as possible, but also
//! strives to make using libgit2 as safe as possible. All resource management
//! is automatic as well as adding strong types to all interfaces (including
//! `Result`)
//!
//! ## Creating a `Repository`
//!
//! The `Repository` is the source from which almost all other objects in git-rs
//! are spawned. A repository can be created through opening, initializing, or
//! cloning.
//!
//! ### Initializing a new repository
//!
//! The `init` method will create a new repository, assuming one does not
//! already exist.
//!
//! ```no_run
//! use git2::Repository;
//!
//! let path = Path::new("/path/to/a/repo");
//! let repo = match Repository::init(&path, false) { // false for not a bare repo
//!     Ok(repo) => repo,
//!     Err(e) => fail!("failed to init `{}`: {}", path.display(), e),
//! };
//! ```
//!
//! ### Opening an existing repository
//!
//! ```no_run
//! use git2::Repository;
//!
//! let path = Path::new("/path/to/a/repo");
//! let repo = match Repository::open(&path) {
//!     Ok(repo) => repo,
//!     Err(e) => fail!("failed to open `{}`: {}", path.display(), e),
//! };
//! ```
//!
//! ### Cloning an existing repository
//!
//! ```no_run
//! use git2::Repository;
//!
//! let url = "https://github.com/alexcrichton/git2-rs";
//! let path = Path::new("/path/to/a/repo");
//! let repo = match Repository::clone(url, &path) {
//!     Ok(repo) => repo,
//!     Err(e) => fail!("failed to clone `{}`: {}", path.display(), e),
//! };
//! ```
//!
//! ## Working with a `Repository`
//!
//! All deriviative objects, references, etc are attached to the lifetime of the
//! source `Repository`, to ensure that they do not outlive the repository
//! itself.

#![feature(macro_rules, unsafe_destructor)]
#![deny(missing_doc)]

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
pub use reference::{Reference, References, ReferenceNames};
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

/// A listing of the possible states that a repository can be in.
#[deriving(PartialEq, Eq, Clone, Show)]
#[allow(missing_doc)]
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

/// An enumeration of the possible directions for a remote.
pub enum Direction {
    /// Data will be fetched (read) from this remote.
    Fetch,
    /// Data will be pushed (written) to this remote.
    Push,
}

/// An enumeration of the operations that can be performed for the `reset`
/// method on a `Repository`.
pub enum ResetType {
    /// Move the head to the given commit.
    Soft,
    /// Soft plus reset the index to the commit.
    Mixed,
    /// Mixed plus changes in the working tree are discarded.
    Hard,
}

/// An enumeration all possible kinds objects may have.
pub enum ObjectKind {
    /// An object which corresponds to a any git object
    Any,
    /// An object which corresponds to a git commit
    Commit,
    /// An object which corresponds to a git tree
    Tree,
    /// An object which corresponds to a git blob
    Blob,
    /// An object which corresponds to a git tag
    Tag,
}

pub mod build;

mod error;
mod object;
mod oid;
mod reference;
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
