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
//! To clone using SSH, refer to [RepoBuilder](./build/struct.RepoBuilder.html).
//!
//! ## Working with a `Repository`
//!
//! All derivative objects, references, etc are attached to the lifetime of the
//! source `Repository`, to ensure that they do not outlive the repository
//! itself.

#![doc(html_root_url = "https://docs.rs/git2/0.19")]
#![allow(trivial_numeric_casts, trivial_casts)]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![cfg_attr(test, deny(warnings))]

use bitflags::bitflags;
use libgit2_sys as raw;

use std::ffi::{CStr, CString};
use std::fmt;
use std::str;
use std::sync::Once;

pub use crate::apply::{ApplyLocation, ApplyOptions};
pub use crate::attr::AttrValue;
pub use crate::blame::{Blame, BlameHunk, BlameIter, BlameOptions};
pub use crate::blob::{Blob, BlobWriter};
pub use crate::branch::{Branch, Branches};
pub use crate::buf::Buf;
pub use crate::cherrypick::CherrypickOptions;
pub use crate::commit::{Commit, Parents};
pub use crate::config::{Config, ConfigEntries, ConfigEntry};
pub use crate::cred::{Cred, CredentialHelper};
pub use crate::describe::{Describe, DescribeFormatOptions, DescribeOptions};
pub use crate::diff::{Deltas, Diff, DiffDelta, DiffFile, DiffOptions};
pub use crate::diff::{DiffBinary, DiffBinaryFile, DiffBinaryKind, DiffPatchidOptions};
pub use crate::diff::{DiffFindOptions, DiffHunk, DiffLine, DiffLineType, DiffStats};
pub use crate::email::{Email, EmailCreateOptions};
pub use crate::error::Error;
pub use crate::index::{
    Index, IndexConflict, IndexConflicts, IndexEntries, IndexEntry, IndexMatchedPath,
};
pub use crate::indexer::{Indexer, IndexerProgress, Progress};
pub use crate::mailmap::Mailmap;
pub use crate::mempack::Mempack;
pub use crate::merge::{AnnotatedCommit, MergeOptions};
pub use crate::message::{
    message_prettify, message_trailers_bytes, message_trailers_strs, MessageTrailersBytes,
    MessageTrailersBytesIterator, MessageTrailersStrs, MessageTrailersStrsIterator,
    DEFAULT_COMMENT_CHAR,
};
pub use crate::note::{Note, Notes};
pub use crate::object::Object;
pub use crate::odb::{Odb, OdbObject, OdbPackwriter, OdbReader, OdbWriter};
pub use crate::oid::Oid;
pub use crate::packbuilder::{PackBuilder, PackBuilderStage};
pub use crate::patch::Patch;
pub use crate::pathspec::{Pathspec, PathspecFailedEntries, PathspecMatchList};
pub use crate::pathspec::{PathspecDiffEntries, PathspecEntries};
pub use crate::proxy_options::ProxyOptions;
pub use crate::push_update::PushUpdate;
pub use crate::rebase::{Rebase, RebaseOperation, RebaseOperationType, RebaseOptions};
pub use crate::reference::{Reference, ReferenceNames, References};
pub use crate::reflog::{Reflog, ReflogEntry, ReflogIter};
pub use crate::refspec::Refspec;
pub use crate::remote::{
    FetchOptions, PushOptions, Refspecs, Remote, RemoteConnection, RemoteHead, RemoteRedirect,
};
pub use crate::remote_callbacks::{CertificateCheckStatus, Credentials, RemoteCallbacks};
pub use crate::remote_callbacks::{TransportMessage, UpdateTips};
pub use crate::repo::{Repository, RepositoryInitOptions};
pub use crate::revert::RevertOptions;
pub use crate::revspec::Revspec;
pub use crate::revwalk::Revwalk;
pub use crate::signature::Signature;
pub use crate::stash::{StashApplyOptions, StashApplyProgressCb, StashCb, StashSaveOptions};
pub use crate::status::{StatusEntry, StatusIter, StatusOptions, StatusShow, Statuses};
pub use crate::submodule::{Submodule, SubmoduleUpdateOptions};
pub use crate::tag::Tag;
pub use crate::time::{IndexTime, Time};
pub use crate::tracing::{trace_set, TraceLevel};
pub use crate::transaction::Transaction;
pub use crate::tree::{Tree, TreeEntry, TreeIter, TreeWalkMode, TreeWalkResult};
pub use crate::treebuilder::TreeBuilder;
pub use crate::util::IntoCString;
pub use crate::version::Version;
pub use crate::worktree::{Worktree, WorktreeAddOptions, WorktreeLockStatus, WorktreePruneOptions};

// Create a convinience method on bitflag struct which checks the given flag
macro_rules! is_bit_set {
    ($name:ident, $flag:expr) => {
        #[allow(missing_docs)]
        pub fn $name(&self) -> bool {
            self.intersects($flag)
        }
    };
}

/// An enumeration of possible errors that can happen when working with a git
/// repository.
// Note: We omit a few native error codes, as they are unlikely to be propagated
// to the library user. Currently:
//
// * GIT_EPASSTHROUGH
// * GIT_ITEROVER
// * GIT_RETRY
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
    /// Operation was not valid for a directory
    Directory,
    /// A merge conflict exists and cannot continue
    MergeConflict,
    /// Hashsum mismatch in object
    HashsumMismatch,
    /// Unsaved changes in the index would be overwritten
    IndexDirty,
    /// Patch application failed
    ApplyFail,
    /// The object is not owned by the current user
    Owner,
    /// Timeout
    Timeout,
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
    /// Error manipulating a tag
    Tag,
    /// Invalid value in tree
    Tree,
    /// Hashing or packing error
    Indexer,
    /// Error from SSL
    Ssl,
    /// Error involving submodules
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
    /// Invalid patch data
    Patch,
    /// Error involving worktrees
    Worktree,
    /// Hash library error or SHA-1 collision
    Sha1,
    /// HTTP error
    Http,
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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Direction {
    /// Data will be fetched (read) from this remote.
    Fetch,
    /// Data will be pushed (written) to this remote.
    Push,
}

/// An enumeration of the operations that can be performed for the `reset`
/// method on a `Repository`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

/// An enumeration of all possible kinds of references.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ReferenceType {
    /// A reference which points at an object id.
    Direct,

    /// A reference which points at another reference.
    Symbolic,
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
    ProgramData = 1,
    /// System-wide configuration file, e.g. /etc/gitconfig
    System,
    /// XDG-compatible configuration file, e.g. ~/.config/git/config
    XDG,
    /// User-specific configuration, e.g. ~/.gitconfig
    Global,
    /// Repository specific config, e.g. $PWD/.git/config
    Local,
    ///  Worktree specific configuration file, e.g. $GIT_DIR/config.worktree
    Worktree,
    /// Application specific configuration file
    App,
    /// Highest level available
    Highest = -1,
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
    /// Orderings that may be specified for Revwalk iteration.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Sort: u32 {
        /// Sort the repository contents in no particular ordering.
        ///
        /// This sorting is arbitrary, implementation-specific, and subject to
        /// change at any time. This is the default sorting for new walkers.
        const NONE = raw::GIT_SORT_NONE as u32;

        /// Sort the repository contents in topological order (children before
        /// parents).
        ///
        /// This sorting mode can be combined with time sorting.
        const TOPOLOGICAL = raw::GIT_SORT_TOPOLOGICAL as u32;

        /// Sort the repository contents by commit time.
        ///
        /// This sorting mode can be combined with topological sorting.
        const TIME = raw::GIT_SORT_TIME as u32;

        /// Iterate through the repository contents in reverse order.
        ///
        /// This sorting mode can be combined with any others.
        const REVERSE = raw::GIT_SORT_REVERSE as u32;
    }
}

