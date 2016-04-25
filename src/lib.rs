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
//! # #![allow(unstable)]
//! use git2::Repository;
//!
//! let repo = match Repository::init("/path/to/a/repo") {
//!     Ok(repo) => repo,
//!     Err(e) => panic!("failed to init: {}", e),
//! };
//! ```
//!
//! ### Opening an existing repository
//!
//! ```no_run
//! # #![allow(unstable)]
//! use git2::Repository;
//!
//! let repo = match Repository::open("/path/to/a/repo") {
//!     Ok(repo) => repo,
//!     Err(e) => panic!("failed to open: {}", e),
//! };
//! ```
//!
//! ### Cloning an existing repository
//!
//! ```no_run
//! # #![allow(unstable)]
//! use git2::Repository;
//!
//! let url = "https://github.com/alexcrichton/git2-rs";
//! let repo = match Repository::clone(url, "/path/to/a/repo") {
//!     Ok(repo) => repo,
//!     Err(e) => panic!("failed to clone: {}", e),
//! };
//! ```
//!
//! ## Working with a `Repository`
//!
//! All deriviative objects, references, etc are attached to the lifetime of the
//! source `Repository`, to ensure that they do not outlive the repository
//! itself.

#![doc(html_root_url = "http://alexcrichton.com/git2-rs")]
#![allow(trivial_numeric_casts, trivial_casts)]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]
#![cfg_attr(feature = "unstable", feature(recover, std_panic))]

extern crate libc;
extern crate url;
extern crate libgit2_sys as raw;
#[macro_use] extern crate bitflags;
#[cfg(test)] extern crate tempdir;

use std::ffi::{CStr, CString};
use std::fmt;
use std::str;
use std::sync::{Once, ONCE_INIT};

pub use blame::{Blame, BlameHunk, BlameIter, BlameOptions};
pub use blob::Blob;
pub use branch::{Branch, Branches};
pub use buf::Buf;
pub use commit::{Commit, Parents};
pub use config::{Config, ConfigEntry, ConfigEntries};
pub use cred::{Cred, CredentialHelper};
pub use describe::{Describe, DescribeFormatOptions, DescribeOptions};
pub use diff::{Diff, DiffDelta, DiffFile, DiffOptions, Deltas};
pub use diff::{DiffLine, DiffHunk, DiffStats, DiffFindOptions};
pub use diff::{DiffBinary, DiffBinaryFile, DiffBinaryKind};
pub use merge::{AnnotatedCommit, MergeOptions};
pub use error::Error;
pub use index::{Index, IndexEntry, IndexEntries, IndexMatchedPath};
pub use message::{message_prettify, DEFAULT_COMMENT_CHAR};
pub use note::{Note, Notes};
pub use object::Object;
pub use oid::Oid;
pub use pathspec::{Pathspec, PathspecMatchList, PathspecFailedEntries};
pub use pathspec::{PathspecDiffEntries, PathspecEntries};
pub use reference::{Reference, References, ReferenceNames};
pub use reflog::{Reflog, ReflogEntry, ReflogIter};
pub use refspec::Refspec;
pub use remote::{Remote, Refspecs, RemoteHead, FetchOptions, PushOptions};
pub use remote_callbacks::{RemoteCallbacks, Credentials, TransferProgress};
pub use remote_callbacks::{TransportMessage, Progress, UpdateTips};
pub use repo::{Repository, RepositoryInitOptions};
pub use revspec::Revspec;
pub use revwalk::Revwalk;
pub use signature::Signature;
pub use status::{StatusOptions, Statuses, StatusIter, StatusEntry, StatusShow};
pub use submodule::Submodule;
pub use tag::Tag;
pub use time::{Time, IndexTime};
pub use tree::{Tree, TreeEntry, TreeIter};
pub use treebuilder::TreeBuilder;
pub use util::IntoCString;

