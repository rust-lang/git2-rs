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

#![doc(html_root_url = "https://docs.rs/git2/0.6")]
#![allow(trivial_numeric_casts, trivial_casts)]
#![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

extern crate libc;
extern crate url;
extern crate libgit2_sys as raw;
#[macro_use] extern crate bitflags;
#[macro_use] extern crate log;
#[cfg(test)] extern crate tempdir;

use std::ffi::{CStr, CString};
use std::fmt;
use std::str;
use std::sync::{Once, ONCE_INIT};

pub use blame::{Blame, BlameHunk, BlameIter, BlameOptions};
pub use blob::{Blob, BlobWriter};
pub use branch::{Branch, Branches};
pub use buf::Buf;
pub use commit::{Commit, Parents};
pub use config::{Config, ConfigEntry, ConfigEntries};
pub use cred::{Cred, CredentialHelper};
pub use describe::{Describe, DescribeFormatOptions, DescribeOptions};
pub use diff::{Diff, DiffDelta, DiffFile, DiffOptions, Deltas};
pub use diff::{DiffBinary, DiffBinaryFile, DiffBinaryKind};
pub use diff::{DiffLine, DiffHunk, DiffStats, DiffFindOptions};
pub use error::Error;
pub use index::{Index, IndexConflict, IndexConflicts, IndexEntry, IndexEntries, IndexMatchedPath};
pub use merge::{AnnotatedCommit, MergeOptions};
pub use message::{message_prettify, DEFAULT_COMMENT_CHAR};
pub use note::{Note, Notes};
pub use object::Object;
pub use oid::Oid;
pub use packbuilder::{PackBuilder, PackBuilderStage};
pub use pathspec::{Pathspec, PathspecMatchList, PathspecFailedEntries};
pub use pathspec::{PathspecDiffEntries, PathspecEntries};
pub use patch::Patch;
pub use proxy_options::ProxyOptions;
pub use rebase::{Rebase, RebaseOptions, RebaseOperation, RebaseOperationType};
pub use reference::{Reference, References, ReferenceNames};
pub use reflog::{Reflog, ReflogEntry, ReflogIter};
pub use refspec::Refspec;
pub use remote::{Remote, RemoteConnection, Refspecs, RemoteHead, FetchOptions, PushOptions};
pub use remote_callbacks::{RemoteCallbacks, Credentials, TransferProgress};
pub use remote_callbacks::{TransportMessage, Progress, UpdateTips};
pub use repo::{Repository, RepositoryInitOptions};
pub use revspec::Revspec;
pub use revwalk::Revwalk;
pub use signature::Signature;
pub use status::{StatusOptions, Statuses, StatusIter, StatusEntry, StatusShow};
pub use stash::{StashApplyOptions, StashCb, StashApplyProgressCb};
pub use submodule::{Submodule, SubmoduleUpdateOptions};
pub use tag::Tag;
pub use time::{Time, IndexTime};
pub use tree::{Tree, TreeEntry, TreeIter, TreeWalkMode, TreeWalkResult};
pub use treebuilder::TreeBuilder;
pub use odb::{Odb, OdbObject, OdbReader, OdbWriter};
pub use util::IntoCString;

// Create a convinience method on bitflag struct which checks the given flag
macro_rules! is_bit_set {
    ($name:ident, $flag:expr) => (
        #[allow(missing_docs)]
        pub fn $name(&self) -> bool {
            self.intersects($flag)
        }
    )
}

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

/// An enumeration of all possile kinds of references.
#[derive(PartialEq, Eq, Copy, Clone, Debug)]
pub enum ReferenceType {
    /// A reference which points at an object id.
    Oid,

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
    /// Orderings that may be specified for Revwalk iteration.
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
    pub struct IndexEntryFlag: u16 {
        /// Set when the `extended_flags` field is valid.
        const EXTENDED = raw::GIT_IDXENTRY_EXTENDED as u16;
        /// "Assume valid" flag
        const VALID = raw::GIT_IDXENTRY_VALID as u16;
    }
}

impl IndexEntryFlag {
    is_bit_set!(is_extended, IndexEntryFlag::EXTENDED);
    is_bit_set!(is_valid, IndexEntryFlag::VALID);
}

