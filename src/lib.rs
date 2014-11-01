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
//! let repo = match Repository::init(&path) {
//!     Ok(repo) => repo,
//!     Err(e) => panic!("failed to init `{}`: {}", path.display(), e),
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
//!     Err(e) => panic!("failed to open `{}`: {}", path.display(), e),
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
//!     Err(e) => panic!("failed to clone `{}`: {}", path.display(), e),
//! };
//! ```
//!
//! ## Working with a `Repository`
//!
//! All deriviative objects, references, etc are attached to the lifetime of the
//! source `Repository`, to ensure that they do not outlive the repository
//! itself.

#![feature(macro_rules, unsafe_destructor)]
#![deny(missing_docs)]

extern crate libc;
extern crate time;
extern crate url;
extern crate "libgit2-sys" as raw;

use std::c_str::CString;
use std::fmt;
use std::mem;
use std::rt;
use std::str;
use std::sync::{Once, ONCE_INIT};

pub use blob::Blob;
pub use branch::{Branch, Branches};
pub use buf::Buf;
pub use commit::{Commit, Parents};
pub use config::{Config, ConfigEntry, ConfigEntries};
pub use cred::{Cred, CredentialHelper};
pub use error::Error;
pub use index::{Index, IndexEntry, IndexEntries, IndexMatchedPath};
pub use object::Object;
pub use oid::Oid;
pub use push::{Push, Status};
pub use reference::{Reference, References, ReferenceNames};
pub use refspec::Refspec;
pub use remote::{Remote, Refspecs};
pub use remote_callbacks::{RemoteCallbacks, Credentials, TransferProgress};
pub use remote_callbacks::{TransportMessage, Progress};
pub use repo::{Repository, RepositoryInitOptions};
pub use revspec::Revspec;
pub use signature::Signature;
pub use string_array::{StringArray, StringArrayItems, StringArrayBytes};
pub use submodule::Submodule;
pub use tag::Tag;
pub use tree::{Tree, TreeEntry};