/// An enumeration of possible errors that can happen when working with a git
/// repository.
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
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
    /// User-generated error
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
    /// Checkout conflicts prevented operation
    Conflict,
    /// Lock file prevented operation
    Locked,
    /// Reference value does not match expected
    Modified,
    /// Authentication error
    Auth,
    /// Server certificate is invalid
    Certificate,
    /// Patch/merge has already been applied
    Applied,
    /// The requested peel operation is not possible
    Peel,
    /// Unexpected EOF
    Eof,
    /// Invalid operation or input
    Invalid,
    /// Uncommitted changes in index prevented operation
    Uncommitted,
    /// Operation was not valid for a directory,
    Directory,
}

/// An enumeration of possible categories of things that can have
/// errors when working with a git repository.
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
pub enum ErrorClass {
    /// Uncategorized
    None,
    /// Out of memory or insufficient allocated space
    NoMemory,
    /// Syscall or standard system library error
    Os,
    /// Invalid input
    Invalid,
    /// Error resolving or manipulating a reference
    Reference,
    /// ZLib failure
    Zlib,
    /// Bad repository state
    Repository,
    /// Bad configuration
    Config,
    /// Regex failure
    Regex,
    /// Bad object
    Odb,
    /// Invalid index data
    Index,
    /// Error creating or obtaining an object
    Object,
    /// Network error
    Net,
    /// Error manpulating a tag
    Tag,
    /// Invalid value in tree
    Tree,
    /// Hashing or packing error
    Indexer,
    /// Error from SSL
    Ssl,
    /// Error involing submodules
    Submodule,
    /// Threading error
    Thread,
    /// Error manipulating a stash
    Stash,
    /// Checkout failure
    Checkout,
    /// Invalid FETCH_HEAD
    FetchHead,
    /// Merge failure
    Merge,
    /// SSH failure
    Ssh,
    /// Error manipulating filters
    Filter,
    /// Error reverting commit
    Revert,
    /// Error from a user callback
    Callback,
    /// Error cherry-picking commit
    CherryPick,
    /// Can't describe object
    Describe,
    /// Error during rebase
    Rebase,
    /// Filesystem-related error
    Filesystem,
}

/// A listing of the possible states that a repository can be in.
#[derive(PartialEq, Eq, Clone, Debug, Copy)]
#[allow(missing_docs)]
pub enum RepositoryState {
    Clean,
    Merge,
    Revert,
    RevertSequence,
    CherryPick,
    CherryPickSequence,
    Bisect,
    Rebase,
    RebaseInteractive,
    RebaseMerge,
    ApplyMailbox,
    ApplyMailboxOrRebase,
}

/// An enumeration of the possible directions for a remote.
#[derive(Copy, Clone)]
pub enum Direction {
    /// Data will be fetched (read) from this remote.
    Fetch,
    /// Data will be pushed (written) to this remote.
    Push,
}

/// An enumeration of the operations that can be performed for the `reset`
/// method on a `Repository`.
#[derive(Copy, Clone)]
pub enum ResetType {
    /// Move the head to the given commit.
    Soft,
    /// Soft plus reset the index to the commit.
    Mixed,
    /// Mixed plus changes in the working tree are discarded.
    Hard,
}

/// An enumeration all possible kinds objects may have.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ObjectType {
    /// Any kind of git object
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

/// An enumeration for the possible types of branches
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum BranchType {
    /// A local branch not on a remote.
    Local,
    /// A branch for a remote.
    Remote,
}

/// An enumeration of the possible priority levels of a config file.
///
/// The levels corresponding to the escalation logic (higher to lower) when
/// searching for config entries.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum ConfigLevel {
    /// System-wide on Windows, for compatibility with portable git
    ProgramData,
    /// System-wide configuration file, e.g. /etc/gitconfig
    System,
    /// XDG-compatible configuration file, e.g. ~/.config/git/config
    XDG,
    /// User-specific configuration, e.g. ~/.gitconfig
    Global,
    /// Repository specific config, e.g. $PWD/.git/config
    Local,
    /// Application specific configuration file
    App,
    /// Highest level available
    Highest,
}