impl Sort {
    is_bit_set!(is_none, Sort::NONE);
    is_bit_set!(is_topological, Sort::TOPOLOGICAL);
    is_bit_set!(is_time, Sort::TIME);
    is_bit_set!(is_reverse, Sort::REVERSE);
}

bitflags! {
    /// Types of credentials that can be requested by a credential callback.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct CredentialType: u32 {
        #[allow(missing_docs)]
        const USER_PASS_PLAINTEXT = raw::GIT_CREDTYPE_USERPASS_PLAINTEXT as u32;
        #[allow(missing_docs)]
        const SSH_KEY = raw::GIT_CREDTYPE_SSH_KEY as u32;
        #[allow(missing_docs)]
        const SSH_MEMORY = raw::GIT_CREDTYPE_SSH_MEMORY as u32;
        #[allow(missing_docs)]
        const SSH_CUSTOM = raw::GIT_CREDTYPE_SSH_CUSTOM as u32;
        #[allow(missing_docs)]
        const DEFAULT = raw::GIT_CREDTYPE_DEFAULT as u32;
        #[allow(missing_docs)]
        const SSH_INTERACTIVE = raw::GIT_CREDTYPE_SSH_INTERACTIVE as u32;
        #[allow(missing_docs)]
        const USERNAME = raw::GIT_CREDTYPE_USERNAME as u32;
    }
}

impl CredentialType {
    is_bit_set!(is_user_pass_plaintext, CredentialType::USER_PASS_PLAINTEXT);
    is_bit_set!(is_ssh_key, CredentialType::SSH_KEY);
    is_bit_set!(is_ssh_memory, CredentialType::SSH_MEMORY);
    is_bit_set!(is_ssh_custom, CredentialType::SSH_CUSTOM);
    is_bit_set!(is_default, CredentialType::DEFAULT);
    is_bit_set!(is_ssh_interactive, CredentialType::SSH_INTERACTIVE);
    is_bit_set!(is_username, CredentialType::USERNAME);
}

impl Default for CredentialType {
    fn default() -> Self {
        CredentialType::DEFAULT
    }
}

bitflags! {
    /// Flags for the `flags` field of an IndexEntry.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct IndexEntryFlag: u16 {
        /// Set when the `extended_flags` field is valid.
        const EXTENDED = raw::GIT_INDEX_ENTRY_EXTENDED as u16;
        /// "Assume valid" flag
        const VALID = raw::GIT_INDEX_ENTRY_VALID as u16;
    }
}

impl IndexEntryFlag {
    is_bit_set!(is_extended, IndexEntryFlag::EXTENDED);
    is_bit_set!(is_valid, IndexEntryFlag::VALID);
}

bitflags! {
    /// Flags for the `extended_flags` field of an IndexEntry.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct IndexEntryExtendedFlag: u16 {
        /// An "intent to add" entry from "git add -N"
        const INTENT_TO_ADD = raw::GIT_INDEX_ENTRY_INTENT_TO_ADD as u16;
        /// Skip the associated worktree file, for sparse checkouts
        const SKIP_WORKTREE = raw::GIT_INDEX_ENTRY_SKIP_WORKTREE as u16;

        #[allow(missing_docs)]
        const UPTODATE = raw::GIT_INDEX_ENTRY_UPTODATE as u16;
    }
}

impl IndexEntryExtendedFlag {
    is_bit_set!(is_intent_to_add, IndexEntryExtendedFlag::INTENT_TO_ADD);
    is_bit_set!(is_skip_worktree, IndexEntryExtendedFlag::SKIP_WORKTREE);
    is_bit_set!(is_up_to_date, IndexEntryExtendedFlag::UPTODATE);
}

bitflags! {
    /// Flags for APIs that add files matching pathspec
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct IndexAddOption: u32 {
        #[allow(missing_docs)]
        const DEFAULT = raw::GIT_INDEX_ADD_DEFAULT as u32;
        #[allow(missing_docs)]
        const FORCE = raw::GIT_INDEX_ADD_FORCE as u32;
        #[allow(missing_docs)]
        const DISABLE_PATHSPEC_MATCH =
                raw::GIT_INDEX_ADD_DISABLE_PATHSPEC_MATCH as u32;
        #[allow(missing_docs)]
        const CHECK_PATHSPEC = raw::GIT_INDEX_ADD_CHECK_PATHSPEC as u32;
    }
}

impl IndexAddOption {
    is_bit_set!(is_default, IndexAddOption::DEFAULT);
    is_bit_set!(is_force, IndexAddOption::FORCE);
    is_bit_set!(
        is_disable_pathspec_match,
        IndexAddOption::DISABLE_PATHSPEC_MATCH
    );
    is_bit_set!(is_check_pathspec, IndexAddOption::CHECK_PATHSPEC);
}

impl Default for IndexAddOption {
    fn default() -> Self {
        IndexAddOption::DEFAULT
    }
}

bitflags! {
    /// Flags for `Repository::open_ext`
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct RepositoryOpenFlags: u32 {
        /// Only open the specified path; don't walk upward searching.
        const NO_SEARCH = raw::GIT_REPOSITORY_OPEN_NO_SEARCH as u32;
        /// Search across filesystem boundaries.
        const CROSS_FS = raw::GIT_REPOSITORY_OPEN_CROSS_FS as u32;
        /// Force opening as bare repository, and defer loading its config.
        const BARE = raw::GIT_REPOSITORY_OPEN_BARE as u32;
        /// Don't try appending `/.git` to the specified repository path.
        const NO_DOTGIT = raw::GIT_REPOSITORY_OPEN_NO_DOTGIT as u32;
        /// Respect environment variables like `$GIT_DIR`.
        const FROM_ENV = raw::GIT_REPOSITORY_OPEN_FROM_ENV as u32;
    }
}

impl RepositoryOpenFlags {
    is_bit_set!(is_no_search, RepositoryOpenFlags::NO_SEARCH);
    is_bit_set!(is_cross_fs, RepositoryOpenFlags::CROSS_FS);
    is_bit_set!(is_bare, RepositoryOpenFlags::BARE);
    is_bit_set!(is_no_dotgit, RepositoryOpenFlags::NO_DOTGIT);
    is_bit_set!(is_from_env, RepositoryOpenFlags::FROM_ENV);
}

bitflags! {
    /// Flags for the return value of `Repository::revparse`
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct RevparseMode: u32 {
        /// The spec targeted a single object
        const SINGLE = raw::GIT_REVPARSE_SINGLE as u32;
        /// The spec targeted a range of commits
        const RANGE = raw::GIT_REVPARSE_RANGE as u32;
        /// The spec used the `...` operator, which invokes special semantics.
        const MERGE_BASE = raw::GIT_REVPARSE_MERGE_BASE as u32;
    }
}

