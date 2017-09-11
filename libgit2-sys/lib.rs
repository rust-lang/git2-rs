#![doc(html_root_url = "http://alexcrichton.com/git2-rs")]
#![allow(non_camel_case_types)]

extern crate libc;
#[cfg(feature = "ssh")]
extern crate libssh2_sys as libssh2;
#[cfg(feature = "curl")]
extern crate curl_sys;
#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), feature = "https"))]
extern crate openssl_sys;
extern crate libz_sys as libz;

use libc::{c_int, c_char, c_uint, size_t, c_uchar, c_void};

pub const GIT_OID_RAWSZ: usize = 20;
pub const GIT_OID_HEXSZ: usize = GIT_OID_RAWSZ * 2;
pub const GIT_CLONE_OPTIONS_VERSION: c_uint = 1;
pub const GIT_STASH_APPLY_OPTIONS_VERSION: c_uint = 1;
pub const GIT_CHECKOUT_OPTIONS_VERSION: c_uint = 1;
pub const GIT_MERGE_OPTIONS_VERSION: c_uint = 1;
pub const GIT_REMOTE_CALLBACKS_VERSION: c_uint = 1;
pub const GIT_STATUS_OPTIONS_VERSION: c_uint = 1;
pub const GIT_BLAME_OPTIONS_VERSION: c_uint = 1;
pub const GIT_PROXY_OPTIONS_VERSION: c_uint = 1;
pub const GIT_SUBMODULE_UPDATE_OPTIONS_VERSION: c_uint = 1;

macro_rules! git_enum {
    (pub enum $name:ident { $($variants:tt)* }) => {
        #[cfg(target_env = "msvc")]
        pub type $name = i32;
        #[cfg(not(target_env = "msvc"))]
        pub type $name = u32;
        git_enum!(gen, $name, 0, $($variants)*);
    };
    (pub enum $name:ident: $t:ty { $($variants:tt)* }) => {
        pub type $name = $t;
        git_enum!(gen, $name, 0, $($variants)*);
    };
    (gen, $name:ident, $val:expr, $variant:ident, $($rest:tt)*) => {
        pub const $variant: $name = $val;
        git_enum!(gen, $name, $val+1, $($rest)*);
    };
    (gen, $name:ident, $val:expr, $variant:ident = $e:expr, $($rest:tt)*) => {
        pub const $variant: $name = $e;
        git_enum!(gen, $name, $e+1, $($rest)*);
    };
    (gen, $name:ident, $val:expr, ) => {}
}

pub enum git_blob {}
pub enum git_branch_iterator {}
pub enum git_blame {}
pub enum git_commit {}
pub enum git_config {}
pub enum git_config_iterator {}
pub enum git_index {}
pub enum git_object {}
pub enum git_reference {}
pub enum git_reference_iterator {}
pub enum git_annotated_commit {}
pub enum git_refspec {}
pub enum git_remote {}
pub enum git_repository {}
pub enum git_revwalk {}
pub enum git_submodule {}
pub enum git_tag {}
pub enum git_tree {}
pub enum git_tree_entry {}
pub enum git_treebuilder {}
pub enum git_push {}
pub enum git_note {}
pub enum git_note_iterator {}
pub enum git_status_list {}
pub enum git_pathspec {}
pub enum git_pathspec_match_list {}
pub enum git_diff {}
pub enum git_diff_stats {}
pub enum git_patch {}
pub enum git_reflog {}
pub enum git_reflog_entry {}
pub enum git_describe_result {}
pub enum git_packbuilder {}
pub enum git_odb {}
pub enum git_odb_stream {}

#[repr(C)]
pub struct git_revspec {
    pub from: *mut git_object,
    pub to: *mut git_object,
    pub flags: c_uint,
}