/// Merge file favor options for `MergeOptions` instruct the file-level
/// merging functionality how to deal with conflicting regions of the files.
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum FileFavor {
    /// When a region of a file is changed in both branches, a conflict will be
    /// recorded in the index so that git_checkout can produce a merge file with
    /// conflict markers in the working directory. This is the default.
    Normal,
    /// When a region of a file is changed in both branches, the file created
    /// in the index will contain the "ours" side of any conflicting region.
    /// The index will not record a conflict.
    Ours,
    /// When a region of a file is changed in both branches, the file created
    /// in the index will contain the "theirs" side of any conflicting region.
    /// The index will not record a conflict.
    Theirs,
    /// When a region of a file is changed in both branches, the file created
    /// in the index will contain each unique line from each side, which has
    /// the result of combining both files. The index will not record a conflict.
    Union,
}

bitflags! {
    #[doc = "
Orderings that may be specified for Revwalk iteration.
"]
    flags Sort: u32 {
        /// Sort the repository contents in no particular ordering.
        ///
        /// This sorting is arbitrary, implementation-specific, and subject to
        /// change at any time. This is the default sorting for new walkers.
        const SORT_NONE = raw::GIT_SORT_NONE as u32,

        /// Sort the repository contents in topological order (parents before
        /// children).
        ///
        /// This sorting mode can be combined with time sorting.
        const SORT_TOPOLOGICAL = raw::GIT_SORT_TOPOLOGICAL as u32,

        /// Sort the repository contents by commit time.
        ///
        /// This sorting mode can be combined with topological sorting.
        const SORT_TIME = raw::GIT_SORT_TIME as u32,

        /// Iterate through the repository contents in reverse order.
        ///
        /// This sorting mode can be combined with any others.
        const SORT_REVERSE = raw::GIT_SORT_REVERSE as u32,
    }
}

bitflags! {
    #[doc = "
Types of credentials that can be requested by a credential callback.
"]
    flags CredentialType: u32 {
        #[allow(missing_docs)]
        const USER_PASS_PLAINTEXT = raw::GIT_CREDTYPE_USERPASS_PLAINTEXT as u32,
        #[allow(missing_docs)]
        const SSH_KEY = raw::GIT_CREDTYPE_SSH_KEY as u32,
        #[allow(missing_docs)]
        const SSH_MEMORY = raw::GIT_CREDTYPE_SSH_MEMORY as u32,
        #[allow(missing_docs)]
        const SSH_CUSTOM = raw::GIT_CREDTYPE_SSH_CUSTOM as u32,
        #[allow(missing_docs)]
        const DEFAULT = raw::GIT_CREDTYPE_DEFAULT as u32,
        #[allow(missing_docs)]
        const SSH_INTERACTIVE = raw::GIT_CREDTYPE_SSH_INTERACTIVE as u32,
        #[allow(missing_docs)]
        const USERNAME = raw::GIT_CREDTYPE_USERNAME as u32,
    }
}

bitflags! {
    #[doc = "
Flags for APIs that add files matching pathspec
"]
    flags IndexAddOption: u32 {
        #[allow(missing_docs)]
        const ADD_DEFAULT = raw::GIT_INDEX_ADD_DEFAULT as u32,
        #[allow(missing_docs)]
        const ADD_FORCE = raw::GIT_INDEX_ADD_FORCE as u32,
        #[allow(missing_docs)]
        const ADD_DISABLE_PATHSPEC_MATCH =
                raw::GIT_INDEX_ADD_DISABLE_PATHSPEC_MATCH as u32,
        #[allow(missing_docs)]
        const ADD_CHECK_PATHSPEC = raw::GIT_INDEX_ADD_CHECK_PATHSPEC as u32,
    }
}

bitflags! {
    #[doc = "
Flags for `Repository::open_ext`
"]
    flags RepositoryOpenFlags: u32 {
        /// Only open the specified path; don't walk upward searching.
        const REPOSITORY_OPEN_NO_SEARCH = raw::GIT_REPOSITORY_OPEN_NO_SEARCH as u32,
        /// Search across filesystem boundaries.
        const REPOSITORY_OPEN_CROSS_FS = raw::GIT_REPOSITORY_OPEN_CROSS_FS as u32,
        /// Force opening as bare repository, and defer loading its config.
        const REPOSITORY_OPEN_BARE = raw::GIT_REPOSITORY_OPEN_BARE as u32,
    }
}