impl RevparseMode {
    is_bit_set!(is_no_single, RevparseMode::SINGLE);
    is_bit_set!(is_range, RevparseMode::RANGE);
    is_bit_set!(is_merge_base, RevparseMode::MERGE_BASE);
}

bitflags! {
    /// The results of `merge_analysis` indicating the merge opportunities.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct MergeAnalysis: u32 {
        /// No merge is possible.
        const ANALYSIS_NONE = raw::GIT_MERGE_ANALYSIS_NONE as u32;
        /// A "normal" merge; both HEAD and the given merge input have diverged
        /// from their common ancestor. The divergent commits must be merged.
        const ANALYSIS_NORMAL = raw::GIT_MERGE_ANALYSIS_NORMAL as u32;
        /// All given merge inputs are reachable from HEAD, meaning the
        /// repository is up-to-date and no merge needs to be performed.
        const ANALYSIS_UP_TO_DATE = raw::GIT_MERGE_ANALYSIS_UP_TO_DATE as u32;
        /// The given merge input is a fast-forward from HEAD and no merge
        /// needs to be performed.  Instead, the client can check out the
        /// given merge input.
        const ANALYSIS_FASTFORWARD = raw::GIT_MERGE_ANALYSIS_FASTFORWARD as u32;
        /// The HEAD of the current repository is "unborn" and does not point to
        /// a valid commit.  No merge can be performed, but the caller may wish
        /// to simply set HEAD to the target commit(s).
        const ANALYSIS_UNBORN = raw::GIT_MERGE_ANALYSIS_UNBORN as u32;
    }
}

impl MergeAnalysis {
    is_bit_set!(is_none, MergeAnalysis::ANALYSIS_NONE);
    is_bit_set!(is_normal, MergeAnalysis::ANALYSIS_NORMAL);
    is_bit_set!(is_up_to_date, MergeAnalysis::ANALYSIS_UP_TO_DATE);
    is_bit_set!(is_fast_forward, MergeAnalysis::ANALYSIS_FASTFORWARD);
    is_bit_set!(is_unborn, MergeAnalysis::ANALYSIS_UNBORN);
}

bitflags! {
    /// The user's stated preference for merges.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct MergePreference: u32 {
        /// No configuration was found that suggests a preferred behavior for
        /// merge.
        const NONE = raw::GIT_MERGE_PREFERENCE_NONE as u32;
        /// There is a `merge.ff=false` configuration setting, suggesting that
        /// the user does not want to allow a fast-forward merge.
        const NO_FAST_FORWARD = raw::GIT_MERGE_PREFERENCE_NO_FASTFORWARD as u32;
        /// There is a `merge.ff=only` configuration setting, suggesting that
        /// the user only wants fast-forward merges.
        const FASTFORWARD_ONLY = raw::GIT_MERGE_PREFERENCE_FASTFORWARD_ONLY as u32;
    }
}

impl MergePreference {
    is_bit_set!(is_none, MergePreference::NONE);
    is_bit_set!(is_no_fast_forward, MergePreference::NO_FAST_FORWARD);
    is_bit_set!(is_fastforward_only, MergePreference::FASTFORWARD_ONLY);
}

bitflags! {
    /// Flags controlling the behavior of ODB lookup operations
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct OdbLookupFlags: u32 {
        /// Don't call `git_odb_refresh` if the lookup fails. Useful when doing
        /// a batch of lookup operations for objects that may legitimately not
        /// exist. When using this flag, you may wish to manually call
        /// `git_odb_refresh` before processing a batch of objects.
        const NO_REFRESH = raw::GIT_ODB_LOOKUP_NO_REFRESH as u32;
    }
}

bitflags! {
    /// How to handle reference updates.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct RemoteUpdateFlags: u32 {
       /// Write the fetch results to FETCH_HEAD.
       const UPDATE_FETCHHEAD = raw::GIT_REMOTE_UPDATE_FETCHHEAD as u32;
       /// Report unchanged tips in the update_tips callback.
       const REPORT_UNCHANGED = raw::GIT_REMOTE_UPDATE_REPORT_UNCHANGED as u32;
    }
}

#[cfg(test)]
#[macro_use]
mod test;
#[macro_use]
mod panic;
mod attr;
mod call;
mod util;

pub mod build;
pub mod cert;
pub mod oid_array;
pub mod opts;
pub mod string_array;
pub mod transport;

mod apply;
mod blame;
mod blob;
mod branch;
mod buf;
mod cherrypick;
mod commit;
mod config;
mod cred;
mod describe;
mod diff;
mod email;
mod error;
mod index;
mod indexer;
mod mailmap;
mod mempack;
mod merge;
mod message;
mod note;
mod object;
mod odb;
mod oid;
mod packbuilder;
mod patch;
mod pathspec;
mod proxy_options;
mod push_update;
mod rebase;
mod reference;
mod reflog;
mod refspec;
mod remote;
mod remote_callbacks;
mod repo;
mod revert;
mod revspec;
mod revwalk;
mod signature;
mod stash;
mod status;
mod submodule;
mod tag;
mod tagforeach;
mod time;
mod tracing;
mod transaction;
mod tree;
mod treebuilder;
mod version;
mod worktree;

fn init() {
    static INIT: Once = Once::new();

    INIT.call_once(|| {
        openssl_env_init();
    });

    raw::init();
}