bitflags! {
    /// Flags for the `extended_flags` field of an IndexEntry.
    pub struct IndexEntryExtendedFlag: u16 {
        /// An "intent to add" entry from "git add -N"
        const INTENT_TO_ADD = raw::GIT_IDXENTRY_INTENT_TO_ADD as u16;
        /// Skip the associated worktree file, for sparse checkouts
        const SKIP_WORKTREE = raw::GIT_IDXENTRY_SKIP_WORKTREE as u16;
        /// Reserved for a future on-disk extended flag
        const EXTENDED2 = raw::GIT_IDXENTRY_EXTENDED2 as u16;

        #[allow(missing_docs)]
        const UPDATE = raw::GIT_IDXENTRY_UPDATE as u16;
        #[allow(missing_docs)]
        const REMOVE = raw::GIT_IDXENTRY_REMOVE as u16;
        #[allow(missing_docs)]
        const UPTODATE = raw::GIT_IDXENTRY_UPTODATE as u16;
        #[allow(missing_docs)]
        const ADDED = raw::GIT_IDXENTRY_ADDED as u16;

        #[allow(missing_docs)]
        const HASHED = raw::GIT_IDXENTRY_HASHED as u16;
        #[allow(missing_docs)]
        const UNHASHED = raw::GIT_IDXENTRY_UNHASHED as u16;
        #[allow(missing_docs)]
        const WT_REMOVE = raw::GIT_IDXENTRY_WT_REMOVE as u16;
        #[allow(missing_docs)]
        const CONFLICTED = raw::GIT_IDXENTRY_CONFLICTED as u16;

        #[allow(missing_docs)]
        const UNPACKED = raw::GIT_IDXENTRY_UNPACKED as u16;
        #[allow(missing_docs)]
        const NEW_SKIP_WORKTREE = raw::GIT_IDXENTRY_NEW_SKIP_WORKTREE as u16;
    }
}

impl IndexEntryExtendedFlag {
    is_bit_set!(is_intent_to_add, IndexEntryExtendedFlag::INTENT_TO_ADD);
    is_bit_set!(is_skip_worktree, IndexEntryExtendedFlag::SKIP_WORKTREE);
    is_bit_set!(is_extended2, IndexEntryExtendedFlag::EXTENDED2);
    is_bit_set!(is_update, IndexEntryExtendedFlag::UPDATE);
    is_bit_set!(is_remove, IndexEntryExtendedFlag::REMOVE);
    is_bit_set!(is_up_to_date, IndexEntryExtendedFlag::UPTODATE);
    is_bit_set!(is_added, IndexEntryExtendedFlag::ADDED);
    is_bit_set!(is_hashed, IndexEntryExtendedFlag::HASHED);
    is_bit_set!(is_unhashed, IndexEntryExtendedFlag::UNHASHED);
    is_bit_set!(is_wt_remove, IndexEntryExtendedFlag::WT_REMOVE);
    is_bit_set!(is_conflicted, IndexEntryExtendedFlag::CONFLICTED);
    is_bit_set!(is_unpacked, IndexEntryExtendedFlag::UNPACKED);
    is_bit_set!(is_new_skip_worktree, IndexEntryExtendedFlag::NEW_SKIP_WORKTREE);
}

bitflags! {
    /// Flags for APIs that add files matching pathspec
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
    is_bit_set!(is_disable_pathspec_match, IndexAddOption::DISABLE_PATHSPEC_MATCH);
    is_bit_set!(is_check_pathspec, IndexAddOption::CHECK_PATHSPEC);
}

impl Default for IndexAddOption {
    fn default() -> Self {
        IndexAddOption::DEFAULT
    }
}

bitflags! {
    /// Flags for `Repository::open_ext`
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
mod error;
mod index;
mod merge;
mod message;
mod note;
mod object;
mod odb;
mod oid;
mod packbuilder;
mod pathspec;
mod patch;
mod proxy_options;
mod rebase;
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
mod stash;
mod tag;
mod time;
mod tree;
mod treebuilder;

fn init() {
    static INIT: Once = ONCE_INIT;

    INIT.call_once(|| {
        openssl_env_init();
    });

    raw::init();
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), feature = "https"))]
fn openssl_env_init() {
    extern crate openssl_probe;

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

#[cfg(any(windows, target_os = "macos", target_os = "ios", not(feature = "https")))]
fn openssl_env_init() {}

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

impl ReferenceType {
    /// Convert an object type to its string representation.
    pub fn str(&self) -> &'static str {
        match self {
            &ReferenceType::Oid => "oid",
            &ReferenceType::Symbolic => "symbolic",
        }
    }

    /// Convert a raw git_ref_t to a ReferenceType.
    pub fn from_raw(raw: raw::git_ref_t) -> Option<ReferenceType> {
        match raw {
            raw::GIT_REF_OID => Some(ReferenceType::Oid),
            raw::GIT_REF_SYMBOLIC => Some(ReferenceType::Symbolic),
            _ => None,
        }
    }
}

impl fmt::Display for ReferenceType {
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
    is_bit_set!(is_current, Status::CURRENT);
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
    /// * WD_UNTRACKED      - wd contains untracked files
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
    pub struct PathspecFlags: u32 {
        /// Use the default pathspec matching configuration.
        const DEFAULT = raw::GIT_PATHSPEC_DEFAULT as u32;
        /// Force matching to ignore case, otherwise matching will use native
        /// case sensitivity fo the platform filesystem.
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

#[allow(missing_docs)]
#[derive(Debug)]
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