bitflags! {
    #[doc = "
Flags for the return value of `Repository::revparse`
"]
    flags RevparseMode: u32 {
        /// The spec targeted a single object
        const REVPARSE_SINGLE = raw::GIT_REVPARSE_SINGLE as u32,
        /// The spec targeted a range of commits
        const REVPARSE_RANGE = raw::GIT_REVPARSE_RANGE as u32,
        /// The spec used the `...` operator, which invokes special semantics.
        const REVPARSE_MERGE_BASE = raw::GIT_REVPARSE_MERGE_BASE as u32,
    }
}

#[cfg(test)] #[macro_use] mod test;
#[macro_use] mod panic;
mod call;
mod util;

pub mod build;
pub mod cert;
pub mod string_array;
pub mod oid_array;
pub mod transport;

mod blame;
mod blob;
mod branch;
mod buf;
mod commit;
mod config;
mod cred;
mod describe;
mod diff;
mod merge;
mod error;
mod index;
mod message;
mod note;
mod object;
mod oid;
mod pathspec;
mod reference;
mod reflog;
mod refspec;
mod remote;
mod remote_callbacks;
mod repo;
mod revspec;
mod revwalk;
mod signature;
mod status;
mod submodule;
mod tag;
mod time;
mod tree;
mod treebuilder;

fn init() {
    static INIT: Once = ONCE_INIT;
    INIT.call_once(|| unsafe {
        raw::openssl_init();
        let r = raw::git_libgit2_init();
        assert!(r >= 0,
                "couldn't initialize the libgit2 library: {}", r);
        assert_eq!(libc::atexit(shutdown), 0);
    });
    extern fn shutdown() {
        unsafe { raw::git_libgit2_shutdown(); }
    }
}

unsafe fn opt_bytes<'a, T>(_anchor: &'a T,
                           c: *const libc::c_char) -> Option<&'a [u8]> {
    if c.is_null() {
        None
    } else {
        Some(CStr::from_ptr(c).to_bytes())
    }
}

fn opt_cstr<T: IntoCString>(o: Option<T>) -> Result<Option<CString>, Error> {
    match o {
        Some(s) => s.into_c_string().map(Some),
        None => Ok(None)
    }
}

impl ObjectType {
    /// Convert an object type to its string representation.
    pub fn str(&self) -> &'static str {
        unsafe {
            let ptr = call!(raw::git_object_type2string(*self)) as *const _;
            let data = CStr::from_ptr(ptr).to_bytes();
            str::from_utf8(data).unwrap()
        }
    }

    /// Determine if the given git_otype is a valid loose object type.
    pub fn is_loose(&self) -> bool {
        unsafe { (call!(raw::git_object_typeisloose(*self)) == 1) }
    }

    /// Convert a raw git_otype to an ObjectType
    pub fn from_raw(raw: raw::git_otype) -> Option<ObjectType> {
        match raw {
            raw::GIT_OBJ_ANY => Some(ObjectType::Any),
            raw::GIT_OBJ_COMMIT => Some(ObjectType::Commit),
            raw::GIT_OBJ_TREE => Some(ObjectType::Tree),
            raw::GIT_OBJ_BLOB => Some(ObjectType::Blob),
            raw::GIT_OBJ_TAG => Some(ObjectType::Tag),
            _ => None,
        }
    }

    /// Convert this kind into its raw representation
    pub fn raw(&self) -> raw::git_otype {
        call::convert(self)
    }

    /// Convert a string object type representation to its object type.
    pub fn from_str(s: &str) -> Option<ObjectType> {
        let raw = unsafe { call!(raw::git_object_string2type(CString::new(s).unwrap())) };
        ObjectType::from_raw(raw)
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.str().fmt(f)
    }
}