#[cfg(all(
    unix,
    not(target_os = "macos"),
    not(target_os = "ios"),
    feature = "https"
))]
fn openssl_env_init() {
    // Currently, libgit2 leverages OpenSSL for SSL support when cloning
    // repositories over HTTPS. This means that we're picking up an OpenSSL
    // dependency on non-Windows platforms (where it has its own HTTPS
    // subsystem). As a result, we need to link to OpenSSL.
    //
    // Now actually *linking* to OpenSSL isn't so hard. We just need to make
    // sure to use pkg-config to discover any relevant system dependencies for
    // differences between distributions like CentOS and Ubuntu. The actual
    // trickiness comes about when we start *distributing* the resulting
    // binaries. Currently Cargo is distributed in binary form as nightlies,
    // which means we're distributing a binary with OpenSSL linked in.
    //
    // For historical reasons, the Linux nightly builder is running a CentOS
    // distribution in order to have as much ABI compatibility with other
    // distributions as possible. Sadly, however, this compatibility does not
    // extend to OpenSSL. Currently OpenSSL has two major versions, 0.9 and 1.0,
    // which are incompatible (many ABI differences). The CentOS builder we
    // build on has version 1.0, as do most distributions today. Some still have
    // 0.9, however. This means that if we are to distribute the binaries built
    // by the CentOS machine, we would only be compatible with OpenSSL 1.0 and
    // we would fail to run (a dynamic linker error at runtime) on systems with
    // only 9.8 installed (hopefully).
    //
    // But wait, the plot thickens! Apparently CentOS has dubbed their OpenSSL
    // library as `libssl.so.10`, notably the `10` is included at the end. On
    // the other hand Ubuntu, for example, only distributes `libssl.so`. This
    // means that the binaries created at CentOS are hard-wired to probe for a
    // file called `libssl.so.10` at runtime (using the LD_LIBRARY_PATH), which
    // will not be found on ubuntu. The conclusion of this is that binaries
    // built on CentOS cannot be distributed to Ubuntu and run successfully.
    //
    // There are a number of sneaky things we could do, including, but not
    // limited to:
    //
    // 1. Create a shim program which runs "just before" cargo runs. The
    //    responsibility of this shim program would be to locate `libssl.so`,
    //    whatever it's called, on the current system, make sure there's a
    //    symlink *somewhere* called `libssl.so.10`, and then set up
    //    LD_LIBRARY_PATH and run the actual cargo.
    //
    //    This approach definitely seems unconventional, and is borderline
    //    overkill for this problem. It's also dubious if we can find a
    //    libssl.so reliably on the target system.
    //
    // 2. Somehow re-work the CentOS installation so that the linked-against
    //    library is called libssl.so instead of libssl.so.10
    //
    //    The problem with this approach is that systems with 0.9 installed will
    //    start to silently fail, due to also having libraries called libssl.so
    //    (probably symlinked under a more appropriate version).
    //
    // 3. Compile Cargo against both OpenSSL 1.0 *and* OpenSSL 0.9, and
    //    distribute both. Also make sure that the linked-against name of the
    //    library is `libssl.so`. At runtime we determine which version is
    //    installed, and we then the appropriate binary.
    //
    //    This approach clearly has drawbacks in terms of infrastructure and
    //    feasibility.
    //
    // 4. Build a nightly of Cargo for each distribution we'd like to support.
    //    You would then pick the appropriate Cargo nightly to install locally.
    //
    // So, with all this in mind, the decision was made to *statically* link
    // OpenSSL. This solves any problem of relying on a downstream OpenSSL
    // version being available. This does, however, open a can of worms related
    // to security issues. It's generally a good idea to dynamically link
    // OpenSSL as you'll get security updates over time without having to do
    // anything (the system administrator will update the local openssl
    // package). By statically linking, we're forfeiting this feature.
    //
    // The conclusion was made it is likely appropriate for the Cargo nightlies
    // to statically link OpenSSL, but highly encourage distributions and
    // packagers of Cargo to dynamically link OpenSSL. Packagers are targeting
    // one system and are distributing to only that system, so none of the
    // problems mentioned above would arise.
    //
    // In order to support this, a new package was made: openssl-static-sys.
    // This package currently performs a fairly simple task:
    //
    // 1. Run pkg-config to discover where openssl is installed.
    // 2. If openssl is installed in a nonstandard location, *and* static copies
    //    of the libraries are available, copy them to $OUT_DIR.
    //
    // This library will bring in libssl.a and libcrypto.a into the local build,
    // allowing them to be picked up by this crate. This allows us to configure
    // our own buildbots to have pkg-config point to these local pre-built
    // copies of a static OpenSSL (with very few dependencies) while allowing
    // most other builds of Cargo to naturally dynamically link OpenSSL.
    //
    // So in summary, if you're with me so far, we've statically linked OpenSSL
    // to the Cargo binary (or any binary, for that matter) and we're ready to
    // distribute it to *all* linux distributions. Remember that our original
    // intent for openssl was for HTTPS support, which implies that we need some
    // for of CA certificate store to validate certificates. This is normally
    // installed in a standard system location.
    //
    // Unfortunately, as one might imagine, OpenSSL is configured for where this
    // standard location is at *build time*, but it often varies widely
    // per-system. Consequently, it was discovered that OpenSSL will respect the
    // SSL_CERT_FILE and SSL_CERT_DIR environment variables in order to assist
    // in discovering the location of this file (hurray!).
    //
    // So, finally getting to the point, this function solely exists to support
    // our static builds of OpenSSL by probing for the "standard system
    // location" of certificates and setting relevant environment variable to
    // point to them.
    //
    // Ah, and as a final note, this is only a problem on Linux, not on OS X. On
    // OS X the OpenSSL binaries are stable enough that we can just rely on
    // dynamic linkage (plus they have some weird modifications to OpenSSL which
    // means we wouldn't want to link statically).
    openssl_probe::init_ssl_cert_env_vars();
}

#[cfg(any(
    windows,
    target_os = "macos",
    target_os = "ios",
    not(feature = "https")
))]
fn openssl_env_init() {}

unsafe fn opt_bytes<'a, T>(_anchor: &'a T, c: *const libc::c_char) -> Option<&'a [u8]> {
    if c.is_null() {
        None
    } else {
        Some(CStr::from_ptr(c).to_bytes())
    }
}

fn opt_cstr<T: IntoCString>(o: Option<T>) -> Result<Option<CString>, Error> {
    match o {
        Some(s) => s.into_c_string().map(Some),
        None => Ok(None),
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

    /// Determine if the given git_object_t is a valid loose object type.
    pub fn is_loose(&self) -> bool {
        unsafe { call!(raw::git_object_typeisloose(*self)) == 1 }
    }

    /// Convert a raw git_object_t to an ObjectType
    pub fn from_raw(raw: raw::git_object_t) -> Option<ObjectType> {
        match raw {
            raw::GIT_OBJECT_ANY => Some(ObjectType::Any),
            raw::GIT_OBJECT_COMMIT => Some(ObjectType::Commit),
            raw::GIT_OBJECT_TREE => Some(ObjectType::Tree),
            raw::GIT_OBJECT_BLOB => Some(ObjectType::Blob),
            raw::GIT_OBJECT_TAG => Some(ObjectType::Tag),
            _ => None,
        }
    }

    /// Convert this kind into its raw representation
    pub fn raw(&self) -> raw::git_object_t {
        call::convert(self)
    }

    /// Convert a string object type representation to its object type.
    pub fn from_str(s: &str) -> Option<ObjectType> {
        let raw = unsafe { call!(raw::git_object_string2type(CString::new(s).unwrap())) };
        ObjectType::from_raw(raw)
    }
}

impl fmt::Display for ObjectType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.str().fmt(f)
    }
}

impl ReferenceType {
    /// Convert an object type to its string representation.
    pub fn str(&self) -> &'static str {
        match self {
            ReferenceType::Direct => "direct",
            ReferenceType::Symbolic => "symbolic",
        }
    }

    /// Convert a raw git_reference_t to a ReferenceType.
    pub fn from_raw(raw: raw::git_reference_t) -> Option<ReferenceType> {
        match raw {
            raw::GIT_REFERENCE_DIRECT => Some(ReferenceType::Direct),
            raw::GIT_REFERENCE_SYMBOLIC => Some(ReferenceType::Symbolic),
            _ => None,
        }
    }
}

impl fmt::Display for ReferenceType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
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
            raw::GIT_CONFIG_LEVEL_WORKTREE => ConfigLevel::Worktree,
            raw::GIT_CONFIG_LEVEL_APP => ConfigLevel::App,
            raw::GIT_CONFIG_HIGHEST_LEVEL => ConfigLevel::Highest,
            n => panic!("unknown config level: {}", n),
        }
    }
}