/// An enumeration of possible errors that can happen when working with a git
/// repository.
#[deriving(PartialEq, Eq, Clone, Show)]
pub enum ErrorCode {
    /// Generic error
    GenericError,
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
#[allow(missing_docs)]
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
    DirFetch,
    /// Data will be pushed (written) to this remote.
    DirPush,
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
#[deriving(PartialEq, Eq)]
pub enum ObjectType {
    /// An object which corresponds to a any git object
    ObjectAny,
    /// An object which corresponds to a git commit
    ObjectCommit,
    /// An object which corresponds to a git tree
    ObjectTree,
    /// An object which corresponds to a git blob
    ObjectBlob,
    /// An object which corresponds to a git tag
    ObjectTag,
}

/// An enumeration for the possible types of branches
#[deriving(PartialEq, Eq, Show)]
pub enum BranchType {
    /// A local branch not on a remote.
    BranchLocal,
    /// A branch for a remote.
    BranchRemote,
}

/// An enumeration of the possible priority levels of a config file.
///
/// The levels corresponding to the escalation logic (higher to lower) when
/// searching for config entries.
#[deriving(PartialEq, Eq, Show)]
pub enum ConfigLevel {
    /// System-wide configuration file, e.g. /etc/gitconfig
    ConfigSystem,
    /// XDG-compatible configuration file, e.g. ~/.config/git/config
    ConfigXDG,
    /// User-specific configuration, e.g. ~/.gitconfig
    ConfigGlobal,
    /// Reopsitory specific config, e.g. $PWD/.git/config
    ConfigLocal,
    /// Application specific configuration file
    ConfigApp,
    /// Highest level available
    ConfigHighest,
}

bitflags! {
    #[doc = "
Types of credentials that can be requested by a credential callback.
"]
    flags CredentialType: uint {
        const USER_PASS_PLAINTEXT = raw::GIT_CREDTYPE_USERPASS_PLAINTEXT as uint,
        const SSH_KEY = raw::GIT_CREDTYPE_SSH_KEY as uint,
        const SSH_CUSTOM = raw::GIT_CREDTYPE_SSH_CUSTOM as uint,
        const DEFAULT = raw::GIT_CREDTYPE_DEFAULT as uint,
        const SSH_INTERACTIVE = raw::GIT_CREDTYPE_SSH_INTERACTIVE as uint,
    }
}

bitflags! {
    #[doc = "
Flags for APIs that add files matching pathspec
"]
    flags IndexAddOption: u32 {
        const ADD_DEFAULT = raw::GIT_INDEX_ADD_DEFAULT as u32,
        const ADD_FORCE = raw::GIT_INDEX_ADD_FORCE as u32,
        const ADD_DISABLE_PATHSPEC_MATCH =
                raw::GIT_INDEX_ADD_DISABLE_PATHSPEC_MATCH as u32,
        const ADD_CHECK_PATHSPEC = raw::GIT_INDEX_ADD_CHECK_PATHSPEC as u32,
    }
}

mod call;

pub mod build;

mod blob;
mod branch;
mod buf;
mod commit;
mod config;
mod cred;
mod error;
mod index;
mod object;
mod oid;
mod push;
mod reference;
mod refspec;
mod remote;
mod remote_callbacks;
mod repo;
mod revspec;
mod signature;
mod string_array;
mod submodule;
mod tag;
mod tree;

#[cfg(test)] mod test;

fn init() {
    static mut INIT: Once = ONCE_INIT;
    unsafe {
        INIT.doit(|| {
            raw::openssl_init();
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

impl ObjectType {
    /// Convert an object type to its string representation.
    pub fn str(&self) -> &'static str {
        unsafe {
            let ptr = call!(raw::git_object_type2string(*self));
            str::raw::c_str_to_static_slice(ptr)
        }
    }

    /// Determine if the given git_otype is a valid loose object type.
    pub fn is_loose(&self) -> bool {
        unsafe { (call!(raw::git_object_typeisloose(*self)) == 1) }
    }

    /// Convert a raw git_otype to an ObjectType
    pub fn from_raw(raw: raw::git_otype) -> Option<ObjectType> {
        match raw {
            raw::GIT_OBJ_ANY => Some(ObjectAny),
            raw::GIT_OBJ_BAD => None,
            raw::GIT_OBJ__EXT1 => None,
            raw::GIT_OBJ_COMMIT => Some(ObjectCommit),
            raw::GIT_OBJ_TREE => Some(ObjectTree),
            raw::GIT_OBJ_BLOB => Some(ObjectBlob),
            raw::GIT_OBJ_TAG => Some(ObjectTag),
            raw::GIT_OBJ__EXT2 => None,
            raw::GIT_OBJ_OFS_DELTA => None,
            raw::GIT_OBJ_REF_DELTA => None,
        }
    }

    /// Convert this kind into its raw representation
    pub fn raw(&self) -> raw::git_otype {
        call::convert(self)
    }

    /// Convert a string object type representation to its object type.
    pub fn from_str(s: &str) -> Option<ObjectType> {
        let raw = unsafe { call!(raw::git_object_string2type(s.to_c_str())) };
        ObjectType::from_raw(raw)
    }
}

impl fmt::Show for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.str().fmt(f)
    }
}

impl ConfigLevel {
    /// Converts a raw configuration level to a ConfigLevel
    pub fn from_raw(raw: raw::git_config_level_t) -> ConfigLevel {
        match raw {
            raw::GIT_CONFIG_LEVEL_SYSTEM => ::ConfigSystem,
            raw::GIT_CONFIG_LEVEL_XDG => ::ConfigXDG,
            raw::GIT_CONFIG_LEVEL_GLOBAL => ::ConfigGlobal,
            raw::GIT_CONFIG_LEVEL_LOCAL => ::ConfigLocal,
            raw::GIT_CONFIG_LEVEL_APP => ::ConfigApp,
            raw::GIT_CONFIG_HIGHEST_LEVEL => ::ConfigHighest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ObjectType;

    #[test]
    fn convert() {
        assert_eq!(::ObjectBlob.str(), "blob");
        assert_eq!(ObjectType::from_str("blob"), Some(::ObjectBlob));
        assert!(::ObjectBlob.is_loose());
    }

}