impl ConfigLevel {
    /// Converts a raw configuration level to a ConfigLevel
    pub fn from_raw(raw: raw::git_config_level_t) -> ConfigLevel {
        match raw {
            raw::GIT_CONFIG_LEVEL_PROGRAMDATA => ConfigLevel::ProgramData,
            raw::GIT_CONFIG_LEVEL_SYSTEM => ConfigLevel::System,
            raw::GIT_CONFIG_LEVEL_XDG => ConfigLevel::XDG,
            raw::GIT_CONFIG_LEVEL_GLOBAL => ConfigLevel::Global,
            raw::GIT_CONFIG_LEVEL_LOCAL => ConfigLevel::Local,
            raw::GIT_CONFIG_LEVEL_APP => ConfigLevel::App,
            raw::GIT_CONFIG_HIGHEST_LEVEL => ConfigLevel::Highest,
            n => panic!("unknown config level: {}", n),
        }
    }
}

bitflags! {
    /// Status flags for a single file
    ///
    /// A combination of these values will be returned to indicate the status of
    /// a file.  Status compares the working directory, the index, and the
    /// current HEAD of the repository.  The `STATUS_INDEX_*` set of flags
    /// represents the status of file in the index relative to the HEAD, and the
    /// `STATUS_WT_*` set of flags represent the status of the file in the
    /// working directory relative to the index.
    flags Status: u32 {
        #[allow(missing_docs)]
        const STATUS_CURRENT = raw::GIT_STATUS_CURRENT as u32,

        #[allow(missing_docs)]
        const STATUS_INDEX_NEW = raw::GIT_STATUS_INDEX_NEW as u32,
        #[allow(missing_docs)]
        const STATUS_INDEX_MODIFIED = raw::GIT_STATUS_INDEX_MODIFIED as u32,
        #[allow(missing_docs)]
        const STATUS_INDEX_DELETED = raw::GIT_STATUS_INDEX_DELETED as u32,
        #[allow(missing_docs)]
        const STATUS_INDEX_RENAMED = raw::GIT_STATUS_INDEX_RENAMED as u32,
        #[allow(missing_docs)]
        const STATUS_INDEX_TYPECHANGE = raw::GIT_STATUS_INDEX_TYPECHANGE as u32,

        #[allow(missing_docs)]
        const STATUS_WT_NEW = raw::GIT_STATUS_WT_NEW as u32,
        #[allow(missing_docs)]
        const STATUS_WT_MODIFIED = raw::GIT_STATUS_WT_MODIFIED as u32,
        #[allow(missing_docs)]
        const STATUS_WT_DELETED = raw::GIT_STATUS_WT_DELETED as u32,
        #[allow(missing_docs)]
        const STATUS_WT_TYPECHANGE = raw::GIT_STATUS_WT_TYPECHANGE as u32,
        #[allow(missing_docs)]
        const STATUS_WT_RENAMED = raw::GIT_STATUS_WT_RENAMED as u32,

        #[allow(missing_docs)]
        const STATUS_IGNORED = raw::GIT_STATUS_IGNORED as u32,
        #[allow(missing_docs)]
        const STATUS_CONFLICTED = raw::GIT_STATUS_CONFLICTED as u32,
    }
}

bitflags! {
    #[doc = "
Mode options for RepositoryInitOptions
"]
    flags RepositoryInitMode: u32 {
        /// Use permissions configured by umask - the default
        const REPOSITORY_INIT_SHARED_UMASK =
                    raw::GIT_REPOSITORY_INIT_SHARED_UMASK as u32,
        /// Use `--shared=group` behavior, chmod'ing the new repo to be
        /// group writable and \"g+sx\" for sticky group assignment
        const REPOSITORY_INIT_SHARED_GROUP =
                    raw::GIT_REPOSITORY_INIT_SHARED_GROUP as u32,
        /// Use `--shared=all` behavior, adding world readability.
        const REPOSITORY_INIT_SHARED_ALL =
                    raw::GIT_REPOSITORY_INIT_SHARED_ALL as u32,
    }
}