impl SubmoduleIgnore {
    /// Converts a [`raw::git_submodule_ignore_t`] to a [`SubmoduleIgnore`]
    pub fn from_raw(raw: raw::git_submodule_ignore_t) -> Self {
        match raw {
            raw::GIT_SUBMODULE_IGNORE_UNSPECIFIED => SubmoduleIgnore::Unspecified,
            raw::GIT_SUBMODULE_IGNORE_NONE => SubmoduleIgnore::None,
            raw::GIT_SUBMODULE_IGNORE_UNTRACKED => SubmoduleIgnore::Untracked,
            raw::GIT_SUBMODULE_IGNORE_DIRTY => SubmoduleIgnore::Dirty,
            raw::GIT_SUBMODULE_IGNORE_ALL => SubmoduleIgnore::All,
            n => panic!("unknown submodule ignore rule: {}", n),
        }
    }
}

impl SubmoduleUpdate {
    /// Converts a [`raw::git_submodule_update_t`] to a [`SubmoduleUpdate`]
    pub fn from_raw(raw: raw::git_submodule_update_t) -> Self {
        match raw {
            raw::GIT_SUBMODULE_UPDATE_CHECKOUT => SubmoduleUpdate::Checkout,
            raw::GIT_SUBMODULE_UPDATE_REBASE => SubmoduleUpdate::Rebase,
            raw::GIT_SUBMODULE_UPDATE_MERGE => SubmoduleUpdate::Merge,
            raw::GIT_SUBMODULE_UPDATE_NONE => SubmoduleUpdate::None,
            raw::GIT_SUBMODULE_UPDATE_DEFAULT => SubmoduleUpdate::Default,
            n => panic!("unknown submodule update strategy: {}", n),
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
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct Status: u32 {
        #[allow(missing_docs)]
        const CURRENT = raw::GIT_STATUS_CURRENT as u32;

        #[allow(missing_docs)]
        const INDEX_NEW = raw::GIT_STATUS_INDEX_NEW as u32;
        #[allow(missing_docs)]
        const INDEX_MODIFIED = raw::GIT_STATUS_INDEX_MODIFIED as u32;
        #[allow(missing_docs)]
        const INDEX_DELETED = raw::GIT_STATUS_INDEX_DELETED as u32;
        #[allow(missing_docs)]
        const INDEX_RENAMED = raw::GIT_STATUS_INDEX_RENAMED as u32;
        #[allow(missing_docs)]
        const INDEX_TYPECHANGE = raw::GIT_STATUS_INDEX_TYPECHANGE as u32;

        #[allow(missing_docs)]
        const WT_NEW = raw::GIT_STATUS_WT_NEW as u32;
        #[allow(missing_docs)]
        const WT_MODIFIED = raw::GIT_STATUS_WT_MODIFIED as u32;
        #[allow(missing_docs)]
        const WT_DELETED = raw::GIT_STATUS_WT_DELETED as u32;
        #[allow(missing_docs)]
        const WT_TYPECHANGE = raw::GIT_STATUS_WT_TYPECHANGE as u32;
        #[allow(missing_docs)]
        const WT_RENAMED = raw::GIT_STATUS_WT_RENAMED as u32;

        #[allow(missing_docs)]
        const IGNORED = raw::GIT_STATUS_IGNORED as u32;
        #[allow(missing_docs)]
        const CONFLICTED = raw::GIT_STATUS_CONFLICTED as u32;
    }
}

impl Status {
    is_bit_set!(is_index_new, Status::INDEX_NEW);
    is_bit_set!(is_index_modified, Status::INDEX_MODIFIED);
    is_bit_set!(is_index_deleted, Status::INDEX_DELETED);
    is_bit_set!(is_index_renamed, Status::INDEX_RENAMED);
    is_bit_set!(is_index_typechange, Status::INDEX_TYPECHANGE);
    is_bit_set!(is_wt_new, Status::WT_NEW);
    is_bit_set!(is_wt_modified, Status::WT_MODIFIED);
    is_bit_set!(is_wt_deleted, Status::WT_DELETED);
    is_bit_set!(is_wt_typechange, Status::WT_TYPECHANGE);
    is_bit_set!(is_wt_renamed, Status::WT_RENAMED);
    is_bit_set!(is_ignored, Status::IGNORED);
    is_bit_set!(is_conflicted, Status::CONFLICTED);
}

bitflags! {
    /// Mode options for RepositoryInitOptions
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct RepositoryInitMode: u32 {
        /// Use permissions configured by umask - the default
        const SHARED_UMASK = raw::GIT_REPOSITORY_INIT_SHARED_UMASK as u32;
        /// Use `--shared=group` behavior, chmod'ing the new repo to be
        /// group writable and \"g+sx\" for sticky group assignment
        const SHARED_GROUP = raw::GIT_REPOSITORY_INIT_SHARED_GROUP as u32;
        /// Use `--shared=all` behavior, adding world readability.
        const SHARED_ALL = raw::GIT_REPOSITORY_INIT_SHARED_ALL as u32;
    }
}

impl RepositoryInitMode {
    is_bit_set!(is_shared_umask, RepositoryInitMode::SHARED_UMASK);
    is_bit_set!(is_shared_group, RepositoryInitMode::SHARED_GROUP);
    is_bit_set!(is_shared_all, RepositoryInitMode::SHARED_ALL);
}

/// What type of change is described by a `DiffDelta`?
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Delta {
    /// No changes
    Unmodified,
    /// Entry does not exist in old version
    Added,
    /// Entry does not exist in new version
    Deleted,
    /// Entry content changed between old and new
    Modified,
    /// Entry was renamed between old and new
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

/// Valid modes for index and tree entries.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FileMode {
    /// Unreadable
    Unreadable,
    /// Tree
    Tree,
    /// Blob
    Blob,
    /// Group writable blob. Obsolete mode kept for compatibility reasons
    BlobGroupWritable,
    /// Blob executable
    BlobExecutable,
    /// Link
    Link,
    /// Commit
    Commit,
}

impl From<FileMode> for i32 {
    fn from(mode: FileMode) -> i32 {
        match mode {
            FileMode::Unreadable => raw::GIT_FILEMODE_UNREADABLE as i32,
            FileMode::Tree => raw::GIT_FILEMODE_TREE as i32,
            FileMode::Blob => raw::GIT_FILEMODE_BLOB as i32,
            FileMode::BlobGroupWritable => raw::GIT_FILEMODE_BLOB_GROUP_WRITABLE as i32,
            FileMode::BlobExecutable => raw::GIT_FILEMODE_BLOB_EXECUTABLE as i32,
            FileMode::Link => raw::GIT_FILEMODE_LINK as i32,
            FileMode::Commit => raw::GIT_FILEMODE_COMMIT as i32,
        }
    }
}

impl From<FileMode> for u32 {
    fn from(mode: FileMode) -> u32 {
        match mode {
            FileMode::Unreadable => raw::GIT_FILEMODE_UNREADABLE as u32,
            FileMode::Tree => raw::GIT_FILEMODE_TREE as u32,
            FileMode::Blob => raw::GIT_FILEMODE_BLOB as u32,
            FileMode::BlobGroupWritable => raw::GIT_FILEMODE_BLOB_GROUP_WRITABLE as u32,
            FileMode::BlobExecutable => raw::GIT_FILEMODE_BLOB_EXECUTABLE as u32,
            FileMode::Link => raw::GIT_FILEMODE_LINK as u32,
            FileMode::Commit => raw::GIT_FILEMODE_COMMIT as u32,
        }
    }
}

bitflags! {
    /// Return codes for submodule status.
    ///
    /// A combination of these flags will be returned to describe the status of a
    /// submodule.  Depending on the "ignore" property of the submodule, some of
    /// the flags may never be returned because they indicate changes that are
    /// supposed to be ignored.
    ///
    /// Submodule info is contained in 4 places: the HEAD tree, the index, config
    /// files (both .git/config and .gitmodules), and the working directory.  Any
    /// or all of those places might be missing information about the submodule
    /// depending on what state the repo is in.  We consider all four places to
    /// build the combination of status flags.
    ///
    /// There are four values that are not really status, but give basic info
    /// about what sources of submodule data are available.  These will be
    /// returned even if ignore is set to "ALL".
    ///
    /// * IN_HEAD   - superproject head contains submodule
    /// * IN_INDEX  - superproject index contains submodule
    /// * IN_CONFIG - superproject gitmodules has submodule
    /// * IN_WD     - superproject workdir has submodule
    ///
    /// The following values will be returned so long as ignore is not "ALL".
    ///
    /// * INDEX_ADDED       - in index, not in head
    /// * INDEX_DELETED     - in head, not in index
    /// * INDEX_MODIFIED    - index and head don't match
    /// * WD_UNINITIALIZED  - workdir contains empty directory
    /// * WD_ADDED          - in workdir, not index
    /// * WD_DELETED        - in index, not workdir
    /// * WD_MODIFIED       - index and workdir head don't match
    ///
    /// The following can only be returned if ignore is "NONE" or "UNTRACKED".
    ///
    /// * WD_INDEX_MODIFIED - submodule workdir index is dirty
    /// * WD_WD_MODIFIED    - submodule workdir has modified files
    ///
    /// Lastly, the following will only be returned for ignore "NONE".
    ///
    /// * WD_UNTRACKED      - workdir contains untracked files
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct SubmoduleStatus: u32 {
        #[allow(missing_docs)]
        const IN_HEAD = raw::GIT_SUBMODULE_STATUS_IN_HEAD as u32;
        #[allow(missing_docs)]
        const IN_INDEX = raw::GIT_SUBMODULE_STATUS_IN_INDEX as u32;
        #[allow(missing_docs)]
        const IN_CONFIG = raw::GIT_SUBMODULE_STATUS_IN_CONFIG as u32;
        #[allow(missing_docs)]
        const IN_WD = raw::GIT_SUBMODULE_STATUS_IN_WD as u32;
        #[allow(missing_docs)]
        const INDEX_ADDED = raw::GIT_SUBMODULE_STATUS_INDEX_ADDED as u32;
        #[allow(missing_docs)]
        const INDEX_DELETED = raw::GIT_SUBMODULE_STATUS_INDEX_DELETED as u32;
        #[allow(missing_docs)]
        const INDEX_MODIFIED = raw::GIT_SUBMODULE_STATUS_INDEX_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_UNINITIALIZED =
                raw::GIT_SUBMODULE_STATUS_WD_UNINITIALIZED as u32;
        #[allow(missing_docs)]
        const WD_ADDED = raw::GIT_SUBMODULE_STATUS_WD_ADDED as u32;
        #[allow(missing_docs)]
        const WD_DELETED = raw::GIT_SUBMODULE_STATUS_WD_DELETED as u32;
        #[allow(missing_docs)]
        const WD_MODIFIED = raw::GIT_SUBMODULE_STATUS_WD_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_INDEX_MODIFIED =
                raw::GIT_SUBMODULE_STATUS_WD_INDEX_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_WD_MODIFIED = raw::GIT_SUBMODULE_STATUS_WD_WD_MODIFIED as u32;
        #[allow(missing_docs)]
        const WD_UNTRACKED = raw::GIT_SUBMODULE_STATUS_WD_UNTRACKED as u32;
    }
}

impl SubmoduleStatus {
    is_bit_set!(is_in_head, SubmoduleStatus::IN_HEAD);
    is_bit_set!(is_in_index, SubmoduleStatus::IN_INDEX);
    is_bit_set!(is_in_config, SubmoduleStatus::IN_CONFIG);
    is_bit_set!(is_in_wd, SubmoduleStatus::IN_WD);
    is_bit_set!(is_index_added, SubmoduleStatus::INDEX_ADDED);
    is_bit_set!(is_index_deleted, SubmoduleStatus::INDEX_DELETED);
    is_bit_set!(is_index_modified, SubmoduleStatus::INDEX_MODIFIED);
    is_bit_set!(is_wd_uninitialized, SubmoduleStatus::WD_UNINITIALIZED);
    is_bit_set!(is_wd_added, SubmoduleStatus::WD_ADDED);
    is_bit_set!(is_wd_deleted, SubmoduleStatus::WD_DELETED);
    is_bit_set!(is_wd_modified, SubmoduleStatus::WD_MODIFIED);
    is_bit_set!(is_wd_wd_modified, SubmoduleStatus::WD_WD_MODIFIED);
    is_bit_set!(is_wd_untracked, SubmoduleStatus::WD_UNTRACKED);
}

/// Submodule ignore values
///
/// These values represent settings for the `submodule.$name.ignore`
/// configuration value which says how deeply to look at the working
/// directory when getting the submodule status.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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

/// Submodule update values
///
/// These values represent settings for the `submodule.$name.update`
/// configuration value which says how to handle `git submodule update`
/// for this submodule. The value is usually set in the ".gitmodules"
/// file and copied to ".git/config" when the submodule is initialized.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum SubmoduleUpdate {
    /// The default; when a submodule is updated, checkout the new detached
    /// HEAD to the submodule directory.
    Checkout,
    /// Update by rebasing the current checked out branch onto the commit from
    /// the superproject.
    Rebase,
    /// Update by merging the commit in the superproject into the current
    /// checkout out branch of the submodule.
    Merge,
    /// Do not update this submodule even when the commit in the superproject
    /// is updated.
    None,
    /// Not used except as static initializer when we don't want any particular
    /// update rule to be specified.
    Default,
}

bitflags! {
    /// ...
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct PathspecFlags: u32 {
        /// Use the default pathspec matching configuration.
        const DEFAULT = raw::GIT_PATHSPEC_DEFAULT as u32;
        /// Force matching to ignore case, otherwise matching will use native
        /// case sensitivity of the platform filesystem.
        const IGNORE_CASE = raw::GIT_PATHSPEC_IGNORE_CASE as u32;
        /// Force case sensitive matches, otherwise match will use the native
        /// case sensitivity of the platform filesystem.
        const USE_CASE = raw::GIT_PATHSPEC_USE_CASE as u32;
        /// Disable glob patterns and just use simple string comparison for
        /// matching.
        const NO_GLOB = raw::GIT_PATHSPEC_NO_GLOB as u32;
        /// Means that match functions return the error code `NotFound` if no
        /// matches are found. By default no matches is a success.
        const NO_MATCH_ERROR = raw::GIT_PATHSPEC_NO_MATCH_ERROR as u32;
        /// Means that the list returned should track which patterns matched
        /// which files so that at the end of the match we can identify patterns
        /// that did not match any files.
        const FIND_FAILURES = raw::GIT_PATHSPEC_FIND_FAILURES as u32;
        /// Means that the list returned does not need to keep the actual
        /// matching filenames. Use this to just test if there were any matches
        /// at all or in combination with `PATHSPEC_FAILURES` to validate a
        /// pathspec.
        const FAILURES_ONLY = raw::GIT_PATHSPEC_FAILURES_ONLY as u32;
    }
}

impl PathspecFlags {
    is_bit_set!(is_default, PathspecFlags::DEFAULT);
    is_bit_set!(is_ignore_case, PathspecFlags::IGNORE_CASE);
    is_bit_set!(is_use_case, PathspecFlags::USE_CASE);
    is_bit_set!(is_no_glob, PathspecFlags::NO_GLOB);
    is_bit_set!(is_no_match_error, PathspecFlags::NO_MATCH_ERROR);
    is_bit_set!(is_find_failures, PathspecFlags::FIND_FAILURES);
    is_bit_set!(is_failures_only, PathspecFlags::FAILURES_ONLY);
}

impl Default for PathspecFlags {
    fn default() -> Self {
        PathspecFlags::DEFAULT
    }
}

bitflags! {
    /// Types of notifications emitted from checkouts.
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct CheckoutNotificationType: u32 {
        /// Notification about a conflict.
        const CONFLICT = raw::GIT_CHECKOUT_NOTIFY_CONFLICT as u32;
        /// Notification about a dirty file.
        const DIRTY = raw::GIT_CHECKOUT_NOTIFY_DIRTY as u32;
        /// Notification about an updated file.
        const UPDATED = raw::GIT_CHECKOUT_NOTIFY_UPDATED as u32;
        /// Notification about an untracked file.
        const UNTRACKED = raw::GIT_CHECKOUT_NOTIFY_UNTRACKED as u32;
        /// Notification about an ignored file.
        const IGNORED = raw::GIT_CHECKOUT_NOTIFY_IGNORED as u32;
    }
}

impl CheckoutNotificationType {
    is_bit_set!(is_conflict, CheckoutNotificationType::CONFLICT);
    is_bit_set!(is_dirty, CheckoutNotificationType::DIRTY);
    is_bit_set!(is_updated, CheckoutNotificationType::UPDATED);
    is_bit_set!(is_untracked, CheckoutNotificationType::UNTRACKED);
    is_bit_set!(is_ignored, CheckoutNotificationType::IGNORED);
}

/// Possible output formats for diff data
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
    /// git diff as used by git patch-id
    PatchId,
}