#[repr(C)]
pub struct git_error {
    pub message: *mut c_char,
    pub klass: c_int,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct git_oid {
    pub id: [u8; GIT_OID_RAWSZ],
}

#[repr(C)]
#[derive(Copy)]
pub struct git_strarray {
    pub strings: *mut *mut c_char,
    pub count: size_t,
}
impl Clone for git_strarray {
    fn clone(&self) -> git_strarray { *self }
}

#[repr(C)]
#[derive(Copy)]
pub struct git_oidarray {
    pub ids: *mut git_oid,
    pub count: size_t,
}
impl Clone for git_oidarray {
    fn clone(&self) -> git_oidarray { *self }
}

#[repr(C)]
pub struct git_signature {
    pub name: *mut c_char,
    pub email: *mut c_char,
    pub when: git_time,
}

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct git_time {
    pub time: git_time_t,
    pub offset: c_int,
}

pub type git_off_t = i64;
pub type git_time_t = i64;

git_enum! {
    pub enum git_revparse_mode_t {
        GIT_REVPARSE_SINGLE = 1 << 0,
        GIT_REVPARSE_RANGE = 1 << 1,
        GIT_REVPARSE_MERGE_BASE = 1 << 2,
    }
}

git_enum! {
    pub enum git_error_code: c_int {
        GIT_OK = 0,

        GIT_ERROR = -1,
        GIT_ENOTFOUND = -3,
        GIT_EEXISTS = -4,
        GIT_EAMBIGUOUS = -5,
        GIT_EBUFS = -6,
        GIT_EUSER = -7,
        GIT_EBAREREPO = -8,
        GIT_EUNBORNBRANCH = -9,
        GIT_EUNMERGED = -10,
        GIT_ENONFASTFORWARD = -11,
        GIT_EINVALIDSPEC = -12,
        GIT_ECONFLICT = -13,
        GIT_ELOCKED = -14,
        GIT_EMODIFIED = -15,
        GIT_EAUTH = -16,
        GIT_ECERTIFICATE = -17,
        GIT_EAPPLIED = -18,
        GIT_EPEEL = -19,
        GIT_EEOF = -20,
        GIT_EINVALID = -21,
        GIT_EUNCOMMITTED = -22,
        GIT_EDIRECTORY = -23,
        GIT_EMERGECONFLICT = -24,
        GIT_PASSTHROUGH = -30,
        GIT_ITEROVER = -31,
    }
}

git_enum! {
    pub enum git_error_t {
        GITERR_NONE = 0,
        GITERR_NOMEMORY,
        GITERR_OS,
        GITERR_INVALID,
        GITERR_REFERENCE,
        GITERR_ZLIB,
        GITERR_REPOSITORY,
        GITERR_CONFIG,
        GITERR_REGEX,
        GITERR_ODB,
        GITERR_INDEX,
        GITERR_OBJECT,
        GITERR_NET,
        GITERR_TAG,
        GITERR_TREE,
        GITERR_INDEXER,
        GITERR_SSL,
        GITERR_SUBMODULE,
        GITERR_THREAD,
        GITERR_STASH,
        GITERR_CHECKOUT,
        GITERR_FETCHHEAD,
        GITERR_MERGE,
        GITERR_SSH,
        GITERR_FILTER,
        GITERR_REVERT,
        GITERR_CALLBACK,
        GITERR_CHERRYPICK,
        GITERR_DESCRIBE,
        GITERR_REBASE,
        GITERR_FILESYSTEM,
    }
}

git_enum! {
    pub enum git_repository_state_t {
        GIT_REPOSITORY_STATE_NONE,
        GIT_REPOSITORY_STATE_MERGE,
        GIT_REPOSITORY_STATE_REVERT,
        GIT_REPOSITORY_STATE_REVERT_SEQUENCE,
        GIT_REPOSITORY_STATE_CHERRYPICK,
        GIT_REPOSITORY_STATE_CHERRYPICK_SEQUENCE,
        GIT_REPOSITORY_STATE_BISECT,
        GIT_REPOSITORY_STATE_REBASE,
        GIT_REPOSITORY_STATE_REBASE_INTERACTIVE,
        GIT_REPOSITORY_STATE_REBASE_MERGE,
        GIT_REPOSITORY_STATE_APPLY_MAILBOX,
        GIT_REPOSITORY_STATE_APPLY_MAILBOX_OR_REBASE,
    }
}

git_enum! {
    pub enum git_direction {
        GIT_DIRECTION_FETCH,
        GIT_DIRECTION_PUSH,
    }
}

#[repr(C)]
pub struct git_clone_options {
    pub version: c_uint,
    pub checkout_opts: git_checkout_options,
    pub fetch_opts: git_fetch_options,
    pub bare: c_int,
    pub local: git_clone_local_t,
    pub checkout_branch: *const c_char,
    pub repository_cb: Option<git_repository_create_cb>,
    pub repository_cb_payload: *mut c_void,
    pub remote_cb: Option<git_remote_create_cb>,
    pub remote_cb_payload: *mut c_void,
}

git_enum! {
    pub enum git_clone_local_t {
        GIT_CLONE_LOCAL_AUTO,
        GIT_CLONE_LOCAL,
        GIT_CLONE_NO_LOCAL,
        GIT_CLONE_LOCAL_NO_LINKS,
    }
}

#[repr(C)]
pub struct git_checkout_options {
    pub version: c_uint,
    pub checkout_strategy: c_uint,
    pub disable_filters: c_int,
    pub dir_mode: c_uint,
    pub file_mode: c_uint,
    pub file_open_flags: c_int,
    pub notify_flags: c_uint,
    pub notify_cb: Option<git_checkout_notify_cb>,
    pub notify_payload: *mut c_void,
    pub progress_cb: Option<git_checkout_progress_cb>,
    pub progress_payload: *mut c_void,
    pub paths: git_strarray,
    pub baseline: *mut git_tree,
    pub baseline_index: *mut git_index,
    pub target_directory: *const c_char,
    pub ancestor_label: *const c_char,
    pub our_label: *const c_char,
    pub their_label: *const c_char,
    pub perfdata_cb: Option<git_checkout_perfdata_cb>,
    pub perfdata_payload: *mut c_void,
}

pub type git_checkout_notify_cb = extern fn(git_checkout_notify_t,
                                            *const c_char,
                                            *const git_diff_file,
                                            *const git_diff_file,
                                            *const git_diff_file,
                                            *mut c_void) -> c_int;
pub type git_checkout_progress_cb = extern fn(*const c_char,
                                              size_t,
                                              size_t,
                                              *mut c_void);

pub type git_checkout_perfdata_cb = extern fn(*const git_checkout_perfdata,
                                              *mut c_void);

#[repr(C)]
pub struct git_checkout_perfdata {
    pub mkdir_calls: size_t,
    pub stat_calls: size_t,
    pub chmod_calls: size_t,
}

#[repr(C)]
pub struct git_remote_callbacks {
    pub version: c_uint,
    pub sideband_progress: Option<git_transport_message_cb>,
    pub completion: Option<extern fn(git_remote_completion_type,
                                     *mut c_void) -> c_int>,
    pub credentials: Option<git_cred_acquire_cb>,
    pub certificate_check: Option<git_transport_certificate_check_cb>,
    pub transfer_progress: Option<git_transfer_progress_cb>,
    pub update_tips: Option<extern fn(*const c_char,
                                      *const git_oid,
                                      *const git_oid,
                                      *mut c_void) -> c_int>,
    pub pack_progress: Option<git_packbuilder_progress>,
    pub push_transfer_progress: Option<git_push_transfer_progress>,
    pub push_update_reference: Option<extern fn(*const c_char,
                                                *const c_char,
                                                *mut c_void) -> c_int>,
    pub push_negotiation: Option<git_push_negotiation>,
    pub transport: Option<git_transport_cb>,
    pub payload: *mut c_void,
}

#[repr(C)]
pub struct git_fetch_options {
    pub version: c_int,
    pub callbacks: git_remote_callbacks,
    pub prune: git_fetch_prune_t,
    pub update_fetchhead: c_int,
    pub download_tags: git_remote_autotag_option_t,
    pub proxy_opts: git_proxy_options,
    pub custom_headers: git_strarray,
}

git_enum! {
    pub enum git_remote_autotag_option_t {
        GIT_REMOTE_DOWNLOAD_TAGS_UNSPECIFIED,
        GIT_REMOTE_DOWNLOAD_TAGS_AUTO,
        GIT_REMOTE_DOWNLOAD_TAGS_NONE,
        GIT_REMOTE_DOWNLOAD_TAGS_ALL,
    }
}

git_enum! {
    pub enum git_fetch_prune_t {
        GIT_FETCH_PRUNE_UNSPECIFIED,
        GIT_FETCH_PRUNE,
        GIT_FETCH_NO_PRUNE,
    }
}

git_enum! {
    pub enum git_remote_completion_type {
        GIT_REMOTE_COMPLETION_DOWNLOAD,
        GIT_REMOTE_COMPLETION_INDEXING,
        GIT_REMOTE_COMPLETION_ERROR,
    }
}

pub type git_transport_message_cb = extern fn(*const c_char, c_int,
                                              *mut c_void) -> c_int;
pub type git_cred_acquire_cb = extern fn(*mut *mut git_cred,
                                         *const c_char, *const c_char,
                                         c_uint, *mut c_void) -> c_int;
pub type git_transfer_progress_cb = extern fn(*const git_transfer_progress,
                                              *mut c_void) -> c_int;
pub type git_packbuilder_progress = extern fn(git_packbuilder_stage_t, c_uint,
                                              c_uint, *mut c_void) -> c_int;
pub type git_push_transfer_progress = extern fn(c_uint, c_uint, size_t,
                                                *mut c_void) -> c_int;
pub type git_transport_certificate_check_cb = extern fn(*mut git_cert,
                                                        c_int,
                                                        *const c_char,
                                                        *mut c_void) -> c_int;
pub type git_push_negotiation = extern fn(*mut *const git_push_update,
                                          size_t,
                                          *mut c_void) -> c_int;

#[repr(C)]
pub struct git_push_update {
    pub src_refname: *mut c_char,
    pub dst_refname: *mut c_char,
    pub src: git_oid,
    pub dst: git_oid,
}

git_enum! {
    pub enum git_cert_t {
        GIT_CERT_NONE,
        GIT_CERT_X509,
        GIT_CERT_HOSTKEY_LIBSSH2,
    }
}

#[repr(C)]
pub struct git_cert {
    pub cert_type: git_cert_t,
}

#[repr(C)]
pub struct git_cert_hostkey {
    pub parent: git_cert,
    pub kind: git_cert_ssh_t,
    pub hash_md5: [u8; 16],
    pub hash_sha1: [u8; 20],
}

#[repr(C)]
pub struct git_cert_x509 {
    pub parent: git_cert,
    pub data: *mut c_void,
    pub len: size_t,
}

git_enum! {
    pub enum git_cert_ssh_t {
        GIT_CERT_SSH_MD5 = 1 << 0,
        GIT_CERT_SSH_SHA1 = 1 << 1,
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct git_transfer_progress {
    pub total_objects: c_uint,
    pub indexed_objects: c_uint,
    pub received_objects: c_uint,
    pub local_objects: c_uint,
    pub total_deltas: c_uint,
    pub indexed_deltas: c_uint,
    pub received_bytes: size_t,
}

#[repr(C)]
pub struct git_diff_file {
    pub id: git_oid,
    pub path: *const c_char,
    pub size: git_off_t,
    pub flags: u32,
    pub mode: u16,
}

pub type git_repository_create_cb = extern fn(*mut *mut git_repository,
                                              *const c_char,
                                              c_int, *mut c_void) -> c_int;
pub type git_remote_create_cb = extern fn(*mut *mut git_remote,
                                          *mut git_repository,
                                          *const c_char,
                                          *const c_char,
                                          *mut c_void) -> c_int;

git_enum! {
    pub enum git_checkout_notify_t {
        GIT_CHECKOUT_NOTIFY_NONE = 0,
        GIT_CHECKOUT_NOTIFY_CONFLICT = (1 << 0),
        GIT_CHECKOUT_NOTIFY_DIRTY = (1 << 1),
        GIT_CHECKOUT_NOTIFY_UPDATED = (1 << 2),
        GIT_CHECKOUT_NOTIFY_UNTRACKED = (1 << 3),
        GIT_CHECKOUT_NOTIFY_IGNORED = (1 << 4),

        GIT_CHECKOUT_NOTIFY_ALL = 0x0FFFF,
    }
}

git_enum! {
    pub enum git_status_t {
        GIT_STATUS_CURRENT = 0,

        GIT_STATUS_INDEX_NEW = (1 << 0),
        GIT_STATUS_INDEX_MODIFIED = (1 << 1),
        GIT_STATUS_INDEX_DELETED = (1 << 2),
        GIT_STATUS_INDEX_RENAMED = (1 << 3),
        GIT_STATUS_INDEX_TYPECHANGE = (1 << 4),

        GIT_STATUS_WT_NEW = (1 << 7),
        GIT_STATUS_WT_MODIFIED = (1 << 8),
        GIT_STATUS_WT_DELETED = (1 << 9),
        GIT_STATUS_WT_TYPECHANGE = (1 << 10),
        GIT_STATUS_WT_RENAMED = (1 << 11),
        GIT_STATUS_WT_UNREADABLE = (1 << 12),

        GIT_STATUS_IGNORED = (1 << 14),
        GIT_STATUS_CONFLICTED = (1 << 15),
    }
}

git_enum! {
    pub enum git_status_opt_t {
        GIT_STATUS_OPT_INCLUDE_UNTRACKED                = (1 << 0),
        GIT_STATUS_OPT_INCLUDE_IGNORED                  = (1 << 1),
        GIT_STATUS_OPT_INCLUDE_UNMODIFIED               = (1 << 2),
        GIT_STATUS_OPT_EXCLUDE_SUBMODULES               = (1 << 3),
        GIT_STATUS_OPT_RECURSE_UNTRACKED_DIRS           = (1 << 4),
        GIT_STATUS_OPT_DISABLE_PATHSPEC_MATCH           = (1 << 5),
        GIT_STATUS_OPT_RECURSE_IGNORED_DIRS             = (1 << 6),
        GIT_STATUS_OPT_RENAMES_HEAD_TO_INDEX            = (1 << 7),
        GIT_STATUS_OPT_RENAMES_INDEX_TO_WORKDIR         = (1 << 8),
        GIT_STATUS_OPT_SORT_CASE_SENSITIVELY            = (1 << 9),
        GIT_STATUS_OPT_SORT_CASE_INSENSITIVELY          = (1 << 10),

        GIT_STATUS_OPT_RENAMES_FROM_REWRITES            = (1 << 11),
        GIT_STATUS_OPT_NO_REFRESH                       = (1 << 12),
        GIT_STATUS_OPT_UPDATE_INDEX                     = (1 << 13),
        GIT_STATUS_OPT_INCLUDE_UNREADABLE               = (1 << 14),
        GIT_STATUS_OPT_INCLUDE_UNREADABLE_AS_UNTRACKED  = (1 << 15),
    }
}

git_enum! {
    pub enum git_status_show_t {
        GIT_STATUS_SHOW_INDEX_AND_WORKDIR = 0,
        GIT_STATUS_SHOW_INDEX_ONLY = 1,
        GIT_STATUS_SHOW_WORKDIR_ONLY = 2,
    }
}

git_enum! {
    pub enum git_delta_t {
        GIT_DELTA_UNMODIFIED,
        GIT_DELTA_ADDED,
        GIT_DELTA_DELETED,
        GIT_DELTA_MODIFIED,
        GIT_DELTA_RENAMED,
        GIT_DELTA_COPIED,
        GIT_DELTA_IGNORED,
        GIT_DELTA_UNTRACKED,
        GIT_DELTA_TYPECHANGE,
        GIT_DELTA_UNREADABLE,
        GIT_DELTA_CONFLICTED,
    }
}

#[repr(C)]
pub struct git_status_options {
    pub version: c_uint,
    pub show: git_status_show_t,
    pub flags: c_uint,
    pub pathspec: git_strarray,
}

#[repr(C)]
pub struct git_diff_delta {
    pub status: git_delta_t,
    pub flags: u32,
    pub similarity: u16,
    pub nfiles: u16,
    pub old_file: git_diff_file,
    pub new_file: git_diff_file,
}

#[repr(C)]
pub struct git_status_entry {
    pub status: git_status_t,
    pub head_to_index: *mut git_diff_delta,
    pub index_to_workdir: *mut git_diff_delta
}

git_enum! {
    pub enum git_checkout_strategy_t {
        GIT_CHECKOUT_NONE = 0,
        GIT_CHECKOUT_SAFE = (1 << 0),
        GIT_CHECKOUT_FORCE = (1 << 1),
        GIT_CHECKOUT_RECREATE_MISSING = (1 << 2),
        GIT_CHECKOUT_ALLOW_CONFLICTS = (1 << 4),
        GIT_CHECKOUT_REMOVE_UNTRACKED = (1 << 5),
        GIT_CHECKOUT_REMOVE_IGNORED = (1 << 6),
        GIT_CHECKOUT_UPDATE_ONLY = (1 << 7),
        GIT_CHECKOUT_DONT_UPDATE_INDEX = (1 << 8),
        GIT_CHECKOUT_NO_REFRESH = (1 << 9),
        GIT_CHECKOUT_SKIP_UNMERGED = (1 << 10),
        GIT_CHECKOUT_USE_OURS = (1 << 11),
        GIT_CHECKOUT_USE_THEIRS = (1 << 12),
        GIT_CHECKOUT_DISABLE_PATHSPEC_MATCH = (1 << 13),
        GIT_CHECKOUT_SKIP_LOCKED_DIRECTORIES = (1 << 18),
        GIT_CHECKOUT_DONT_OVERWRITE_IGNORED = (1 << 19),
        GIT_CHECKOUT_CONFLICT_STYLE_MERGE = (1 << 20),
        GIT_CHECKOUT_CONFLICT_STYLE_DIFF3 = (1 << 21),

        GIT_CHECKOUT_UPDATE_SUBMODULES = (1 << 16),
        GIT_CHECKOUT_UPDATE_SUBMODULES_IF_CHANGED = (1 << 17),
    }
}

git_enum! {
    pub enum git_reset_t {
        GIT_RESET_SOFT = 1,
        GIT_RESET_MIXED = 2,
        GIT_RESET_HARD = 3,
    }
}

git_enum! {
    pub enum git_otype: c_int {
        GIT_OBJ_ANY = -2,
        GIT_OBJ_BAD = -1,
        GIT_OBJ__EXT1 = 0,
        GIT_OBJ_COMMIT = 1,
        GIT_OBJ_TREE = 2,
        GIT_OBJ_BLOB = 3,
        GIT_OBJ_TAG = 4,
        GIT_OBJ__EXT2 = 5,
        GIT_OBJ_OFS_DELTA = 6,
        GIT_OBJ_REF_DELTA = 7,
    }
}

git_enum! {
    pub enum git_ref_t {
        GIT_REF_INVALID = 0,
        GIT_REF_OID = 1,
        GIT_REF_SYMBOLIC = 2,
        GIT_REF_LISTALL = GIT_REF_OID | GIT_REF_SYMBOLIC,
    }
}

git_enum! {
    pub enum git_filemode_t {
        GIT_FILEMODE_UNREADABLE = 0o000000,
        GIT_FILEMODE_TREE = 0o040000,
        GIT_FILEMODE_BLOB = 0o100644,
        GIT_FILEMODE_BLOB_EXECUTABLE = 0o100755,
        GIT_FILEMODE_LINK = 0o120000,
        GIT_FILEMODE_COMMIT = 0o160000,
    }
}

git_enum! {
    pub enum git_treewalk_mode {
        GIT_TREEWALK_PRE = 0,
        GIT_TREEWALK_POST = 1,
    }
}

pub type git_treewalk_cb = extern fn(*const c_char, *const git_tree_entry,
                                     *mut c_void) -> c_int;
pub type git_treebuilder_filter_cb = extern fn(*const git_tree_entry,
                                               *mut c_void) -> c_int;

pub type git_revwalk_hide_cb = extern fn(*const git_oid, *mut c_void) -> c_int;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct git_buf {
    pub ptr: *mut c_char,
    pub asize: size_t,
    pub size: size_t,
}

git_enum! {
    pub enum git_branch_t {
        GIT_BRANCH_LOCAL = 1,
        GIT_BRANCH_REMOTE = 2,
        GIT_BRANCH_ALL = GIT_BRANCH_LOCAL | GIT_BRANCH_REMOTE,
    }
}

pub const GIT_BLAME_NORMAL: u32 = 0;
pub const GIT_BLAME_TRACK_COPIES_SAME_FILE: u32 = 1<<0;
pub const GIT_BLAME_TRACK_COPIES_SAME_COMMIT_MOVES: u32 = 1<<1;
pub const GIT_BLAME_TRACK_COPIES_SAME_COMMIT_COPIES: u32 = 1<<2;
pub const GIT_BLAME_TRACK_COPIES_ANY_COMMIT_COPIES: u32 = 1<<3;
pub const GIT_BLAME_FIRST_PARENT: u32 = 1<<4;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct git_blame_options {
    pub version: c_uint,

    pub flags: u32,
    pub min_match_characters: u16,
    pub newest_commit: git_oid,
    pub oldest_commit: git_oid,
    pub min_line: usize,
    pub max_line: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct git_blame_hunk {
    pub lines_in_hunk: usize,
    pub final_commit_id: git_oid,
    pub final_start_line_number: usize,
    pub final_signature: *mut git_signature,
    pub orig_commit_id: git_oid,
    pub orig_path: *const c_char,
    pub orig_start_line_number: usize,
    pub orig_signature: *mut git_signature,
    pub boundary: c_char,
}

pub type git_index_matched_path_cb = extern fn(*const c_char, *const c_char,
                                               *mut c_void) -> c_int;

git_enum! {
    pub enum git_idxentry_extended_flag_t {
        GIT_IDXENTRY_INTENT_TO_ADD     = 1 << 13,
        GIT_IDXENTRY_SKIP_WORKTREE     = 1 << 14,
        GIT_IDXENTRY_EXTENDED2         = 1 << 15,

        GIT_IDXENTRY_UPDATE            = 1 << 0,
        GIT_IDXENTRY_REMOVE            = 1 << 1,
        GIT_IDXENTRY_UPTODATE          = 1 << 2,
        GIT_IDXENTRY_ADDED             = 1 << 3,

        GIT_IDXENTRY_HASHED            = 1 << 4,
        GIT_IDXENTRY_UNHASHED          = 1 << 5,
        GIT_IDXENTRY_WT_REMOVE         = 1 << 6,
        GIT_IDXENTRY_CONFLICTED        = 1 << 7,

        GIT_IDXENTRY_UNPACKED          = 1 << 8,
        GIT_IDXENTRY_NEW_SKIP_WORKTREE = 1 << 9,
    }
}

git_enum! {
    pub enum git_indxentry_flag_t {
        GIT_IDXENTRY_EXTENDED = 0x4000,
        GIT_IDXENTRY_VALID    = 0x8000,
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct git_index_entry {
    pub ctime: git_index_time,
    pub mtime: git_index_time,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub id: git_oid,
    pub flags: u16,
    pub flags_extended: u16,
    pub path: *const c_char,
}

pub const GIT_IDXENTRY_NAMEMASK: u16 = 0xfff;
pub const GIT_IDXENTRY_STAGEMASK: u16 = 0x3000;
pub const GIT_IDXENTRY_STAGESHIFT: u16 = 12;

#[repr(C)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub struct git_index_time {
    pub seconds: i32,
    pub nanoseconds: u32,
}

#[repr(C)]
pub struct git_config_entry {
    pub name: *const c_char,
    pub value: *const c_char,
    pub level: git_config_level_t,
    pub free: extern fn(*mut git_config_entry),
    pub payload: *mut c_void,
}

git_enum! {
    pub enum git_config_level_t: c_int {
        GIT_CONFIG_LEVEL_PROGRAMDATA = 1,
        GIT_CONFIG_LEVEL_SYSTEM = 2,
        GIT_CONFIG_LEVEL_XDG = 3,
        GIT_CONFIG_LEVEL_GLOBAL = 4,
        GIT_CONFIG_LEVEL_LOCAL = 5,
        GIT_CONFIG_LEVEL_APP = 6,
        GIT_CONFIG_HIGHEST_LEVEL = -1,
    }
}

git_enum! {
    pub enum git_submodule_update_t {
        GIT_SUBMODULE_UPDATE_CHECKOUT = 1,
        GIT_SUBMODULE_UPDATE_REBASE   = 2,
        GIT_SUBMODULE_UPDATE_MERGE    = 3,
        GIT_SUBMODULE_UPDATE_NONE     = 4,
        GIT_SUBMODULE_UPDATE_DEFAULT  = 0,
    }
}

git_enum! {
    pub enum git_submodule_ignore_t: c_int {
        GIT_SUBMODULE_IGNORE_UNSPECIFIED = -1,

        GIT_SUBMODULE_IGNORE_NONE      = 1,
        GIT_SUBMODULE_IGNORE_UNTRACKED = 2,
        GIT_SUBMODULE_IGNORE_DIRTY     = 3,
        GIT_SUBMODULE_IGNORE_ALL       = 4,
    }
}

pub type git_submodule_cb = extern fn(*mut git_submodule,
                                      *const c_char,
                                      *mut c_void) -> c_int;

#[repr(C)]
pub struct git_submodule_update_options {
    pub version: c_uint,
    pub checkout_opts: git_checkout_options,
    pub fetch_opts: git_fetch_options,
    pub allow_fetch: c_int,
}

#[repr(C)]
pub struct git_writestream {
    pub write: extern fn(*mut git_writestream,
                         *const c_char,
                         size_t) -> c_int,
    pub close: extern fn(*mut git_writestream) -> c_int,
    pub free: extern fn(*mut git_writestream),
}

#[repr(C)]
pub struct git_cred {
    pub credtype: git_credtype_t,
    pub free: extern fn(*mut git_cred),
}

git_enum! {
    pub enum git_credtype_t {
        GIT_CREDTYPE_USERPASS_PLAINTEXT = 1 << 0,
        GIT_CREDTYPE_SSH_KEY = 1 << 1,
        GIT_CREDTYPE_SSH_CUSTOM = 1 << 2,
        GIT_CREDTYPE_DEFAULT = 1 << 3,
        GIT_CREDTYPE_SSH_INTERACTIVE = 1 << 4,
        GIT_CREDTYPE_USERNAME = 1 << 5,
        GIT_CREDTYPE_SSH_MEMORY = 1 << 6,
    }
}

pub type git_cred_ssh_interactive_callback = extern fn(
    name: *const c_char,
    name_len: c_int,
    instruction: *const c_char,
    instruction_len: c_int,
    num_prompts: c_int,
    prompts: *const LIBSSH2_USERAUTH_KBDINT_PROMPT,
    responses: *mut LIBSSH2_USERAUTH_KBDINT_RESPONSE,
    abstrakt: *mut *mut c_void
);

pub type git_cred_sign_callback = extern fn(
    session: *mut LIBSSH2_SESSION,
    sig: *mut *mut c_uchar,
    sig_len: *mut size_t,
    data: *const c_uchar,
    data_len: size_t,
    abstrakt: *mut *mut c_void,
);

pub enum LIBSSH2_SESSION {}
pub enum LIBSSH2_USERAUTH_KBDINT_PROMPT {}
pub enum LIBSSH2_USERAUTH_KBDINT_RESPONSE {}

#[repr(C)]
pub struct git_push_options {
    pub version: c_uint,
    pub pb_parallelism: c_uint,
    pub callbacks: git_remote_callbacks,
    pub proxy_opts: git_proxy_options,
    pub custom_headers: git_strarray,
}

pub type git_tag_foreach_cb = extern fn(name: *const c_char,
                                        oid: *mut git_oid,
                                        payload: *mut c_void) -> c_int;

git_enum! {
    pub enum git_index_add_option_t {
        GIT_INDEX_ADD_DEFAULT = 0,
        GIT_INDEX_ADD_FORCE = 1 << 0,
        GIT_INDEX_ADD_DISABLE_PATHSPEC_MATCH = 1 << 1,
        GIT_INDEX_ADD_CHECK_PATHSPEC = 1 << 2,
    }
}

git_enum! {
    pub enum git_repository_open_flag_t {
        GIT_REPOSITORY_OPEN_NO_SEARCH = (1 << 0),
        GIT_REPOSITORY_OPEN_CROSS_FS = (1 << 1),
        GIT_REPOSITORY_OPEN_BARE = (1 << 2),
        GIT_REPOSITORY_OPEN_NO_DOTGIT = (1 << 3),
        GIT_REPOSITORY_OPEN_FROM_ENV = (1 << 4),
    }
}

#[repr(C)]
pub struct git_repository_init_options {
    pub version: c_uint,
    pub flags: u32,
    pub mode: u32,
    pub workdir_path: *const c_char,
    pub description: *const c_char,
    pub template_path: *const c_char,
    pub initial_head: *const c_char,
    pub origin_url: *const c_char,
}

pub const GIT_REPOSITORY_INIT_OPTIONS_VERSION: c_uint = 1;

git_enum! {
    pub enum git_repository_init_flag_t {
        GIT_REPOSITORY_INIT_BARE              = (1 << 0),
        GIT_REPOSITORY_INIT_NO_REINIT         = (1 << 1),
        GIT_REPOSITORY_INIT_NO_DOTGIT_DIR     = (1 << 2),
        GIT_REPOSITORY_INIT_MKDIR             = (1 << 3),
        GIT_REPOSITORY_INIT_MKPATH            = (1 << 4),
        GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE = (1 << 5),
    }
}

git_enum! {
    pub enum git_repository_init_mode_t {
        GIT_REPOSITORY_INIT_SHARED_UMASK = 0,
        GIT_REPOSITORY_INIT_SHARED_GROUP = 0o002775,
        GIT_REPOSITORY_INIT_SHARED_ALL   = 0o002777,
    }
}

git_enum! {
    pub enum git_sort_t {
        GIT_SORT_NONE        = 0,
        GIT_SORT_TOPOLOGICAL = (1 << 0),
        GIT_SORT_TIME        = (1 << 1),
        GIT_SORT_REVERSE     = (1 << 2),
    }
}

git_enum! {
    pub enum git_submodule_status_t {
        GIT_SUBMODULE_STATUS_IN_HEAD = 1 << 0,
        GIT_SUBMODULE_STATUS_IN_INDEX = 1 << 1,
        GIT_SUBMODULE_STATUS_IN_CONFIG = 1 << 2,
        GIT_SUBMODULE_STATUS_IN_WD = 1 << 3,
        GIT_SUBMODULE_STATUS_INDEX_ADDED = 1 << 4,
        GIT_SUBMODULE_STATUS_INDEX_DELETED = 1 << 5,
        GIT_SUBMODULE_STATUS_INDEX_MODIFIED = 1 << 6,
        GIT_SUBMODULE_STATUS_WD_UNINITIALIZED = 1 << 7,
        GIT_SUBMODULE_STATUS_WD_ADDED = 1 << 8,
        GIT_SUBMODULE_STATUS_WD_DELETED = 1 << 9,
        GIT_SUBMODULE_STATUS_WD_MODIFIED = 1 << 10,
        GIT_SUBMODULE_STATUS_WD_INDEX_MODIFIED = 1 << 11,
        GIT_SUBMODULE_STATUS_WD_WD_MODIFIED = 1 << 12,
        GIT_SUBMODULE_STATUS_WD_UNTRACKED = 1 << 13,
    }
}

#[repr(C)]
pub struct git_remote_head {
    pub local: c_int,
    pub oid: git_oid,
    pub loid: git_oid,
    pub name: *mut c_char,
    pub symref_target: *mut c_char,
}

git_enum! {
    pub enum git_pathspec_flag_t {
        GIT_PATHSPEC_DEFAULT = 0,
        GIT_PATHSPEC_IGNORE_CASE = 1 << 0,
        GIT_PATHSPEC_USE_CASE = 1 << 1,
        GIT_PATHSPEC_NO_GLOB = 1 << 2,
        GIT_PATHSPEC_NO_MATCH_ERROR = 1 << 3,
        GIT_PATHSPEC_FIND_FAILURES = 1 << 4,
        GIT_PATHSPEC_FAILURES_ONLY = 1 << 5,
    }
}

pub type git_diff_file_cb = extern fn(*const git_diff_delta, f32, *mut c_void)
                                      -> c_int;
pub type git_diff_hunk_cb = extern fn(*const git_diff_delta,
                                      *const git_diff_hunk,
                                      *mut c_void) -> c_int;
pub type git_diff_line_cb = extern fn(*const git_diff_delta,
                                      *const git_diff_hunk,
                                      *const git_diff_line,
                                      *mut c_void) -> c_int;
pub type git_diff_binary_cb = extern fn(*const git_diff_delta,
                                        *const git_diff_binary,
                                        *mut c_void) -> c_int;

#[repr(C)]
pub struct git_diff_hunk {
    pub old_start: c_int,
    pub old_lines: c_int,
    pub new_start: c_int,
    pub new_lines: c_int,
    pub header_len: size_t,
    pub header: [c_char; 128],
}

git_enum! {
    pub enum git_diff_line_t {
        GIT_DIFF_LINE_CONTEXT = b' ' as git_diff_line_t,
        GIT_DIFF_LINE_ADDITION = b'+' as git_diff_line_t,
        GIT_DIFF_LINE_DELETION = b'-' as git_diff_line_t,
        GIT_DIFF_LINE_CONTEXT_EOFNL = b'=' as git_diff_line_t,
        GIT_DIFF_LINE_ADD_EOFNL = b'>' as git_diff_line_t,
        GIT_DIFF_LINE_DEL_EOFNL = b'<' as git_diff_line_t,
        GIT_DIFF_LINE_FILE_HDR = b'F' as git_diff_line_t,
        GIT_DIFF_LINE_HUNK_HDR = b'H' as git_diff_line_t,
        GIT_DIFF_LINE_BINARY = b'B' as git_diff_line_t,
    }
}

#[repr(C)]
pub struct git_diff_line {
    pub origin: c_char,
    pub old_lineno: c_int,
    pub new_lineno: c_int,
    pub num_lines: c_int,
    pub content_len: size_t,
    pub content_offset: git_off_t,
    pub content: *const c_char,
}

#[repr(C)]
pub struct git_diff_options {
    pub version: c_uint,
    pub flags: u32,
    pub ignore_submodules: git_submodule_ignore_t,
    pub pathspec: git_strarray,
    pub notify_cb: git_diff_notify_cb,
    pub progress_cb: git_diff_progress_cb,
    pub payload: *mut c_void,
    pub context_lines: u32,
    pub interhunk_lines: u32,
    pub id_abbrev: u16,
    pub max_size: git_off_t,
    pub old_prefix: *const c_char,
    pub new_prefix: *const c_char,
}

git_enum! {
    pub enum git_diff_format_t {
        GIT_DIFF_FORMAT_PATCH = 1,
        GIT_DIFF_FORMAT_PATCH_HEADER = 2,
        GIT_DIFF_FORMAT_RAW = 3,
        GIT_DIFF_FORMAT_NAME_ONLY = 4,
        GIT_DIFF_FORMAT_NAME_STATUS = 5,
    }
}

git_enum! {
    pub enum git_diff_stats_format_t {
        GIT_DIFF_STATS_NONE = 0,
        GIT_DIFF_STATS_FULL = 1 << 0,
        GIT_DIFF_STATS_SHORT = 1 << 1,
        GIT_DIFF_STATS_NUMBER = 1 << 2,
        GIT_DIFF_STATS_INCLUDE_SUMMARY = 1 << 3,
    }
}

pub type git_diff_notify_cb = extern fn(*const git_diff,
                                        *const git_diff_delta,
                                        *const c_char,
                                        *mut c_void) -> c_int;

pub type git_diff_progress_cb = extern fn (*const git_diff,
                                           *const c_char,
                                           *const c_char,
                                           *mut c_void) -> c_int;

pub const GIT_DIFF_NORMAL: u32 = 0;
pub const GIT_DIFF_REVERSE: u32 = 1 << 0;
pub const GIT_DIFF_INCLUDE_IGNORED: u32 = 1 << 1;
pub const GIT_DIFF_RECURSE_IGNORED_DIRS: u32 = 1 << 2;
pub const GIT_DIFF_INCLUDE_UNTRACKED: u32 = 1 << 3;
pub const GIT_DIFF_RECURSE_UNTRACKED_DIRS: u32 = 1 << 4;
pub const GIT_DIFF_INCLUDE_UNMODIFIED: u32 = 1 << 5;
pub const GIT_DIFF_INCLUDE_TYPECHANGE: u32 = 1 << 6;
pub const GIT_DIFF_INCLUDE_TYPECHANGE_TREES: u32 = 1 << 7;
pub const GIT_DIFF_IGNORE_FILEMODE: u32 = 1 << 8;
pub const GIT_DIFF_IGNORE_SUBMODULES: u32 = 1 << 9;
pub const GIT_DIFF_IGNORE_CASE: u32 = 1 << 10;
pub const GIT_DIFF_DISABLE_PATHSPEC_MATCH: u32 = 1 << 12;
pub const GIT_DIFF_SKIP_BINARY_CHECK: u32 = 1 << 13;
pub const GIT_DIFF_ENABLE_FAST_UNTRACKED_DIRS: u32 = 1 << 14;
pub const GIT_DIFF_UPDATE_INDEX: u32 = 1 << 15;
pub const GIT_DIFF_INCLUDE_UNREADABLE: u32 = 1 << 16;
pub const GIT_DIFF_INCLUDE_UNREADABLE_AS_UNTRACKED: u32 = 1 << 17;
pub const GIT_DIFF_FORCE_TEXT: u32 = 1 << 20;
pub const GIT_DIFF_FORCE_BINARY: u32 = 1 << 21;
pub const GIT_DIFF_IGNORE_WHITESPACE: u32 = 1 << 22;
pub const GIT_DIFF_IGNORE_WHITESPACE_CHANGE: u32 = 1 << 23;
pub const GIT_DIFF_IGNORE_WHITESPACE_EOL: u32 = 1 << 24;
pub const GIT_DIFF_SHOW_UNTRACKED_CONTENT: u32 = 1 << 25;
pub const GIT_DIFF_SHOW_UNMODIFIED: u32 = 1 << 26;
pub const GIT_DIFF_PATIENCE: u32 = 1 << 28;
pub const GIT_DIFF_MINIMAL: u32 = 1 << 29;
pub const GIT_DIFF_SHOW_BINARY: u32 = 1 << 30;

#[repr(C)]
pub struct git_diff_find_options {
    pub version: c_uint,
    pub flags: u32,
    pub rename_threshold: u16,
    pub rename_from_rewrite_threshold: u16,
    pub copy_threshold: u16,
    pub break_rewrite_threshold: u16,
    pub rename_limit: size_t,
    pub metric: *mut git_diff_similarity_metric,
}

#[repr(C)]
pub struct git_diff_similarity_metric {
    pub file_signature: extern fn(*mut *mut c_void,
                                  *const git_diff_file,
                                  *const c_char,
                                  *mut c_void) -> c_int,
    pub buffer_signature: extern fn(*mut *mut c_void,
                                    *const git_diff_file,
                                    *const c_char,
                                    size_t,
                                    *mut c_void) -> c_int,
    pub free_signature: extern fn(*mut c_void, *mut c_void),
    pub similarity: extern fn(*mut c_int, *mut c_void, *mut c_void,
                              *mut c_void) -> c_int,
    pub payload: *mut c_void,
}

pub const GIT_DIFF_FIND_OPTIONS_VERSION: c_uint = 1;

pub const GIT_DIFF_FIND_BY_CONFIG: u32 = 0;
pub const GIT_DIFF_FIND_RENAMES: u32 = 1 << 0;
pub const GIT_DIFF_FIND_RENAMES_FROM_REWRITES: u32 = 1 << 1;
pub const GIT_DIFF_FIND_COPIES: u32 = 1 << 2;
pub const GIT_DIFF_FIND_COPIES_FROM_UNMODIFIED: u32 = 1 << 3;
pub const GIT_DIFF_FIND_REWRITES: u32 = 1 << 4;
pub const GIT_DIFF_BREAK_REWRITES: u32 = 1 << 5;
pub const GIT_DIFF_FIND_AND_BREAK_REWRITES: u32 =
        GIT_DIFF_FIND_REWRITES | GIT_DIFF_BREAK_REWRITES;
pub const GIT_DIFF_FIND_FOR_UNTRACKED: u32 = 1 << 6;
pub const GIT_DIFF_FIND_ALL: u32 = 0x0ff;
pub const GIT_DIFF_FIND_IGNORE_LEADING_WHITESPACE: u32 = 0;
pub const GIT_DIFF_FIND_IGNORE_WHITESPACE: u32 = 1 << 12;
pub const GIT_DIFF_FIND_DONT_IGNORE_WHITESPACE: u32 = 1 << 13;
pub const GIT_DIFF_FIND_EXACT_MATCH_ONLY: u32 = 1 << 14;
pub const GIT_DIFF_BREAK_REWRITES_FOR_RENAMES_ONLY : u32 = 1 << 15;
pub const GIT_DIFF_FIND_REMOVE_UNMODIFIED: u32 = 1 << 16;

#[repr(C)]
pub struct git_diff_binary {
    pub contains_data: c_uint,
    pub old_file: git_diff_binary_file,
    pub new_file: git_diff_binary_file,
}

#[repr(C)]
pub struct git_diff_binary_file {
    pub kind: git_diff_binary_t,
    pub data: *const c_char,
    pub datalen: size_t,
    pub inflatedlen: size_t,
}

git_enum! {
    pub enum git_diff_binary_t {
        GIT_DIFF_BINARY_NONE,
        GIT_DIFF_BINARY_LITERAL,
        GIT_DIFF_BINARY_DELTA,
    }
}

#[repr(C)]
pub struct git_merge_options {
    pub version: c_uint,
    pub flags: git_merge_flag_t,
    pub rename_threshold: c_uint,
    pub target_limit: c_uint,
    pub metric: *mut git_diff_similarity_metric,
    pub recursion_limit: c_uint,
    pub default_driver: *const c_char,
    pub file_favor: git_merge_file_favor_t,
    pub file_flags: git_merge_file_flag_t,
}

git_enum! {
    pub enum git_merge_flag_t {
        GIT_MERGE_FIND_RENAMES = 1 << 0,
        GIT_MERGE_FAIL_ON_CONFLICT = 1 << 1,
        GIT_MERGE_SKIP_REUC = 1 << 2,
        GIT_MERGE_NO_RECURSIVE = 1 << 3,
    }
}

git_enum! {
    pub enum git_merge_file_favor_t {
        GIT_MERGE_FILE_FAVOR_NORMAL = 0,
        GIT_MERGE_FILE_FAVOR_OURS = 1,
        GIT_MERGE_FILE_FAVOR_THEIRS = 2,
        GIT_MERGE_FILE_FAVOR_UNION = 3,
    }
}

git_enum! {
    pub enum git_merge_file_flag_t {
        GIT_MERGE_FILE_DEFAULT = 0,
        GIT_MERGE_FILE_STYLE_MERGE = (1 << 0),
        GIT_MERGE_FILE_STYLE_DIFF3 = (1 << 1),
        GIT_MERGE_FILE_SIMPLIFY_ALNUM = (1 << 2),
        GIT_MERGE_FILE_IGNORE_WHITESPACE = (1 << 3),
        GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE = (1 << 4),
        GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL = (1 << 5),
        GIT_MERGE_FILE_DIFF_PATIENCE = (1 << 6),
        GIT_MERGE_FILE_DIFF_MINIMAL = (1 << 7),
    }
}

git_enum! {
    pub enum git_merge_analysis_t {
        GIT_MERGE_ANALYSIS_NONE = 0,
        GIT_MERGE_ANALYSIS_NORMAL = (1 << 0),
        GIT_MERGE_ANALYSIS_UP_TO_DATE = (1 << 1),
        GIT_MERGE_ANALYSIS_FASTFORWARD = (1 << 2),
        GIT_MERGE_ANALYSIS_UNBORN = (1 << 3),
    }
}

git_enum! {
    pub enum git_merge_preference_t {
        GIT_MERGE_PREFERENCE_NONE = 0,
        GIT_MERGE_PREFERENCE_NO_FASTFORWARD = (1 << 0),
        GIT_MERGE_PREFERENCE_FASTFORWARD_ONLY = (1 << 1),
    }
}

pub type git_transport_cb = extern fn(out: *mut *mut git_transport,
                                      owner: *mut git_remote,
                                      param: *mut c_void) -> c_int;

#[repr(C)]
pub struct git_transport {
    pub version: c_uint,
    pub set_callbacks: extern fn(*mut git_transport,
                                 git_transport_message_cb,
                                 git_transport_message_cb,
                                 git_transport_certificate_check_cb,
                                 *mut c_void) -> c_int,
    pub set_custom_headers: extern fn(*mut git_transport,
                                      *const git_strarray) -> c_int,
    pub connect: extern fn(*mut git_transport,
                           *const c_char,
                           git_cred_acquire_cb,
                           *mut c_void,
                           *const git_proxy_options,
                           c_int, c_int) -> c_int,
    pub ls: extern fn(*mut *mut *const git_remote_head,
                      *mut size_t,
                      *mut git_transport) -> c_int,
    pub push: extern fn(*mut git_transport,
                        *mut git_push,
                        *const git_remote_callbacks) -> c_int,
    pub negotiate_fetch: extern fn(*mut git_transport,
                                   *mut git_repository,
                                   *const *const git_remote_head,
                                   size_t) -> c_int,
    pub download_pack: extern fn(*mut git_transport,
                                 *mut git_repository,
                                 *mut git_transfer_progress,
                                 git_transfer_progress_cb,
                                 *mut c_void) -> c_int,
    pub is_connected: extern fn(*mut git_transport) -> c_int,
    pub read_flags: extern fn(*mut git_transport, *mut c_int) -> c_int,
    pub cancel: extern fn(*mut git_transport),
    pub close: extern fn(*mut git_transport) -> c_int,
    pub free: extern fn(*mut git_transport),
}

#[repr(C)]
pub struct git_proxy_options {
    pub version: c_uint,
    pub kind: git_proxy_t,
    pub url: *const c_char,
    pub credentials: Option<git_cred_acquire_cb>,
    pub certificate_check: Option<git_transport_certificate_check_cb>,
    pub payload: *mut c_void,
}

git_enum! {
    pub enum git_proxy_t {
        GIT_PROXY_NONE = 0,
        GIT_PROXY_AUTO = 1,
        GIT_PROXY_SPECIFIED = 2,
    }
}

git_enum! {
    pub enum git_smart_service_t {
        GIT_SERVICE_UPLOADPACK_LS = 1,
        GIT_SERVICE_UPLOADPACK = 2,
        GIT_SERVICE_RECEIVEPACK_LS = 3,
        GIT_SERVICE_RECEIVEPACK = 4,
    }
}

#[repr(C)]
pub struct git_smart_subtransport_stream {
    pub subtransport: *mut git_smart_subtransport,
    pub read: extern fn(*mut git_smart_subtransport_stream,
                        *mut c_char,
                        size_t,
                        *mut size_t) -> c_int,
    pub write: extern fn(*mut git_smart_subtransport_stream,
                         *const c_char,
                         size_t) -> c_int,
    pub free: extern fn(*mut git_smart_subtransport_stream),
}

#[repr(C)]
pub struct git_smart_subtransport {
    pub action: extern fn(*mut *mut git_smart_subtransport_stream,
                          *mut git_smart_subtransport,
                          *const c_char,
                          git_smart_service_t) -> c_int,
    pub close: extern fn(*mut git_smart_subtransport) -> c_int,
    pub free: extern fn(*mut git_smart_subtransport),
}

pub type git_smart_subtransport_cb = extern fn(*mut *mut git_smart_subtransport,
                                               *mut git_transport,
                                               *mut c_void) -> c_int;

#[repr(C)]
pub struct git_smart_subtransport_definition {
    pub callback: git_smart_subtransport_cb,
    pub rpc: c_uint,
    pub param: *mut c_void,
}

#[repr(C)]
pub struct git_describe_options {
    pub version: c_uint,
    pub max_candidates_tags: c_uint,
    pub describe_strategy: c_uint,
    pub pattern: *const c_char,
    pub only_follow_first_parent: c_int,
    pub show_commit_oid_as_fallback: c_int,
}

git_enum! {
    pub enum git_describe_strategy_t {
        GIT_DESCRIBE_DEFAULT,
        GIT_DESCRIBE_TAGS,
        GIT_DESCRIBE_ALL,
    }
}

#[repr(C)]
pub struct git_describe_format_options {
    pub version: c_uint,
    pub abbreviated_size: c_uint,
    pub always_use_long_format: c_int,
    pub dirty_suffix: *const c_char,
}

git_enum! {
    pub enum git_packbuilder_stage_t {
        GIT_PACKBUILDER_ADDING_OBJECTS,
        GIT_PACKBUILDER_DELTAFICATION,
    }
}

git_enum! {
    pub enum git_stash_flags {
        GIT_STASH_DEFAULT = 0,
        GIT_STASH_KEEP_INDEX = 1 << 0,
        GIT_STASH_INCLUDE_UNTRACKED = 1 << 1,
        GIT_STASH_INCLUDE_IGNORED = 1 << 2,
    }
}

git_enum! {
    pub enum git_stash_apply_flags {
        GIT_STASH_APPLY_DEFAULT = 0,
        GIT_STASH_APPLY_REINSTATE_INDEX = 1 << 0,
    }
}

git_enum! {
    pub enum git_stash_apply_progress_t {
        GIT_STASH_APPLY_PROGRESS_NONE = 0,
        GIT_STASH_APPLY_PROGRESS_LOADING_STASH,
        GIT_STASH_APPLY_PROGRESS_ANALYZE_INDEX,
        GIT_STASH_APPLY_PROGRESS_ANALYZE_MODIFIED,
        GIT_STASH_APPLY_PROGRESS_ANALYZE_UNTRACKED,
        GIT_STASH_APPLY_PROGRESS_CHECKOUT_UNTRACKED,
        GIT_STASH_APPLY_PROGRESS_CHECKOUT_MODIFIED,
        GIT_STASH_APPLY_PROGRESS_DONE,
    }
}

#[repr(C)]
pub struct git_stash_apply_options {
    pub version: c_uint,
    pub flags: git_stash_apply_flags,
    pub checkout_options: git_checkout_options,
    pub progress_cb: git_stash_apply_progress_cb,
    pub progress_payload: *mut c_void,
}

pub type git_stash_apply_progress_cb = extern fn(progress: git_stash_apply_progress_t,
                                                 payload: *mut c_void) -> c_int;

pub type git_stash_cb = extern fn(index: size_t,
                                  message: *const c_char,
                                  stash_id: *const git_oid,
                                  payload: *mut c_void) -> c_int;

pub type git_packbuilder_foreach_cb = extern fn(*const c_void, size_t,
                                                *mut c_void) -> c_int;

pub type git_odb_foreach_cb = extern fn(id: *const git_oid, payload: *mut c_void) -> c_int;

extern {
    // threads
    pub fn git_libgit2_init() -> c_int;
    pub fn git_libgit2_shutdown() -> c_int;

    // repository
    pub fn git_repository_free(repo: *mut git_repository);
    pub fn git_repository_open(repo: *mut *mut git_repository,
                               path: *const c_char) -> c_int;
    pub fn git_repository_open_ext(repo: *mut *mut git_repository,
                                   path: *const c_char,
                                   flags: c_uint,
                                   ceiling_dirs: *const c_char) -> c_int;
    pub fn git_repository_init(repo: *mut *mut git_repository,
                               path: *const c_char,
                               is_bare: c_uint) -> c_int;
    pub fn git_repository_init_ext(out: *mut *mut git_repository,
                                   repo_path: *const c_char,
                                   opts: *mut git_repository_init_options)
                                   -> c_int;
    pub fn git_repository_init_init_options(opts: *mut git_repository_init_options,
                                            version: c_uint) -> c_int;
    pub fn git_repository_get_namespace(repo: *mut git_repository)
                                        -> *const c_char;
    pub fn git_repository_head(out: *mut *mut git_reference,
                               repo: *mut git_repository) -> c_int;
    pub fn git_repository_set_head(repo: *mut git_repository,
                                   refname: *const c_char) -> c_int;
    pub fn git_repository_set_head_detached(repo: *mut git_repository,
                                            commitish: *const git_oid) -> c_int;
    pub fn git_repository_is_bare(repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_empty(repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_shallow(repo: *mut git_repository) -> c_int;
    pub fn git_repository_path(repo: *mut git_repository) -> *const c_char;
    pub fn git_repository_state(repo: *mut git_repository) -> c_int;
    pub fn git_repository_workdir(repo: *mut git_repository) -> *const c_char;
    pub fn git_repository_set_workdir(repo: *mut git_repository,
                                      workdir: *const c_char,
                                      update_gitlink: c_int) -> c_int;
    pub fn git_repository_index(out: *mut *mut git_index,
                                repo: *mut git_repository) -> c_int;
    pub fn git_repository_set_index(repo: *mut git_repository,
                                    index: *mut git_index);
    pub fn git_repository_config(out: *mut *mut git_config,
                                 repo: *mut git_repository) -> c_int;
    pub fn git_repository_config_snapshot(out: *mut *mut git_config,
                                          repo: *mut git_repository) -> c_int;
    pub fn git_repository_discover(out: *mut git_buf,
                                   start_path: *const c_char,
                                   across_fs: c_int,
                                   ceiling_dirs: *const c_char) -> c_int;

    // revparse
    pub fn git_revparse(revspec: *mut git_revspec,
                        repo: *mut git_repository,
                        spec: *const c_char) -> c_int;
    pub fn git_revparse_single(out: *mut *mut git_object,
                               repo: *mut git_repository,
                               spec: *const c_char) -> c_int;
    pub fn git_revparse_ext(object_out: *mut *mut git_object,
                            reference_out: *mut *mut git_reference,
                            repo: *mut git_repository,
                            spec: *const c_char) -> c_int;

    // object
    pub fn git_object_dup(dest: *mut *mut git_object,
                          source: *mut git_object) -> c_int;
    pub fn git_object_id(obj: *const git_object) -> *const git_oid;
    pub fn git_object_free(object: *mut git_object);
    pub fn git_object_lookup(dest: *mut *mut git_object,
                             repo: *mut git_repository,
                             id: *const git_oid,
                             kind: git_otype) -> c_int;
    pub fn git_object_type(obj: *const git_object) -> git_otype;
    pub fn git_object_peel(peeled: *mut *mut git_object,
                           object: *const git_object,
                           target_type: git_otype) -> c_int;
    pub fn git_object_short_id(out: *mut git_buf,
                               obj: *const git_object) -> c_int;
    pub fn git_object_type2string(kind: git_otype) -> *const c_char;
    pub fn git_object_string2type(s: *const c_char) -> git_otype;
    pub fn git_object_typeisloose(kind: git_otype) -> c_int;

    // oid
    pub fn git_oid_fromraw(out: *mut git_oid, raw: *const c_uchar);
    pub fn git_oid_fromstrn(out: *mut git_oid, str: *const c_char,
                            len: size_t) -> c_int;
    pub fn git_oid_tostr(out: *mut c_char, n: size_t,
                         id: *const git_oid) -> *mut c_char;
    pub fn git_oid_cmp(a: *const git_oid, b: *const git_oid) -> c_int;
    pub fn git_oid_equal(a: *const git_oid, b: *const git_oid) -> c_int;
    pub fn git_oid_streq(id: *const git_oid, str: *const c_char) -> c_int;
    pub fn git_oid_iszero(id: *const git_oid) -> c_int;

    // giterr
    pub fn giterr_last() -> *const git_error;
    pub fn giterr_clear();
    pub fn giterr_set_str(error_class: c_int, string: *const c_char);

    // remote
    pub fn git_remote_create(out: *mut *mut git_remote,
                             repo: *mut git_repository,
                             name: *const c_char,
                             url: *const c_char) -> c_int;
    pub fn git_remote_lookup(out: *mut *mut git_remote,
                             repo: *mut git_repository,
                             name: *const c_char) -> c_int;
    pub fn git_remote_create_anonymous(out: *mut *mut git_remote,
                                       repo: *mut git_repository,
                                       url: *const c_char) -> c_int;
    pub fn git_remote_delete(repo: *mut git_repository,
                             name: *const c_char) -> c_int;
    pub fn git_remote_free(remote: *mut git_remote);
    pub fn git_remote_name(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_pushurl(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_refspec_count(remote: *const git_remote) -> size_t;
    pub fn git_remote_url(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_connect(remote: *mut git_remote,
                              dir: git_direction,
                              callbacks: *const git_remote_callbacks,
                              proxy_opts: *const git_proxy_options,
                              custom_headers: *const git_strarray) -> c_int;
    pub fn git_remote_connected(remote: *const git_remote) -> c_int;
    pub fn git_remote_disconnect(remote: *mut git_remote);
    pub fn git_remote_add_fetch(repo: *mut git_repository,
                                remote: *const c_char,
                                refspec: *const c_char) -> c_int;
    pub fn git_remote_add_push(repo: *mut git_repository,
                               remote: *const c_char,
                               refspec: *const c_char) -> c_int;
    pub fn git_remote_download(remote: *mut git_remote,
                               refspecs: *const git_strarray,
                               opts: *const git_fetch_options) -> c_int;
    pub fn git_remote_stop(remote: *mut git_remote);
    pub fn git_remote_dup(dest: *mut *mut git_remote,
                          source: *mut git_remote) -> c_int;
    pub fn git_remote_get_fetch_refspecs(array: *mut git_strarray,
                                         remote: *const git_remote) -> c_int;
    pub fn git_remote_get_push_refspecs(array: *mut git_strarray,
                                         remote: *const git_remote) -> c_int;
    pub fn git_remote_get_refspec(remote: *const git_remote,
                                  n: size_t) -> *const git_refspec;
    pub fn git_remote_is_valid_name(remote_name: *const c_char) -> c_int;
    pub fn git_remote_list(out: *mut git_strarray,
                           repo: *mut git_repository) -> c_int;
    pub fn git_remote_rename(problems: *mut git_strarray,
                             repo: *mut git_repository,
                             name: *const c_char,
                             new_name: *const c_char) -> c_int;
    pub fn git_remote_fetch(remote: *mut git_remote,
                            refspecs: *const git_strarray,
                            opts: *const git_fetch_options,
                            reflog_message: *const c_char) -> c_int;
    pub fn git_remote_push(remote: *mut git_remote,
                           refspecs: *const git_strarray,
                           opts: *const git_push_options) -> c_int;
    pub fn git_remote_update_tips(remote: *mut git_remote,
                                  callbacks: *const git_remote_callbacks,
                                  update_fetchead: c_int,
                                  download_tags: git_remote_autotag_option_t,
                                  reflog_message: *const c_char) -> c_int;
    pub fn git_remote_set_url(repo: *mut git_repository,
                              remote: *const c_char,
                              url: *const c_char) -> c_int;
    pub fn git_remote_set_pushurl(repo: *mut git_repository,
                                  remote: *const c_char,
                                  pushurl: *const c_char) -> c_int;
    pub fn git_remote_init_callbacks(opts: *mut git_remote_callbacks,
                                     version: c_uint) -> c_int;
    pub fn git_fetch_init_options(opts: *mut git_fetch_options,
                                  version: c_uint) -> c_int;
    pub fn git_remote_stats(remote: *mut git_remote)
                            -> *const git_transfer_progress;
    pub fn git_remote_ls(out: *mut *mut *const git_remote_head,
                         size: *mut size_t,
                         remote: *mut git_remote) -> c_int;
    pub fn git_remote_set_autotag(repo: *mut git_repository,
                                  remote: *const c_char,
                                  value: git_remote_autotag_option_t) -> c_int;
    pub fn git_remote_prune(remote: *mut git_remote,
                            callbacks: *const git_remote_callbacks) -> c_int;

    // refspec
    pub fn git_refspec_direction(spec: *const git_refspec) -> git_direction;
    pub fn git_refspec_dst(spec: *const git_refspec) -> *const c_char;
    pub fn git_refspec_dst_matches(spec: *const git_refspec,
                                   refname: *const c_char) -> c_int;
    pub fn git_refspec_src(spec: *const git_refspec) -> *const c_char;
    pub fn git_refspec_src_matches(spec: *const git_refspec,
                                   refname: *const c_char) -> c_int;
    pub fn git_refspec_force(spec: *const git_refspec) -> c_int;
    pub fn git_refspec_string(spec: *const git_refspec) -> *const c_char;

    // strarray
    pub fn git_strarray_free(array: *mut git_strarray);

    // oidarray
    pub fn git_oidarray_free(array: *mut git_oidarray);

    // signature
    pub fn git_signature_default(out: *mut *mut git_signature,
                                 repo: *mut git_repository) -> c_int;
    pub fn git_signature_free(sig: *mut git_signature);
    pub fn git_signature_new(out: *mut *mut git_signature,
                             name: *const c_char,
                             email: *const c_char,
                             time: git_time_t,
                             offset: c_int) -> c_int;
    pub fn git_signature_now(out: *mut *mut git_signature,
                             name: *const c_char,
                             email: *const c_char) -> c_int;
    pub fn git_signature_dup(dest: *mut *mut git_signature,
                             sig: *const git_signature) -> c_int;

    // status
    pub fn git_status_list_new(out: *mut *mut git_status_list,
                               repo: *mut git_repository,
                               options: *const git_status_options) -> c_int;
    pub fn git_status_list_entrycount(list: *mut git_status_list) -> size_t;
    pub fn git_status_byindex(statuslist: *mut git_status_list,
                              idx: size_t) -> *const git_status_entry;
    pub fn git_status_list_free(list: *mut git_status_list);
    pub fn git_status_init_options(opts: *mut git_status_options,
                                   version: c_uint) -> c_int;
    pub fn git_status_file(status_flags: *mut c_uint,
                           repo: *mut git_repository,
                           path: *const c_char) -> c_int;
    pub fn git_status_should_ignore(ignored: *mut c_int,
                                    repo: *mut git_repository,
                                    path: *const c_char) -> c_int;

    // clone
    pub fn git_clone(out: *mut *mut git_repository,
                     url: *const c_char,
                     local_path: *const c_char,
                     options: *const git_clone_options) -> c_int;
    pub fn git_clone_init_options(opts: *mut git_clone_options,
                                  version: c_uint) -> c_int;

    // reset
    pub fn git_reset(repo: *mut git_repository,
                     target: *mut git_object,
                     reset_type: git_reset_t,
                     checkout_opts: *const git_checkout_options) -> c_int;
    pub fn git_reset_default(repo: *mut git_repository,
                             target: *mut git_object,
                             pathspecs: *mut git_strarray) -> c_int;

    // reference
    pub fn git_reference_cmp(ref1: *const git_reference,
                             ref2: *const git_reference) -> c_int;
    pub fn git_reference_delete(r: *mut git_reference) -> c_int;
    pub fn git_reference_free(r: *mut git_reference);
    pub fn git_reference_is_branch(r: *const git_reference) -> c_int;
    pub fn git_reference_is_note(r: *const git_reference) -> c_int;
    pub fn git_reference_is_remote(r: *const git_reference) -> c_int;
    pub fn git_reference_is_tag(r: *const git_reference) -> c_int;
    pub fn git_reference_is_valid_name(name: *const c_char) -> c_int;
    pub fn git_reference_lookup(out: *mut *mut git_reference,
                                repo: *mut git_repository,
                                name: *const c_char) -> c_int;
    pub fn git_reference_name(r: *const git_reference) -> *const c_char;
    pub fn git_reference_name_to_id(out: *mut git_oid,
                                    repo: *mut git_repository,
                                    name: *const c_char) -> c_int;
    pub fn git_reference_peel(out: *mut *mut git_object,
                              r: *mut git_reference,
                              otype: git_otype) -> c_int;
    pub fn git_reference_rename(new_ref: *mut *mut git_reference,
                                r: *mut git_reference,
                                new_name: *const c_char,
                                force: c_int,
                                log_message: *const c_char) -> c_int;
    pub fn git_reference_resolve(out: *mut *mut git_reference,
                                 r: *const git_reference) -> c_int;
    pub fn git_reference_shorthand(r: *const git_reference) -> *const c_char;
    pub fn git_reference_symbolic_target(r: *const git_reference) -> *const c_char;
    pub fn git_reference_target(r: *const git_reference) -> *const git_oid;
    pub fn git_reference_target_peel(r: *const git_reference) -> *const git_oid;
    pub fn git_reference_set_target(out: *mut *mut git_reference,
                                    r: *mut git_reference,
                                    id: *const git_oid,
                                    log_message: *const c_char) -> c_int;
    pub fn git_reference_type(r: *const git_reference) -> git_ref_t;
    pub fn git_reference_iterator_new(out: *mut *mut git_reference_iterator,
                                      repo: *mut git_repository) -> c_int;
    pub fn git_reference_iterator_glob_new(out: *mut *mut git_reference_iterator,
                                           repo: *mut git_repository,
                                           glob: *const c_char) -> c_int;
    pub fn git_reference_iterator_free(iter: *mut git_reference_iterator);
    pub fn git_reference_next(out: *mut *mut git_reference,
                              iter: *mut git_reference_iterator) -> c_int;
    pub fn git_reference_next_name(out: *mut *const c_char,
                                   iter: *mut git_reference_iterator) -> c_int;
    pub fn git_reference_create(out: *mut *mut git_reference,
                                repo: *mut git_repository,
                                name: *const c_char,
                                id: *const git_oid,
                                force: c_int,
                                log_message: *const c_char) -> c_int;
    pub fn git_reference_symbolic_create(out: *mut *mut git_reference,
                                         repo: *mut git_repository,
                                         name: *const c_char,
                                         target: *const c_char,
                                         force: c_int,
                                         log_message: *const c_char) -> c_int;
    pub fn git_reference_create_matching(out: *mut *mut git_reference,
                                         repo: *mut git_repository,
                                         name: *const c_char,
                                         id: *const git_oid,
                                         force: c_int,
                                         current_id: *const git_oid,
                                         log_message: *const c_char) -> c_int;
    pub fn git_reference_symbolic_create_matching(out: *mut *mut git_reference,
                                                  repo: *mut git_repository,
                                                  name: *const c_char,
                                                  target: *const c_char,
                                                  force: c_int,
                                                  current_id: *const c_char,
                                                  log_message: *const c_char)
                                                  -> c_int;
    pub fn git_reference_has_log(repo: *mut git_repository,
                                 name: *const c_char) -> c_int;
    pub fn git_reference_ensure_log(repo: *mut git_repository,
                                    name: *const c_char) -> c_int;

    // stash
    pub fn git_stash_save(out: *mut git_oid,
                          repo: *mut git_repository,
                          stasher: *const git_signature,
                          message: *const c_char,
                          flags: c_uint) -> c_int;

    pub fn git_stash_apply_init_options(opts: *mut git_stash_apply_options,
                                        version: c_uint) -> c_int;

    pub fn git_stash_apply(repo: *mut git_repository,
                           index: size_t,
                           options: *const git_stash_apply_options) -> c_int;

    pub fn git_stash_foreach(repo: *mut git_repository,
                             callback: git_stash_cb,
                             payload: *mut c_void) -> c_int;

    pub fn git_stash_drop(repo: *mut git_repository,
                          index: size_t) -> c_int;

    pub fn git_stash_pop(repo: *mut git_repository,
                         index: size_t,
                         options: *const git_stash_apply_options) -> c_int;

    // submodules
    pub fn git_submodule_add_finalize(submodule: *mut git_submodule) -> c_int;
    pub fn git_submodule_add_setup(submodule: *mut *mut git_submodule,
                                   repo: *mut git_repository,
                                   url: *const c_char,
                                   path: *const c_char,
                                   use_gitlink: c_int) -> c_int;
    pub fn git_submodule_add_to_index(submodule: *mut git_submodule,
                                      write_index: c_int) -> c_int;
    pub fn git_submodule_branch(submodule: *mut git_submodule) -> *const c_char;
    pub fn git_submodule_foreach(repo: *mut git_repository,
                                 callback: git_submodule_cb,
                                 payload: *mut c_void) -> c_int;
    pub fn git_submodule_free(submodule: *mut git_submodule);
    pub fn git_submodule_head_id(submodule: *mut git_submodule) -> *const git_oid;
    pub fn git_submodule_index_id(submodule: *mut git_submodule) -> *const git_oid;
    pub fn git_submodule_init(submodule: *mut git_submodule,
                              overwrite: c_int) -> c_int;
    pub fn git_submodule_location(status: *mut c_uint,
                                  submodule: *mut git_submodule) -> c_int;
    pub fn git_submodule_lookup(out: *mut *mut git_submodule,
                                repo: *mut git_repository,
                                name: *const c_char) -> c_int;
    pub fn git_submodule_name(submodule: *mut git_submodule) -> *const c_char;
    pub fn git_submodule_open(repo: *mut *mut git_repository,
                              submodule: *mut git_submodule) -> c_int;
    pub fn git_submodule_path(submodule: *mut git_submodule) -> *const c_char;
    pub fn git_submodule_reload(submodule: *mut git_submodule,
                                force: c_int) -> c_int;
    pub fn git_submodule_set_ignore(repo: *mut git_repository,
                                    name: *const c_char,
                                    ignore: git_submodule_ignore_t)
                                    -> c_int;
    pub fn git_submodule_set_update(repo: *mut git_repository,
                                    name: *const c_char,
                                    update: git_submodule_update_t)
                                    -> c_int;
    pub fn git_submodule_set_url(repo: *mut git_repository,
                                 name: *const c_char,
                                 url: *const c_char) -> c_int;
    pub fn git_submodule_sync(submodule: *mut git_submodule) -> c_int;
    pub fn git_submodule_update_strategy(submodule: *mut git_submodule)
                                         -> git_submodule_update_t;
    pub fn git_submodule_update(submodule: *mut git_submodule,
                                init: c_int,
                                options: *mut git_submodule_update_options)
                                -> c_int;
    pub fn git_submodule_update_init_options(options: *mut git_submodule_update_options,
                                             version: c_uint) -> c_int;
    pub fn git_submodule_url(submodule: *mut git_submodule) -> *const c_char;
    pub fn git_submodule_wd_id(submodule: *mut git_submodule) -> *const git_oid;
    pub fn git_submodule_status(status: *mut c_uint,
                                repo: *mut git_repository,
                                name: *const c_char,
                                ignore: git_submodule_ignore_t) -> c_int;
    pub fn git_submodule_set_branch(repo: *mut git_repository,
                                    name: *const c_char,
                                    branch: *const c_char) -> c_int;

    // blob
    pub fn git_blob_free(blob: *mut git_blob);
    pub fn git_blob_id(blob: *const git_blob) -> *const git_oid;
    pub fn git_blob_is_binary(blob: *const git_blob) -> c_int;
    pub fn git_blob_lookup(blob: *mut *mut git_blob, repo: *mut git_repository,
                           id: *const git_oid) -> c_int;
    pub fn git_blob_lookup_prefix(blob: *mut *mut git_blob,
                                  repo: *mut git_repository,
                                  id: *const git_oid,
                                  len: size_t) -> c_int;
    pub fn git_blob_rawcontent(blob: *const git_blob) -> *const c_void;
    pub fn git_blob_rawsize(blob: *const git_blob) -> git_off_t;
    pub fn git_blob_create_frombuffer(id: *mut git_oid,
                                      repo: *mut git_repository,
                                      buffer: *const c_void,
                                      len: size_t) -> c_int;
    pub fn git_blob_create_fromdisk(id: *mut git_oid,
                                    repo: *mut git_repository,
                                    path: *const c_char) -> c_int;
    pub fn git_blob_create_fromworkdir(id: *mut git_oid,
                                       repo: *mut git_repository,
                                       relative_path: *const c_char) -> c_int;
    pub fn git_blob_create_fromstream(out: *mut *mut git_writestream,
                                      repo: *mut git_repository,
                                      hintpath: *const c_char) -> c_int;
    pub fn git_blob_create_fromstream_commit(id: *mut git_oid,
                                             stream: *mut git_writestream) -> c_int;

    // tree
    pub fn git_tree_entry_byid(tree: *const git_tree,
                               id: *const git_oid) -> *const git_tree_entry;
    pub fn git_tree_entry_byindex(tree: *const git_tree,
                                  idx: size_t) -> *const git_tree_entry;
    pub fn git_tree_entry_byname(tree: *const git_tree,
                                 filename: *const c_char) -> *const git_tree_entry;
    pub fn git_tree_entry_bypath(out: *mut *mut git_tree_entry,
                                 tree: *const git_tree,
                                 filename: *const c_char) -> c_int;
    pub fn git_tree_entry_cmp(e1: *const git_tree_entry,
                              e2: *const git_tree_entry) -> c_int;
    pub fn git_tree_entry_dup(dest: *mut *mut git_tree_entry,
                              src: *const git_tree_entry) -> c_int;
    pub fn git_tree_entry_filemode(entry: *const git_tree_entry) -> git_filemode_t;
    pub fn git_tree_entry_filemode_raw(entry: *const git_tree_entry) -> git_filemode_t;
    pub fn git_tree_entry_free(entry: *mut git_tree_entry);
    pub fn git_tree_entry_id(entry: *const git_tree_entry) -> *const git_oid;
    pub fn git_tree_entry_name(entry: *const git_tree_entry) -> *const c_char;
    pub fn git_tree_entry_to_object(out: *mut *mut git_object,
                                    repo: *mut git_repository,
                                    entry: *const git_tree_entry) -> c_int;
    pub fn git_tree_entry_type(entry: *const git_tree_entry) -> git_otype;
    pub fn git_tree_entrycount(tree: *const git_tree) -> size_t;
    pub fn git_tree_free(tree: *mut git_tree);
    pub fn git_tree_id(tree: *const git_tree) -> *const git_oid;
    pub fn git_tree_lookup(tree: *mut *mut git_tree,
                           repo: *mut git_repository,
                           id: *const git_oid) -> c_int;
    pub fn git_tree_walk(tree: *const git_tree,
                         mode: git_treewalk_mode,
                         callback: git_treewalk_cb,
                         payload: *mut c_void) -> c_int;

    // treebuilder
    pub fn git_treebuilder_new(out: *mut *mut git_treebuilder,
                               repo: *mut git_repository,
                               source: *const git_tree) -> c_int;
    pub fn git_treebuilder_clear(bld: *mut git_treebuilder);
    pub fn git_treebuilder_entrycount(bld: *mut git_treebuilder) -> c_uint;
    pub fn git_treebuilder_free(bld: *mut git_treebuilder);
    pub fn git_treebuilder_get(bld: *mut git_treebuilder,
                               filename: *const c_char) -> *const git_tree_entry;
    pub fn git_treebuilder_insert(out: *mut *const git_tree_entry,
                                  bld: *mut git_treebuilder,
                                  filename: *const c_char,
                                  id: *const git_oid,
                                  filemode: git_filemode_t) -> c_int;
    pub fn git_treebuilder_remove(bld: *mut git_treebuilder,
                                  filename: *const c_char) -> c_int;
    pub fn git_treebuilder_filter(bld: *mut git_treebuilder,
                                  filter: git_treebuilder_filter_cb,
                                  payload: *mut c_void);
    pub fn git_treebuilder_write(id: *mut git_oid,
                                 bld: *mut git_treebuilder) -> c_int;

    // buf
    pub fn git_buf_free(buffer: *mut git_buf);
    pub fn git_buf_grow(buffer: *mut git_buf, target_size: size_t) -> c_int;
    pub fn git_buf_set(buffer: *mut git_buf, data: *const c_void,
                       datalen: size_t) -> c_int;

    // commit
    pub fn git_commit_author(commit: *const git_commit) -> *const git_signature;
    pub fn git_commit_committer(commit: *const git_commit) -> *const git_signature;
    pub fn git_commit_free(commit: *mut git_commit);
    pub fn git_commit_id(commit: *const git_commit) -> *const git_oid;
    pub fn git_commit_lookup(commit: *mut *mut git_commit,
                             repo: *mut git_repository,
                             id: *const git_oid) -> c_int;
    pub fn git_commit_message(commit: *const git_commit) -> *const c_char;
    pub fn git_commit_message_encoding(commit: *const git_commit) -> *const c_char;
    pub fn git_commit_message_raw(commit: *const git_commit) -> *const c_char;
    pub fn git_commit_nth_gen_ancestor(commit: *mut *mut git_commit,
                                       commit: *const git_commit,
                                       n: c_uint) -> c_int;
    pub fn git_commit_parent(out: *mut *mut git_commit,
                             commit: *const git_commit,
                             n: c_uint) -> c_int;
    pub fn git_commit_parent_id(commit: *const git_commit,
                                n: c_uint) -> *const git_oid;
    pub fn git_commit_parentcount(commit: *const git_commit) -> c_uint;
    pub fn git_commit_raw_header(commit: *const git_commit) -> *const c_char;
    pub fn git_commit_summary(commit: *mut git_commit) -> *const c_char;
    pub fn git_commit_time(commit: *const git_commit) -> git_time_t;
    pub fn git_commit_time_offset(commit: *const git_commit) -> c_int;
    pub fn git_commit_tree(tree_out: *mut *mut git_tree,
                           commit: *const git_commit) -> c_int;
    pub fn git_commit_tree_id(commit: *const git_commit) -> *const git_oid;
    pub fn git_commit_amend(id: *mut git_oid,
                            commit_to_amend: *const git_commit,
                            update_ref: *const c_char,
                            author: *const git_signature,
                            committer: *const git_signature,
                            message_encoding: *const c_char,
                            message: *const c_char,
                            tree: *const git_tree) -> c_int;
    pub fn git_commit_create(id: *mut git_oid,
                             repo: *mut git_repository,
                             update_ref: *const c_char,
                             author: *const git_signature,
                             committer: *const git_signature,
                             message_encoding: *const c_char,
                             message: *const c_char,
                             tree: *const git_tree,
                             parent_count: size_t,
                             parents: *mut *const git_commit) -> c_int;
    pub fn git_commit_header_field(out: *mut git_buf,
                                   commit: *const git_commit,
                                   field: *const c_char) -> c_int;
    pub fn git_annotated_commit_lookup(out: *mut *mut git_annotated_commit,
                                       repo: *mut git_repository,
                                       id: *const git_oid) -> c_int;

    // branch
    pub fn git_branch_create(out: *mut *mut git_reference,
                             repo: *mut git_repository,
                             branch_name: *const c_char,
                             target: *const git_commit,
                             force: c_int) -> c_int;
    pub fn git_branch_delete(branch: *mut git_reference) -> c_int;
    pub fn git_branch_is_head(branch: *const git_reference) -> c_int;
    pub fn git_branch_iterator_free(iter: *mut git_branch_iterator);
    pub fn git_branch_iterator_new(iter: *mut *mut git_branch_iterator,
                                   repo: *mut git_repository,
                                   list_flags: git_branch_t) -> c_int;
    pub fn git_branch_lookup(out: *mut *mut git_reference,
                             repo: *mut git_repository,
                             branch_name: *const c_char,
                             branch_type: git_branch_t) -> c_int;
    pub fn git_branch_move(out: *mut *mut git_reference,
                           branch: *mut git_reference,
                           new_branch_name: *const c_char,
                           force: c_int) -> c_int;
    pub fn git_branch_name(out: *mut *const c_char,
                           branch: *const git_reference) -> c_int;
    pub fn git_branch_next(out: *mut *mut git_reference,
                           out_type: *mut git_branch_t,
                           iter: *mut git_branch_iterator) -> c_int;
    pub fn git_branch_set_upstream(branch: *mut git_reference,
                                   upstream_name: *const c_char) -> c_int;
    pub fn git_branch_upstream(out: *mut *mut git_reference,
                               branch: *const git_reference) -> c_int;

    // index
    pub fn git_index_add(index: *mut git_index,
                         entry: *const git_index_entry) -> c_int;
    pub fn git_index_add_all(index: *mut git_index,
                             pathspec: *const git_strarray,
                             flags: c_uint,
                             callback: Option<git_index_matched_path_cb>,
                             payload: *mut c_void) -> c_int;
    pub fn git_index_add_bypath(index: *mut git_index,
                                path: *const c_char) -> c_int;
    pub fn git_index_add_frombuffer(index: *mut git_index,
                                    entry: *const git_index_entry,
                                    buffer: *const c_void,
                                    len: size_t) -> c_int;
    pub fn git_index_conflict_add(index: *mut git_index,
                                  ancestor_entry: *const git_index_entry,
                                  our_entry: *const git_index_entry,
                                  their_entry: *const git_index_entry) -> c_int;
    pub fn git_index_clear(index: *mut git_index) -> c_int;
    pub fn git_index_entry_stage(entry: *const git_index_entry) -> c_int;
    pub fn git_index_entrycount(entry: *const git_index) -> size_t;
    pub fn git_index_find(at_pos: *mut size_t,
                          index: *mut git_index,
                          path: *const c_char) -> c_int;
    pub fn git_index_free(index: *mut git_index);
    pub fn git_index_get_byindex(index: *mut git_index,
                                 n: size_t) -> *const git_index_entry;
    pub fn git_index_get_bypath(index: *mut git_index,
                                path: *const c_char,
                                stage: c_int) -> *const git_index_entry;
    pub fn git_index_has_conflicts(index: *const git_index) -> c_int;
    pub fn git_index_new(index: *mut *mut git_index) -> c_int;
    pub fn git_index_open(index: *mut *mut git_index,
                          index_path: *const c_char) -> c_int;
    pub fn git_index_path(index: *const git_index) -> *const c_char;
    pub fn git_index_read(index: *mut git_index, force: c_int) -> c_int;
    pub fn git_index_read_tree(index: *mut git_index,
                               tree: *const git_tree) -> c_int;
    pub fn git_index_remove(index: *mut git_index,
                            path: *const c_char,
                            stage: c_int) -> c_int;
    pub fn git_index_remove_all(index: *mut git_index,
                                pathspec: *const git_strarray,
                                callback: Option<git_index_matched_path_cb>,
                                payload: *mut c_void) -> c_int;
    pub fn git_index_remove_bypath(index: *mut git_index,
                                   path: *const c_char) -> c_int;
    pub fn git_index_remove_directory(index: *mut git_index,
                                      dir: *const c_char,
                                      stage: c_int) -> c_int;
    pub fn git_index_update_all(index: *mut git_index,
                                pathspec: *const git_strarray,
                                callback: Option<git_index_matched_path_cb>,
                                payload: *mut c_void) -> c_int;
    pub fn git_index_write(index: *mut git_index) -> c_int;
    pub fn git_index_write_tree(out: *mut git_oid,
                                index: *mut git_index) -> c_int;
    pub fn git_index_write_tree_to(out: *mut git_oid,
                                   index: *mut git_index,
                                   repo: *mut git_repository) -> c_int;

    // config
    pub fn git_config_add_file_ondisk(cfg: *mut git_config,
                                      path: *const c_char,
                                      level: git_config_level_t,
                                      force: c_int) -> c_int;
    pub fn git_config_delete_entry(cfg: *mut git_config,
                                   name: *const c_char) -> c_int;
    pub fn git_config_delete_multivar(cfg: *mut git_config,
                                      name: *const c_char,
                                      regexp: *const c_char) -> c_int;
    pub fn git_config_find_programdata(out: *mut git_buf) -> c_int;
    pub fn git_config_find_global(out: *mut git_buf) -> c_int;
    pub fn git_config_find_system(out: *mut git_buf) -> c_int;
    pub fn git_config_find_xdg(out: *mut git_buf) -> c_int;
    pub fn git_config_free(cfg: *mut git_config);
    pub fn git_config_get_bool(out: *mut c_int,
                               cfg: *const git_config,
                               name: *const c_char) -> c_int;
    pub fn git_config_get_entry(out: *mut *mut git_config_entry,
                                cfg: *const git_config,
                                name: *const c_char) -> c_int;
    pub fn git_config_get_int32(out: *mut i32,
                                cfg: *const git_config,
                                name: *const c_char) -> c_int;
    pub fn git_config_get_int64(out: *mut i64,
                                cfg: *const git_config,
                                name: *const c_char) -> c_int;
    pub fn git_config_get_string(out: *mut *const c_char,
                                 cfg: *const git_config,
                                 name: *const c_char) -> c_int;
    pub fn git_config_get_string_buf(out: *mut git_buf,
                                     cfg: *const git_config,
                                     name: *const c_char) -> c_int;
    pub fn git_config_get_path(out: *mut git_buf,
                               cfg: *const git_config,
                               name: *const c_char) -> c_int;
    pub fn git_config_iterator_free(iter: *mut git_config_iterator);
    pub fn git_config_iterator_glob_new(out: *mut *mut git_config_iterator,
                                        cfg: *const git_config,
                                        regexp: *const c_char) -> c_int;
    pub fn git_config_iterator_new(out: *mut *mut git_config_iterator,
                                   cfg: *const git_config) -> c_int;
    pub fn git_config_new(out: *mut *mut git_config) -> c_int;
    pub fn git_config_next(entry: *mut *mut git_config_entry,
                           iter: *mut git_config_iterator) -> c_int;
    pub fn git_config_open_default(out: *mut *mut git_config) -> c_int;
    pub fn git_config_open_global(out: *mut *mut git_config,
                                  config: *mut git_config) -> c_int;
    pub fn git_config_open_level(out: *mut *mut git_config,
                                 parent: *const git_config,
                                 level: git_config_level_t) -> c_int;
    pub fn git_config_open_ondisk(out: *mut *mut git_config,
                                  path: *const c_char) -> c_int;
    pub fn git_config_parse_bool(out: *mut c_int,
                                 value: *const c_char) -> c_int;
    pub fn git_config_parse_int32(out: *mut i32,
                                  value: *const c_char) -> c_int;
    pub fn git_config_parse_int64(out: *mut i64,
                                  value: *const c_char) -> c_int;
    pub fn git_config_set_bool(cfg: *mut git_config,
                               name: *const c_char,
                               value: c_int) -> c_int;
    pub fn git_config_set_int32(cfg: *mut git_config,
                                name: *const c_char,
                                value: i32) -> c_int;
    pub fn git_config_set_int64(cfg: *mut git_config,
                                name: *const c_char,
                                value: i64) -> c_int;
    pub fn git_config_set_multivar(cfg: *mut git_config,
                                   name: *const c_char,
                                   regexp: *const c_char,
                                   value: *const c_char) -> c_int;
    pub fn git_config_set_string(cfg: *mut git_config,
                                 name: *const c_char,
                                 value: *const c_char) -> c_int;
    pub fn git_config_snapshot(out: *mut *mut git_config,
                               config: *mut git_config) -> c_int;
    pub fn git_config_entry_free(entry: *mut git_config_entry);

    // cred
    pub fn git_cred_default_new(out: *mut *mut git_cred) -> c_int;
    pub fn git_cred_has_username(cred: *mut git_cred) -> c_int;
    pub fn git_cred_ssh_custom_new(out: *mut *mut git_cred,
                                   username: *const c_char,
                                   publickey: *const c_char,
                                   publickey_len: size_t,
                                   sign_callback: git_cred_sign_callback,
                                   payload: *mut c_void) -> c_int;
    pub fn git_cred_ssh_interactive_new(out: *mut *mut git_cred,
                                        username: *const c_char,
                                        prompt_callback: git_cred_ssh_interactive_callback,
                                        payload: *mut c_void) -> c_int;
    pub fn git_cred_ssh_key_from_agent(out: *mut *mut git_cred,
                                       username: *const c_char) -> c_int;
    pub fn git_cred_ssh_key_new(out: *mut *mut git_cred,
                                username: *const c_char,
                                publickey: *const c_char,
                                privatekey: *const c_char,
                                passphrase: *const c_char) -> c_int;
    pub fn git_cred_ssh_key_memory_new(out: *mut *mut git_cred,
                                       username: *const c_char,
                                       publickey: *const c_char,
                                       privatekey: *const c_char,
                                       passphrase: *const c_char) -> c_int;
    pub fn git_cred_userpass(cred: *mut *mut git_cred,
                             url: *const c_char,
                             user_from_url: *const c_char,
                             allowed_types: c_uint,
                             payload: *mut c_void) -> c_int;
    pub fn git_cred_userpass_plaintext_new(out: *mut *mut git_cred,
                                           username: *const c_char,
                                           password: *const c_char) -> c_int;
    pub fn git_cred_username_new(cred: *mut *mut git_cred,
                                 username: *const c_char) -> c_int;

    // tags
    pub fn git_tag_annotation_create(oid: *mut git_oid,
                                     repo: *mut git_repository,
                                     tag_name: *const c_char,
                                     target: *const git_object,
                                     tagger: *const git_signature,
                                     message: *const c_char) -> c_int;
    pub fn git_tag_create(oid: *mut git_oid,
                          repo: *mut git_repository,
                          tag_name: *const c_char,
                          target: *const git_object,
                          tagger: *const git_signature,
                          message: *const c_char,
                          force: c_int) -> c_int;
    pub fn git_tag_create_frombuffer(oid: *mut git_oid,
                                     repo: *mut git_repository,
                                     buffer: *const c_char,
                                     force: c_int) -> c_int;
    pub fn git_tag_create_lightweight(oid: *mut git_oid,
                                      repo: *mut git_repository,
                                      tag_name: *const c_char,
                                      target: *const git_object,
                                      force: c_int) -> c_int;
    pub fn git_tag_delete(repo: *mut git_repository,
                          tag_name: *const c_char) -> c_int;
    pub fn git_tag_foreach(repo: *mut git_repository,
                           callback: git_tag_foreach_cb,
                           payload: *mut c_void) -> c_int;
    pub fn git_tag_free(tag: *mut git_tag);
    pub fn git_tag_id(tag: *const git_tag) -> *const git_oid;
    pub fn git_tag_list(tag_names: *mut git_strarray,
                        repo: *mut git_repository) -> c_int;
    pub fn git_tag_list_match(tag_names: *mut git_strarray,
                              pattern: *const c_char,
                              repo: *mut git_repository) -> c_int;
    pub fn git_tag_lookup(out: *mut *mut git_tag,
                          repo: *mut git_repository,
                          id: *const git_oid) -> c_int;
    pub fn git_tag_lookup_prefix(out: *mut *mut git_tag,
                                 repo: *mut git_repository,
                                 id: *const git_oid,
                                 len: size_t) -> c_int;
    pub fn git_tag_message(tag: *const git_tag) -> *const c_char;
    pub fn git_tag_name(tag: *const git_tag) -> *const c_char;
    pub fn git_tag_peel(tag_target_out: *mut *mut git_object,
                        tag: *const git_tag) -> c_int;
    pub fn git_tag_tagger(tag: *const git_tag) -> *const git_signature;
    pub fn git_tag_target(target_out: *mut *mut git_object,
                          tag: *const git_tag) -> c_int;
    pub fn git_tag_target_id(tag: *const git_tag) -> *const git_oid;
    pub fn git_tag_target_type(tag: *const git_tag) -> git_otype;

    // checkout
    pub fn git_checkout_head(repo: *mut git_repository,
                             opts: *const git_checkout_options) -> c_int;
    pub fn git_checkout_index(repo: *mut git_repository,
                              index: *mut git_index,
                              opts: *const git_checkout_options) -> c_int;
    pub fn git_checkout_tree(repo: *mut git_repository,
                             treeish: *const git_object,
                             opts: *const git_checkout_options) -> c_int;
    pub fn git_checkout_init_options(opts: *mut git_checkout_options,
                                     version: c_uint) -> c_int;

    // merge
    pub fn git_annotated_commit_id(commit: *const git_annotated_commit)
                                   -> *const git_oid;
    pub fn git_annotated_commit_from_ref(out: *mut *mut git_annotated_commit,
                                         repo: *mut git_repository,
                                         reference: *const git_reference)
                                         -> c_int;
    pub fn git_annotated_commit_free(commit: *mut git_annotated_commit);
    pub fn git_merge_init_options(opts: *mut git_merge_options,
                                  version: c_uint) -> c_int;
    pub fn git_merge(repo: *mut git_repository,
                     their_heads: *mut *const git_annotated_commit,
                     len: size_t,
                     merge_opts: *const git_merge_options,
                     checkout_opts: *const git_checkout_options) -> c_int;
    pub fn git_merge_commits(out: *mut *mut git_index,
                             repo: *mut git_repository,
                             our_commit: *const git_commit,
                             their_commit: *const git_commit,
                             opts: *const git_merge_options) -> c_int;
    pub fn git_repository_state_cleanup(repo: *mut git_repository) -> c_int;

    // merge analysis

    pub fn git_merge_analysis(analysis_out: *mut git_merge_analysis_t,
                              pref_out: *mut git_merge_preference_t,
                              repo: *mut git_repository,
                              their_heads: *mut *const git_annotated_commit,
                              their_heads_len: usize) -> c_int;

    // notes
    pub fn git_note_author(note: *const git_note) -> *const git_signature;
    pub fn git_note_committer(note: *const git_note) -> *const git_signature;
    pub fn git_note_create(out: *mut git_oid,
                           repo: *mut git_repository,
                           notes_ref: *const c_char,
                           author: *const git_signature,
                           committer: *const git_signature,
                           oid: *const git_oid,
                           note: *const c_char,
                           force: c_int) -> c_int;
    pub fn git_note_default_ref(out: *mut git_buf,
                                repo: *mut git_repository) -> c_int;
    pub fn git_note_free(note: *mut git_note);
    pub fn git_note_id(note: *const git_note) -> *const git_oid;
    pub fn git_note_iterator_free(it: *mut git_note_iterator);
    pub fn git_note_iterator_new(out: *mut *mut git_note_iterator,
                                 repo: *mut git_repository,
                                 notes_ref: *const c_char) -> c_int;
    pub fn git_note_message(note: *const git_note) -> *const c_char;
    pub fn git_note_next(note_id: *mut git_oid,
                         annotated_id: *mut git_oid,
                         it: *mut git_note_iterator) -> c_int;
    pub fn git_note_read(out: *mut *mut git_note,
                         repo: *mut git_repository,
                         notes_ref: *const c_char,
                         oid: *const git_oid) -> c_int;
    pub fn git_note_remove(repo: *mut git_repository,
                           notes_ref: *const c_char,
                           author: *const git_signature,
                           committer: *const git_signature,
                           oid: *const git_oid) -> c_int;

    // blame
    pub fn git_blame_file(out: *mut *mut git_blame,
                          repo: *mut git_repository,
                          path: *const c_char,
                          options: *mut git_blame_options) -> c_int;
    pub fn git_blame_free(blame: *mut git_blame);

    pub fn git_blame_init_options(opts: *mut git_blame_options,
                                  version: c_uint) -> c_int;
    pub fn git_blame_get_hunk_count(blame: *mut git_blame) -> u32;

    pub fn git_blame_get_hunk_byline(blame: *mut git_blame,
                                     lineno: usize) -> *const git_blame_hunk;
    pub fn git_blame_get_hunk_byindex(blame: *mut git_blame,
                                      index: u32) -> *const git_blame_hunk;

    // revwalk
    pub fn git_revwalk_new(out: *mut *mut git_revwalk,
                           repo: *mut git_repository) -> c_int;
    pub fn git_revwalk_free(walk: *mut git_revwalk);

    pub fn git_revwalk_reset(walk: *mut git_revwalk);

    pub fn git_revwalk_sorting(walk: *mut git_revwalk, sort_mode: c_uint);

    pub fn git_revwalk_push_head(walk: *mut git_revwalk) -> c_int;
    pub fn git_revwalk_push(walk: *mut git_revwalk,
                            oid: *const git_oid) -> c_int;
    pub fn git_revwalk_push_ref(walk: *mut git_revwalk,
                                refname: *const c_char) -> c_int;
    pub fn git_revwalk_push_glob(walk: *mut git_revwalk,
                                 glob: *const c_char) -> c_int;
    pub fn git_revwalk_push_range(walk: *mut git_revwalk,
                                  range: *const c_char) -> c_int;
    pub fn git_revwalk_simplify_first_parent(walk: *mut git_revwalk);

    pub fn git_revwalk_hide_head(walk: *mut git_revwalk) -> c_int;
    pub fn git_revwalk_hide(walk: *mut git_revwalk,
                            oid: *const git_oid) -> c_int;
    pub fn git_revwalk_hide_ref(walk: *mut git_revwalk,
                                refname: *const c_char) -> c_int;
    pub fn git_revwalk_hide_glob(walk: *mut git_revwalk,
                                 refname: *const c_char) -> c_int;
    pub fn git_revwalk_add_hide_cb(walk: *mut git_revwalk,
                                   hide_cb: git_revwalk_hide_cb,
                                   payload: *mut c_void) -> c_int;

    pub fn git_revwalk_next(out: *mut git_oid, walk: *mut git_revwalk) -> c_int;

    // merge
    pub fn git_merge_base(out: *mut git_oid,
                          repo: *mut git_repository,
                          one: *const git_oid,
                          two: *const git_oid) -> c_int;

    pub fn git_merge_bases(out: *mut git_oidarray,
                           repo: *mut git_repository,
                           one: *const git_oid,
                           two: *const git_oid) -> c_int;

    // pathspec
    pub fn git_pathspec_free(ps: *mut git_pathspec);
    pub fn git_pathspec_match_diff(out: *mut *mut git_pathspec_match_list,
                                   diff: *mut git_diff,
                                   flags: u32,
                                   ps: *mut git_pathspec) -> c_int;
    pub fn git_pathspec_match_index(out: *mut *mut git_pathspec_match_list,
                                    index: *mut git_index,
                                    flags: u32,
                                    ps: *mut git_pathspec) -> c_int;
    pub fn git_pathspec_match_list_diff_entry(m: *const git_pathspec_match_list,
                                              pos: size_t) -> *const git_diff_delta;
    pub fn git_pathspec_match_list_entry(m: *const git_pathspec_match_list,
                                         pos: size_t) -> *const c_char;
    pub fn git_pathspec_match_list_entrycount(m: *const git_pathspec_match_list)
                                              -> size_t;
    pub fn git_pathspec_match_list_failed_entry(m: *const git_pathspec_match_list,
                                                pos: size_t) -> *const c_char;
    pub fn git_pathspec_match_list_failed_entrycount(
                    m: *const git_pathspec_match_list) -> size_t;
    pub fn git_pathspec_match_list_free(m: *mut git_pathspec_match_list);
    pub fn git_pathspec_match_tree(out: *mut *mut git_pathspec_match_list,
                                   tree: *mut git_tree,
                                   flags: u32,
                                   ps: *mut git_pathspec) -> c_int;
    pub fn git_pathspec_match_workdir(out: *mut *mut git_pathspec_match_list,
                                      repo: *mut git_repository,
                                      flags: u32,
                                      ps: *mut git_pathspec) -> c_int;
    pub fn git_pathspec_matches_path(ps: *const git_pathspec,
                                     flags: u32,
                                     path: *const c_char) -> c_int;
    pub fn git_pathspec_new(out: *mut *mut git_pathspec,
                            pathspec: *const git_strarray) -> c_int;

    // diff
    pub fn git_diff_blob_to_buffer(old_blob: *const git_blob,
                                   old_as_path: *const c_char,
                                   buffer: *const c_char,
                                   buffer_len: size_t,
                                   buffer_as_path: *const c_char,
                                   options: *const git_diff_options,
                                   file_cb: git_diff_file_cb,
                                   binary_cb: git_diff_binary_cb,
                                   hunk_cb: git_diff_hunk_cb,
                                   line_cb: git_diff_line_cb,
                                   payload: *mut c_void) -> c_int;
    pub fn git_diff_blobs(old_blob: *const git_blob,
                          old_as_path: *const c_char,
                          new_blob: *const git_blob,
                          new_as_path: *const c_char,
                          options: *const git_diff_options,
                          file_cb: git_diff_file_cb,
                          binary_cb: git_diff_binary_cb,
                          hunk_cb: git_diff_hunk_cb,
                          line_cb: git_diff_line_cb,
                          payload: *mut c_void) -> c_int;
    pub fn git_diff_buffers(old_buffer: *const c_void,
                            old_len: size_t,
                            old_as_path: *const c_char,
                            new_buffer: *const c_void,
                            new_len: size_t,
                            new_as_path: *const c_char,
                            options: *const git_diff_options,
                            file_cb: git_diff_file_cb,
                            binary_cb: git_diff_binary_cb,
                            hunk_cb: git_diff_hunk_cb,
                            line_cb: git_diff_line_cb,
                            payload: *mut c_void) -> c_int;
    pub fn git_diff_find_similar(diff: *mut git_diff,
                                 options: *const git_diff_find_options) -> c_int;
    pub fn git_diff_find_init_options(opts: *mut git_diff_find_options,
                                      version: c_uint) -> c_int;
    pub fn git_diff_foreach(diff: *mut git_diff,
                            file_cb: git_diff_file_cb,
                            binary_cb: Option<git_diff_binary_cb>,
                            hunk_cb: Option<git_diff_hunk_cb>,
                            line_cb: Option<git_diff_line_cb>,
                            payload: *mut c_void) -> c_int;
    pub fn git_diff_free(diff: *mut git_diff);
    pub fn git_diff_get_delta(diff: *const git_diff,
                              idx: size_t) -> *const git_diff_delta;
    pub fn git_diff_get_stats(out: *mut *mut git_diff_stats,
                              diff: *mut git_diff) -> c_int;
    pub fn git_diff_index_to_index(diff: *mut *mut git_diff,
                                   repo: *mut git_repository,
                                   old_index: *mut git_index,
                                   new_index: *mut git_index,
                                   opts: *const git_diff_options) -> c_int;
    pub fn git_diff_index_to_workdir(diff: *mut *mut git_diff,
                                     repo: *mut git_repository,
                                     index: *mut git_index,
                                     opts: *const git_diff_options) -> c_int;
    pub fn git_diff_init_options(opts: *mut git_diff_options,
                                 version: c_uint) -> c_int;
    pub fn git_diff_is_sorted_icase(diff: *const git_diff) -> c_int;
    pub fn git_diff_merge(onto: *mut git_diff,
                          from: *const git_diff) -> c_int;
    pub fn git_diff_num_deltas(diff: *const git_diff) -> size_t;
    pub fn git_diff_num_deltas_of_type(diff: *const git_diff,
                                       delta: git_delta_t) -> size_t;
    pub fn git_diff_print(diff: *mut git_diff,
                          format: git_diff_format_t,
                          print_cb: git_diff_line_cb,
                          payload: *mut c_void) -> c_int;
    pub fn git_diff_stats_deletions(stats: *const git_diff_stats) -> size_t;
    pub fn git_diff_stats_files_changed(stats: *const git_diff_stats) -> size_t;
    pub fn git_diff_stats_free(stats: *mut git_diff_stats);
    pub fn git_diff_stats_insertions(stats: *const git_diff_stats) -> size_t;
    pub fn git_diff_stats_to_buf(out: *mut git_buf,
                                 stats: *const git_diff_stats,
                                 format: git_diff_stats_format_t,
                                 width: size_t) -> c_int;
    pub fn git_diff_status_char(status: git_delta_t) -> c_char;
    pub fn git_diff_tree_to_index(diff: *mut *mut git_diff,
                                  repo: *mut git_repository,
                                  old_tree: *mut git_tree,
                                  index: *mut git_index,
                                  opts: *const git_diff_options) -> c_int;
    pub fn git_diff_tree_to_tree(diff: *mut *mut git_diff,
                                 repo: *mut git_repository,
                                 old_tree: *mut git_tree,
                                 new_tree: *mut git_tree,
                                 opts: *const git_diff_options) -> c_int;
    pub fn git_diff_tree_to_workdir(diff: *mut *mut git_diff,
                                    repo: *mut git_repository,
                                    old_tree: *mut git_tree,
                                    opts: *const git_diff_options) -> c_int;
    pub fn git_diff_tree_to_workdir_with_index(diff: *mut *mut git_diff,
                                               repo: *mut git_repository,
                                               old_tree: *mut git_tree,
                                               opts: *const git_diff_options)
                                               -> c_int;

    pub fn git_graph_ahead_behind(ahead: *mut size_t, behind: *mut size_t,
                                  repo: *mut git_repository,
                                  local: *const git_oid, upstream: *const git_oid)
                                  -> c_int;

    pub fn git_graph_descendant_of(repo: *mut git_repository,
                                   commit: *const git_oid, ancestor: *const git_oid)
                                   -> c_int;

    // patch
    pub fn git_patch_from_diff(out: *mut *mut git_patch,
                               diff: *mut git_diff,
                               idx: size_t) -> c_int;
    pub fn git_patch_from_blobs(out: *mut *mut git_patch,
                                old_blob: *const git_blob,
                                old_as_path: *const c_char,
                                new_blob: *const git_blob,
                                new_as_path: *const c_char,
                                opts: *const git_diff_options) -> c_int;
    pub fn git_patch_from_blob_and_buffer(out: *mut *mut git_patch,
                                          old_blob: *const git_blob,
                                          old_as_path: *const c_char,
                                          buffer: *const c_char,
                                          buffer_len: size_t,
                                          buffer_as_path: *const c_char,
                                          opts: *const git_diff_options) -> c_int;
    pub fn git_patch_from_buffers(out: *mut *mut git_patch,
                                  old_buffer: *const c_void,
                                  old_len: size_t,
                                  old_as_path: *const c_char,
                                  new_buffer: *const c_char,
                                  new_len: size_t,
                                  new_as_path: *const c_char,
                                  opts: *const git_diff_options) -> c_int;
    pub fn git_patch_free(patch: *mut git_patch);
    pub fn git_patch_get_delta(patch: *const git_patch) -> *const git_diff_delta;
    pub fn git_patch_num_hunks(patch: *const git_patch) -> size_t;
    pub fn git_patch_line_stats(total_context: *mut size_t,
                                total_additions: *mut size_t,
                                total_deletions: *mut size_t,
                                patch: *const git_patch) -> c_int;
    pub fn git_patch_get_hunk(out: *mut *const git_diff_hunk,
                              lines_in_hunk: *mut size_t,
                              patch: *mut git_patch,
                              hunk_idx: size_t) -> c_int;
    pub fn git_patch_num_lines_in_hunk(patch: *const git_patch,
                                       hunk_idx: size_t) -> c_int;
    pub fn git_patch_get_line_in_hunk(out: *mut *const git_diff_line,
                                      patch: *mut git_patch,
                                      hunk_idx: size_t,
                                      line_of_hunk: size_t) -> c_int;
    pub fn git_patch_size(patch: *mut git_patch,
                          include_context: c_int,
                          include_hunk_headers: c_int,
                          include_file_headers: c_int) -> size_t;
    pub fn git_patch_print(patch: *mut git_patch,
                           print_cb: git_diff_line_cb,
                           payload: *mut c_void) -> c_int;
    pub fn git_patch_to_buf(buf: *mut git_buf,
                            patch: *mut git_patch) -> c_int;

    // reflog
    pub fn git_reflog_append(reflog: *mut git_reflog,
                             id: *const git_oid,
                             committer: *const git_signature,
                             msg: *const c_char) -> c_int;
    pub fn git_reflog_delete(repo: *mut git_repository,
                             name: *const c_char) -> c_int;
    pub fn git_reflog_drop(reflog: *mut git_reflog,
                           idx: size_t,
                           rewrite_previous_entry: c_int) -> c_int;
    pub fn git_reflog_entry_byindex(reflog: *const git_reflog,
                                    idx: size_t) -> *const git_reflog_entry;
    pub fn git_reflog_entry_committer(entry: *const git_reflog_entry)
                                      -> *const git_signature;
    pub fn git_reflog_entry_id_new(entry: *const git_reflog_entry)
                                   -> *const git_oid;
    pub fn git_reflog_entry_id_old(entry: *const git_reflog_entry)
                                   -> *const git_oid;
    pub fn git_reflog_entry_message(entry: *const git_reflog_entry)
                                    -> *const c_char;
    pub fn git_reflog_entrycount(reflog: *mut git_reflog) -> size_t;
    pub fn git_reflog_free(reflog: *mut git_reflog);
    pub fn git_reflog_read(out: *mut *mut git_reflog,
                           repo: *mut git_repository,
                           name: *const c_char) -> c_int;
    pub fn git_reflog_rename(repo: *mut git_repository,
                             old_name: *const c_char,
                             name: *const c_char) -> c_int;
    pub fn git_reflog_write(reflog: *mut git_reflog) -> c_int;

    // transport
    pub fn git_transport_register(prefix: *const c_char,
                                  cb: git_transport_cb,
                                  param: *mut c_void) -> c_int;
    pub fn git_transport_unregister(prefix: *const c_char) -> c_int;
    pub fn git_transport_smart(out: *mut *mut git_transport,
                               owner: *mut git_remote,
                               payload: *mut c_void) -> c_int;

    // describe
    pub fn git_describe_commit(result: *mut *mut git_describe_result,
                               object: *mut git_object,
                               opts: *mut git_describe_options) -> c_int;
    pub fn git_describe_format(buf: *mut git_buf,
                               result: *const git_describe_result,
                               opts: *const git_describe_format_options) -> c_int;
    pub fn git_describe_result_free(result: *mut git_describe_result);
    pub fn git_describe_workdir(out: *mut *mut git_describe_result,
                                repo: *mut git_repository,
                                opts: *mut git_describe_options) -> c_int;

    // message
    pub fn git_message_prettify(out: *mut git_buf,
                                message: *const c_char,
                                strip_comments: c_int,
                                comment_char: c_char) -> c_int;

    // packbuilder
    pub fn git_packbuilder_new(out: *mut *mut git_packbuilder,
                               repo: *mut git_repository) -> c_int;
    pub fn git_packbuilder_set_threads(pb: *mut git_packbuilder,
                                       n: c_uint) -> c_uint;
    pub fn git_packbuilder_insert(pb: *mut git_packbuilder,
                                  id: *const git_oid,
                                  name: *const c_char) -> c_int;
    pub fn git_packbuilder_insert_tree(pb: *mut git_packbuilder,
                                       id: *const git_oid) -> c_int;
    pub fn git_packbuilder_insert_commit(pb: *mut git_packbuilder,
                                         id: *const git_oid) -> c_int;
    pub fn git_packbuilder_insert_walk(pb: *mut git_packbuilder,
                                       walk: *mut git_revwalk) -> c_int;
    pub fn git_packbuilder_insert_recur(pb: *mut git_packbuilder,
                                        id: *const git_oid,
                                        name: *const c_char) -> c_int;
    pub fn git_packbuilder_write_buf(buf: *mut git_buf,
                                     pb: *mut git_packbuilder) -> c_int;
    pub fn git_packbuilder_write(pb: *mut git_packbuilder,
                                 path: *const c_char,
                                 mode: c_uint,
                                 progress_cb: Option<git_transfer_progress_cb>,
                                 progress_cb_payload: *mut c_void) -> c_int;
    pub fn git_packbuilder_hash(pb: *mut git_packbuilder) -> *const git_oid;
    pub fn git_packbuilder_foreach(pb: *mut git_packbuilder,
                                   cb: git_packbuilder_foreach_cb,
                                   payload: *mut c_void) -> c_int;
    pub fn git_packbuilder_object_count(pb: *mut git_packbuilder) -> size_t;
    pub fn git_packbuilder_written(pb: *mut git_packbuilder) -> size_t;
    pub fn git_packbuilder_set_callbacks(pb: *mut git_packbuilder,
                                         progress_cb: Option<git_packbuilder_progress>,
                                         progress_cb_payload: *mut c_void) -> c_int;
    pub fn git_packbuilder_free(pb: *mut git_packbuilder);

    // odb
    pub fn git_repository_odb(out: *mut *mut git_odb,
                              repo: *mut git_repository) -> c_int;
    pub fn git_odb_free(db: *mut git_odb);
    pub fn git_odb_open_rstream(out: *mut *mut git_odb_stream,
                                db: *mut git_odb,
                                oid: *const git_oid) -> c_int;
    pub fn git_odb_stream_read(stream: *mut git_odb_stream,
                               buffer: *mut c_char,
                               len: size_t) -> c_int;
    pub fn git_odb_open_wstream(out: *mut *mut git_odb_stream,
                                db: *mut git_odb,
                                size: git_off_t,
                                obj_type: git_otype) -> c_int;
    pub fn git_odb_stream_write(stream: *mut git_odb_stream,
                                buffer: *const c_char,
                                len: size_t) -> c_int;
    pub fn git_odb_stream_finalize_write(id: *mut git_oid,
                                         stream: *mut git_odb_stream) -> c_int;
    pub fn git_odb_stream_free(stream: *mut git_odb_stream);
    pub fn git_odb_foreach(db: *mut git_odb, cb: git_odb_foreach_cb, payload: *mut c_void) -> c_int;
}

pub fn init() {
    use std::sync::{Once, ONCE_INIT};

    static INIT: Once = ONCE_INIT;
    INIT.call_once(|| unsafe {
        openssl_init();
        ssh_init();
        let r = git_libgit2_init();
        assert!(r >= 0,
                "couldn't initialize the libgit2 library: {}", r);
        assert_eq!(libc::atexit(shutdown), 0);
    });
    extern fn shutdown() {
        unsafe { git_libgit2_shutdown(); }
    }
}

#[cfg(all(unix, not(target_os = "macos"), not(target_os = "ios"), feature = "https"))]
#[doc(hidden)]
pub fn openssl_init() {
    openssl_sys::init();
}

#[cfg(any(windows, target_os = "macos", target_os = "ios", not(feature = "https")))]
#[doc(hidden)]
pub fn openssl_init() {}

#[cfg(feature = "ssh")]
fn ssh_init() {
    libssh2::init();
}

#[cfg(not(feature = "ssh"))]
fn ssh_init() {}