/// What type of change is described by a `DiffDelta`?
#[derive(Copy, Clone, Debug)]
pub enum Delta {
    /// No changes
    Unmodified,
    /// Entry does not exist in old version
    Added,
    /// Entry does not exist in new version
    Deleted,
    /// Entry content changed between old and new
    Modified,
    /// Entry was renamed wbetween old and new
    Renamed,
    /// Entry was copied from another old entry
    Copied,
    /// Entry is ignored item in workdir
    Ignored,
    /// Entry is untracked item in workdir
    Untracked,
    /// Type of entry changed between old and new
    Typechange,
    /// Entry is unreadable
    Unreadable,
    /// Entry in the index is conflicted
    Conflicted,
}

bitflags! {
    #[doc = r#"
Return codes for submodule status.

A combination of these flags will be returned to describe the status of a
submodule.  Depending on the "ignore" property of the submodule, some of
the flags may never be returned because they indicate changes that are
supposed to be ignored.

Submodule info is contained in 4 places: the HEAD tree, the index, config
files (both .git/config and .gitmodules), and the working directory.  Any
or all of those places might be missing information about the submodule
depending on what state the repo is in.  We consider all four places to
build the combination of status flags.

There are four values that are not really status, but give basic info
about what sources of submodule data are available.  These will be
returned even if ignore is set to "ALL".

* IN_HEAD   - superproject head contains submodule
* IN_INDEX  - superproject index contains submodule
* IN_CONFIG - superproject gitmodules has submodule
* IN_WD     - superproject workdir has submodule

The following values will be returned so long as ignore is not "ALL".

* INDEX_ADDED       - in index, not in head
* INDEX_DELETED     - in head, not in index
* INDEX_MODIFIED    - index and head don't match
* WD_UNINITIALIZED  - workdir contains empty directory
* WD_ADDED          - in workdir, not index
* WD_DELETED        - in index, not workdir
* WD_MODIFIED       - index and workdir head don't match

The following can only be returned if ignore is "NONE" or "UNTRACKED".

* WD_INDEX_MODIFIED - submodule workdir index is dirty
* WD_WD_MODIFIED    - submodule workdir has modified files

Lastly, the following will only be returned for ignore "NONE".

* WD_UNTRACKED      - wd contains untracked files
"#]
    flags SubmoduleStatus: u32 {
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_IN_HEAD =
                raw::GIT_SUBMODULE_STATUS_IN_HEAD as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_IN_INDEX =
                raw::GIT_SUBMODULE_STATUS_IN_INDEX as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_IN_CONFIG =
                raw::GIT_SUBMODULE_STATUS_IN_CONFIG as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_IN_WD =
                raw::GIT_SUBMODULE_STATUS_IN_WD as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_INDEX_ADDED =
                raw::GIT_SUBMODULE_STATUS_INDEX_ADDED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_INDEX_DELETED =
                raw::GIT_SUBMODULE_STATUS_INDEX_DELETED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_INDEX_MODIFIED =
                raw::GIT_SUBMODULE_STATUS_INDEX_MODIFIED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_UNINITIALIZED =
                raw::GIT_SUBMODULE_STATUS_WD_UNINITIALIZED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_ADDED =
                raw::GIT_SUBMODULE_STATUS_WD_ADDED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_DELETED =
                raw::GIT_SUBMODULE_STATUS_WD_DELETED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_MODIFIED =
                raw::GIT_SUBMODULE_STATUS_WD_MODIFIED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_INDEX_MODIFIED =
                raw::GIT_SUBMODULE_STATUS_WD_INDEX_MODIFIED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_WD_MODIFIED =
                raw::GIT_SUBMODULE_STATUS_WD_WD_MODIFIED as u32,
        #[allow(missing_docs)]
        const SUBMODULE_STATUS_WD_UNTRACKED =
                raw::GIT_SUBMODULE_STATUS_WD_UNTRACKED as u32,
    }

}