bitflags! {
    /// Formatting options for diff stats
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct DiffStatsFormat: raw::git_diff_stats_format_t {
        /// Don't generate any stats
        const NONE = raw::GIT_DIFF_STATS_NONE;
        /// Equivalent of `--stat` in git
        const FULL = raw::GIT_DIFF_STATS_FULL;
        /// Equivalent of `--shortstat` in git
        const SHORT = raw::GIT_DIFF_STATS_SHORT;
        /// Equivalent of `--numstat` in git
        const NUMBER = raw::GIT_DIFF_STATS_NUMBER;
        /// Extended header information such as creations, renames and mode
        /// changes, equivalent of `--summary` in git
        const INCLUDE_SUMMARY = raw::GIT_DIFF_STATS_INCLUDE_SUMMARY;
    }
}

impl DiffStatsFormat {
    is_bit_set!(is_none, DiffStatsFormat::NONE);
    is_bit_set!(is_full, DiffStatsFormat::FULL);
    is_bit_set!(is_short, DiffStatsFormat::SHORT);
    is_bit_set!(is_number, DiffStatsFormat::NUMBER);
    is_bit_set!(is_include_summary, DiffStatsFormat::INCLUDE_SUMMARY);
}

/// Automatic tag following options.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
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
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum FetchPrune {
    /// Use the setting from the configuration
    Unspecified,
    /// Force pruning on
    On,
    /// Force pruning off
    Off,
}

#[allow(missing_docs)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum StashApplyProgress {
    /// None
    None,
    /// Loading the stashed data from the object database
    LoadingStash,
    /// The stored index is being analyzed
    AnalyzeIndex,
    /// The modified files are being analyzed
    AnalyzeModified,
    /// The untracked and ignored files are being analyzed
    AnalyzeUntracked,
    /// The untracked files are being written to disk
    CheckoutUntracked,
    /// The modified files are being written to disk
    CheckoutModified,
    /// The stash was applied successfully
    Done,
}

bitflags! {
    #[allow(missing_docs)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct StashApplyFlags: u32 {
        #[allow(missing_docs)]
        const DEFAULT = raw::GIT_STASH_APPLY_DEFAULT as u32;
        /// Try to reinstate not only the working tree's changes,
        /// but also the index's changes.
        const REINSTATE_INDEX = raw::GIT_STASH_APPLY_REINSTATE_INDEX as u32;
    }
}

impl StashApplyFlags {
    is_bit_set!(is_default, StashApplyFlags::DEFAULT);
    is_bit_set!(is_reinstate_index, StashApplyFlags::REINSTATE_INDEX);
}

impl Default for StashApplyFlags {
    fn default() -> Self {
        StashApplyFlags::DEFAULT
    }
}

bitflags! {
    #[allow(missing_docs)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct StashFlags: u32 {
        #[allow(missing_docs)]
        const DEFAULT = raw::GIT_STASH_DEFAULT as u32;
        /// All changes already added to the index are left intact in
        /// the working directory
        const KEEP_INDEX = raw::GIT_STASH_KEEP_INDEX as u32;
        /// All untracked files are also stashed and then cleaned up
        /// from the working directory
        const INCLUDE_UNTRACKED = raw::GIT_STASH_INCLUDE_UNTRACKED as u32;
        /// All ignored files are also stashed and then cleaned up from
        /// the working directory
        const INCLUDE_IGNORED = raw::GIT_STASH_INCLUDE_IGNORED as u32;
        /// All changes in the index and working directory are left intact
        const KEEP_ALL = raw::GIT_STASH_KEEP_ALL as u32;
    }
}

impl StashFlags {
    is_bit_set!(is_default, StashFlags::DEFAULT);
    is_bit_set!(is_keep_index, StashFlags::KEEP_INDEX);
    is_bit_set!(is_include_untracked, StashFlags::INCLUDE_UNTRACKED);
    is_bit_set!(is_include_ignored, StashFlags::INCLUDE_IGNORED);
}

impl Default for StashFlags {
    fn default() -> Self {
        StashFlags::DEFAULT
    }
}

bitflags! {
    #[allow(missing_docs)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct AttrCheckFlags: u32 {
        /// Check the working directory, then the index.
        const FILE_THEN_INDEX = raw::GIT_ATTR_CHECK_FILE_THEN_INDEX as u32;
        /// Check the index, then the working directory.
        const INDEX_THEN_FILE = raw::GIT_ATTR_CHECK_INDEX_THEN_FILE as u32;
        /// Check the index only.
        const INDEX_ONLY = raw::GIT_ATTR_CHECK_INDEX_ONLY as u32;
        /// Do not use the system gitattributes file.
        const NO_SYSTEM = raw::GIT_ATTR_CHECK_NO_SYSTEM as u32;
    }
}