/// Submodule ignore values
///
/// These values represent settings for the `submodule.$name.ignore`
/// configuration value which says how deeply to look at the working
/// directory when getting the submodule status.
pub enum SubmoduleIgnore {
    /// Use the submodule's configuration
    Unspecified,
    /// Any change or untracked file is considered dirty
    None,
    /// Only dirty if tracked files have changed
    Untracked,
    /// Only dirty if HEAD has moved
    Dirty,
    /// Never dirty
    All,
}

bitflags! {
    /// ...
    flags PathspecFlags: u32 {
        /// Use the default pathspec matching configuration.
        const PATHSPEC_DEFAULT = raw::GIT_PATHSPEC_DEFAULT as u32,
        /// Force matching to ignore case, otherwise matching will use native
        /// case sensitivity fo the platform filesystem.
        const PATHSPEC_IGNORE_CASE = raw::GIT_PATHSPEC_IGNORE_CASE as u32,
        /// Force case sensitive matches, otherwise match will use the native
        /// case sensitivity of the platform filesystem.
        const PATHSPEC_USE_CASE = raw::GIT_PATHSPEC_USE_CASE as u32,
        /// Disable glob patterns and just use simple string comparison for
        /// matching.
        const PATHSPEC_NO_GLOB = raw::GIT_PATHSPEC_NO_GLOB as u32,
        /// Means that match functions return the error code `NotFound` if no
        /// matches are found. By default no matches is a success.
        const PATHSPEC_NO_MATCH_ERROR = raw::GIT_PATHSPEC_NO_MATCH_ERROR as u32,
        /// Means that the list returned should track which patterns matched
        /// which files so that at the end of the match we can identify patterns
        /// that did not match any files.
        const PATHSPEC_FIND_FAILURES = raw::GIT_PATHSPEC_FIND_FAILURES as u32,
        /// Means that the list returned does not need to keep the actual
        /// matching filenames. Use this to just test if there were any matches
        /// at all or in combination with `PATHSPEC_FAILURES` to validate a
        /// pathspec.
        const PATHSPEC_FAILURES_ONLY = raw::GIT_PATHSPEC_FAILURES_ONLY as u32,
    }
}

/// Possible output formats for diff data
#[derive(Copy, Clone)]
pub enum DiffFormat {
    /// full git diff
    Patch,
    /// just the headers of the patch
    PatchHeader,
    /// like git diff --raw
    Raw,
    /// like git diff --name-only
    NameOnly,
    /// like git diff --name-status
    NameStatus,
}

bitflags! {
    /// Formatting options for diff stats
    flags DiffStatsFormat: raw::git_diff_stats_format_t {
        /// Don't generate any stats
        const DIFF_STATS_NONE = raw::GIT_DIFF_STATS_NONE,
        /// Equivalent of `--stat` in git
        const DIFF_STATS_FULL = raw::GIT_DIFF_STATS_FULL,
        /// Equivalent of `--shortstat` in git
        const DIFF_STATS_SHORT = raw::GIT_DIFF_STATS_SHORT,
        /// Equivalent of `--numstat` in git
        const DIFF_STATS_NUMBER = raw::GIT_DIFF_STATS_NUMBER,
        /// Extended header information such as creations, renames and mode
        /// changes, equivalent of `--summary` in git
        const DIFF_STATS_INCLUDE_SUMMARY =
            raw::GIT_DIFF_STATS_INCLUDE_SUMMARY,
    }
}

/// Automatic tag following options.
pub enum AutotagOption {
    /// Use the setting from the remote's configuration
    Unspecified,
    /// Ask the server for tags pointing to objects we're already downloading
    Auto,
    /// Don't ask for any tags beyond the refspecs
    None,
    /// Ask for all the tags
    All,
}

/// Configuration for how pruning is done on a fetch
pub enum FetchPrune {
    /// Use the setting from the configuration
    Unspecified,
    /// Force pruning on
    On,
    /// Force pruning off
    Off,
}

#[cfg(test)]
mod tests {
    use super::ObjectType;

    #[test]
    fn convert() {
        assert_eq!(ObjectType::Blob.str(), "blob");
        assert_eq!(ObjectType::from_str("blob"), Some(ObjectType::Blob));
        assert!(ObjectType::Blob.is_loose());
    }

}