impl Default for AttrCheckFlags {
    fn default() -> Self {
        AttrCheckFlags::FILE_THEN_INDEX
    }
}

bitflags! {
    #[allow(missing_docs)]
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct DiffFlags: u32 {
        /// File(s) treated as binary data.
        const BINARY = raw::GIT_DIFF_FLAG_BINARY as u32;
        /// File(s) treated as text data.
        const NOT_BINARY = raw::GIT_DIFF_FLAG_NOT_BINARY as u32;
        /// `id` value is known correct.
        const VALID_ID = raw::GIT_DIFF_FLAG_VALID_ID as u32;
        /// File exists at this side of the delta.
        const EXISTS = raw::GIT_DIFF_FLAG_EXISTS as u32;
    }
}

impl DiffFlags {
    is_bit_set!(is_binary, DiffFlags::BINARY);
    is_bit_set!(is_not_binary, DiffFlags::NOT_BINARY);
    is_bit_set!(has_valid_id, DiffFlags::VALID_ID);
    is_bit_set!(exists, DiffFlags::EXISTS);
}

bitflags! {
    /// Options for [`Reference::normalize_name`].
    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub struct ReferenceFormat: u32 {
        /// No particular normalization.
        const NORMAL = raw::GIT_REFERENCE_FORMAT_NORMAL as u32;
        /// Control whether one-level refname are accepted (i.e., refnames that
        /// do not contain multiple `/`-separated components). Those are
        /// expected to be written only using uppercase letters and underscore
        /// (e.g. `HEAD`, `FETCH_HEAD`).
        const ALLOW_ONELEVEL = raw::GIT_REFERENCE_FORMAT_ALLOW_ONELEVEL as u32;
        /// Interpret the provided name as a reference pattern for a refspec (as
        /// used with remote repositories). If this option is enabled, the name
        /// is allowed to contain a single `*` in place of a full pathname
        /// components (e.g., `foo/*/bar` but not `foo/bar*`).
        const REFSPEC_PATTERN = raw::GIT_REFERENCE_FORMAT_REFSPEC_PATTERN as u32;
        /// Interpret the name as part of a refspec in shorthand form so the
        /// `ALLOW_ONELEVEL` naming rules aren't enforced and `main` becomes a
        /// valid name.
        const REFSPEC_SHORTHAND = raw::GIT_REFERENCE_FORMAT_REFSPEC_SHORTHAND as u32;
    }
}

impl ReferenceFormat {
    is_bit_set!(is_allow_onelevel, ReferenceFormat::ALLOW_ONELEVEL);
    is_bit_set!(is_refspec_pattern, ReferenceFormat::REFSPEC_PATTERN);
    is_bit_set!(is_refspec_shorthand, ReferenceFormat::REFSPEC_SHORTHAND);
}

impl Default for ReferenceFormat {
    fn default() -> Self {
        ReferenceFormat::NORMAL
    }
}

#[cfg(test)]
mod tests {
    use super::{FileMode, ObjectType};

    #[test]
    fn convert() {
        assert_eq!(ObjectType::Blob.str(), "blob");
        assert_eq!(ObjectType::from_str("blob"), Some(ObjectType::Blob));
        assert!(ObjectType::Blob.is_loose());
    }

    #[test]
    fn convert_filemode() {
        assert_eq!(i32::from(FileMode::Blob), 0o100644);
        assert_eq!(i32::from(FileMode::BlobGroupWritable), 0o100664);
        assert_eq!(i32::from(FileMode::BlobExecutable), 0o100755);
        assert_eq!(u32::from(FileMode::Blob), 0o100644);
        assert_eq!(u32::from(FileMode::BlobGroupWritable), 0o100664);
        assert_eq!(u32::from(FileMode::BlobExecutable), 0o100755);
    }

    #[test]
    fn bitflags_partial_eq() {
        use super::{
            AttrCheckFlags, CheckoutNotificationType, CredentialType, DiffFlags, DiffStatsFormat,
            IndexAddOption, IndexEntryExtendedFlag, IndexEntryFlag, MergeAnalysis, MergePreference,
            OdbLookupFlags, PathspecFlags, ReferenceFormat, RepositoryInitMode,
            RepositoryOpenFlags, RevparseMode, Sort, StashApplyFlags, StashFlags, Status,
            SubmoduleStatus,
        };

        assert_eq!(
            AttrCheckFlags::FILE_THEN_INDEX,
            AttrCheckFlags::FILE_THEN_INDEX
        );
        assert_eq!(
            CheckoutNotificationType::CONFLICT,
            CheckoutNotificationType::CONFLICT
        );
        assert_eq!(
            CredentialType::USER_PASS_PLAINTEXT,
            CredentialType::USER_PASS_PLAINTEXT
        );
        assert_eq!(DiffFlags::BINARY, DiffFlags::BINARY);
        assert_eq!(
            DiffStatsFormat::INCLUDE_SUMMARY,
            DiffStatsFormat::INCLUDE_SUMMARY
        );
        assert_eq!(
            IndexAddOption::CHECK_PATHSPEC,
            IndexAddOption::CHECK_PATHSPEC
        );
        assert_eq!(
            IndexEntryExtendedFlag::INTENT_TO_ADD,
            IndexEntryExtendedFlag::INTENT_TO_ADD
        );
        assert_eq!(IndexEntryFlag::EXTENDED, IndexEntryFlag::EXTENDED);
        assert_eq!(
            MergeAnalysis::ANALYSIS_FASTFORWARD,
            MergeAnalysis::ANALYSIS_FASTFORWARD
        );
        assert_eq!(
            MergePreference::FASTFORWARD_ONLY,
            MergePreference::FASTFORWARD_ONLY
        );
        assert_eq!(OdbLookupFlags::NO_REFRESH, OdbLookupFlags::NO_REFRESH);
        assert_eq!(PathspecFlags::FAILURES_ONLY, PathspecFlags::FAILURES_ONLY);
        assert_eq!(
            ReferenceFormat::ALLOW_ONELEVEL,
            ReferenceFormat::ALLOW_ONELEVEL
        );
        assert_eq!(
            RepositoryInitMode::SHARED_ALL,
            RepositoryInitMode::SHARED_ALL
        );
        assert_eq!(RepositoryOpenFlags::CROSS_FS, RepositoryOpenFlags::CROSS_FS);
        assert_eq!(RevparseMode::RANGE, RevparseMode::RANGE);
        assert_eq!(Sort::REVERSE, Sort::REVERSE);
        assert_eq!(
            StashApplyFlags::REINSTATE_INDEX,
            StashApplyFlags::REINSTATE_INDEX
        );
        assert_eq!(StashFlags::INCLUDE_IGNORED, StashFlags::INCLUDE_IGNORED);
        assert_eq!(Status::WT_MODIFIED, Status::WT_MODIFIED);
        assert_eq!(SubmoduleStatus::WD_ADDED, SubmoduleStatus::WD_ADDED);
    }
}
