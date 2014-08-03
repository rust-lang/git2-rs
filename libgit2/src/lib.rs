extern crate libc;
extern crate openssl;

use libc::{c_int, c_char, c_uint, size_t, c_uchar, c_void};

pub static GIT_OID_RAWSZ: uint = 20;
pub static GIT_OID_HEXSZ: uint = GIT_OID_RAWSZ * 2;
pub static GIT_CLONE_OPTIONS_VERSION: c_uint = 1;
pub static GIT_CHECKOUT_OPTIONS_VERSION: c_uint = 1;
pub static GIT_REMOTE_CALLBACKS_VERSION: c_uint = 1;

pub enum git_object {}
pub enum git_reference {}
pub enum git_refspec {}
pub enum git_remote {}
pub enum git_repository {}
pub enum git_tag {}
pub enum git_cred {}
pub enum git_tree {}

#[repr(C)]
pub struct git_revspec {
    pub from: *mut git_object,
    pub to: *mut git_object,
    pub flags: git_revparse_mode_t,
}

#[repr(C)]
pub struct git_error {
    pub message: *mut c_char,
    pub klass: c_int,
}

#[repr(C)]
pub struct git_oid {
    pub id: [u8, ..GIT_OID_RAWSZ],
}

#[repr(C)]
pub struct git_strarray {
    pub strings: *mut *mut c_char,
    pub count: size_t,
}

#[repr(C)]
pub struct git_signature {
    pub name: *mut c_char,
    pub email: *mut c_char,
    pub when: git_time,
}

#[repr(C)]
pub struct git_time {
    pub time: git_time_t,
    pub offset: c_int,
}

pub type git_off_t = i64;
pub type git_time_t = i64;

bitflags!(
    flags git_revparse_mode_t: c_uint {
        static GIT_REVPARSE_SINGLE = 1 << 0,
        static GIT_REVPARSE_RANGE = 1 << 1,
        static GIT_REVPARSE_MERGE_BASE = 1 << 2
    }
)

#[repr(C)]
#[deriving(PartialEq, Eq, Clone, Show)]
pub enum git_error_code {
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
    GIT_EMERGECONFLICT = -13,
    GIT_ELOCKED = -14,
    GIT_EMODIFIED = -15,
    GIT_PASSTHROUGH = -30,
    GIT_ITEROVER = -31,
}

#[repr(C)]
pub enum git_repository_state_t {
    GIT_REPOSITORY_STATE_NONE,
    GIT_REPOSITORY_STATE_MERGE,
    GIT_REPOSITORY_STATE_REVERT,
    GIT_REPOSITORY_STATE_CHERRYPICK,
    GIT_REPOSITORY_STATE_BISECT,
    GIT_REPOSITORY_STATE_REBASE,
    GIT_REPOSITORY_STATE_REBASE_INTERACTIVE,
    GIT_REPOSITORY_STATE_REBASE_MERGE,
    GIT_REPOSITORY_STATE_APPLY_MAILBOX,
    GIT_REPOSITORY_STATE_APPLY_MAILBOX_OR_REBASE,
}

#[repr(C)]
pub enum git_direction {
    GIT_DIRECTION_FETCH = 0,
    GIT_DIRECTION_PUSH = 1,
}

#[repr(C)]
pub struct git_clone_options {
    pub version: c_uint,
    pub checkout_opts: git_checkout_options,
    pub remote_callbacks: git_remote_callbacks,
    pub bare: c_int,
    pub local: git_clone_local_t,
    pub checkout_branch: *const c_char,
    pub signature: *mut git_signature,
    pub repository_cb: Option<git_repository_create_cb>,
    pub repository_cb_payload: *mut c_void,
    pub remote_cb: Option<git_remote_create_cb>,
    pub remote_cb_payload: *mut c_void,
}

#[repr(C)]
pub enum git_clone_local_t {
    GIT_CLONE_LOCAL_AUTO,
    GIT_CLONE_LOCAL,
    GIT_CLONE_NO_LOCAL,
    GIT_CLONE_LOCAL_NO_LINKS,
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
    pub target_directory: *const c_char,
    pub ancestor_label: *const c_char,
    pub our_label: *const c_char,
    pub their_label: *const c_char,
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

#[repr(C)]
pub struct git_remote_callbacks {
    pub version: c_uint,
    pub sideband_progress: Option<git_transport_message_cb>,
    pub completion: Option<extern fn(git_remote_completion_type,
                                     *mut c_void) -> c_int>,
    pub credentials: Option<git_cred_acquire_cb>,
    pub transfer_progress: Option<git_transfer_progress_cb>,
    pub update_tips: Option<extern fn(*const c_char,
                                      *const git_oid,
                                      *const git_oid,
                                      *mut c_void) -> c_int>,
    pub payload: *mut c_void,
}

#[repr(C)]
pub enum git_remote_completion_type {
    GIT_REMOTE_COMPLETION_DOWNLOAD,
    GIT_REMOTE_COMPLETION_INDEXING,
    GIT_REMOTE_COMPLETION_ERROR,
}

pub type git_transport_message_cb = extern fn(*const c_char, c_int,
                                              *mut c_void) -> c_int;
pub type git_cred_acquire_cb = extern fn(*mut *mut git_cred,
                                         *const c_char, *const c_char,
                                         c_uint, *mut c_void) -> c_int;
pub type git_transfer_progress_cb = extern fn(*const git_transfer_progress,
                                              *mut c_void) -> c_int;

#[repr(C)]
pub struct git_transfer_progress {
    total_objects: c_uint,
    indexed_objects: c_uint,
    received_objects: c_uint,
    local_objects: c_uint,
    total_deltas: c_uint,
    indexed_deltas: c_uint,
    received_bytes: size_t,
}

#[repr(C)]
pub struct git_diff_file {
    id: git_oid,
    path: *const c_char,
    size: git_off_t,
    flags: u32,
    mode: u16,
}

pub type git_repository_create_cb = extern fn(*mut *mut git_repository,
                                              *const c_char,
                                              c_int, *mut c_void) -> c_int;
pub type git_remote_create_cb = extern fn(*mut *mut git_remote,
                                          *mut git_repository,
                                          *const c_char,
                                          *const c_char,
                                          *mut c_void) -> c_int;

#[repr(C)]
pub enum git_checkout_notify_t {
    GIT_CHECKOUT_NOTIFY_NONE = 0,
    GIT_CHECKOUT_NOTIFY_CONFLICT = (1 << 0),
    GIT_CHECKOUT_NOTIFY_DIRTY = (1 << 1),
    GIT_CHECKOUT_NOTIFY_UPDATED = (1 << 2),
    GIT_CHECKOUT_NOTIFY_UNTRACKED = (1 << 3),
    GIT_CHECKOUT_NOTIFY_IGNORED = (1 << 4),

    GIT_CHECKOUT_NOTIFY_ALL = 0x0FFFF,
}

#[repr(C)]
pub enum git_checkout_strategy_t {
    GIT_CHECKOUT_NONE = 0,
    GIT_CHECKOUT_SAFE = (1 << 0),
    GIT_CHECKOUT_SAFE_CREATE = (1 << 1),
    GIT_CHECKOUT_FORCE = (1 << 2),
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

#[link(name = "git2", kind = "static")]
#[link(name = "z")]
extern {
    // threads
    pub fn git_threads_init() -> c_int;
    pub fn git_threads_shutdown();

    // repository
    pub fn git_repository_free(repo: *mut git_repository);
    pub fn git_repository_open(repo: *mut *mut git_repository,
                               path: *const c_char) -> c_int;
    pub fn git_repository_init(repo: *mut *mut git_repository,
                               path: *const c_char,
                               is_bare: c_uint) -> c_int;
    pub fn git_repository_get_namespace(repo: *mut git_repository)
                                        -> *const c_char;
    pub fn git_repository_head(out: *mut *mut git_reference,
                               repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_bare(repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_empty(repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_shallow(repo: *mut git_repository) -> c_int;
    pub fn git_repository_path(repo: *mut git_repository) -> *const c_char;
    pub fn git_repository_state(repo: *mut git_repository) -> c_int;
    pub fn git_repository_workdir(repo: *mut git_repository) -> *const c_char;

    // revparse
    pub fn git_revparse(revspec: *mut git_revspec,
                        repo: *mut git_repository,
                        spec: *const c_char) -> c_int;
    pub fn git_revparse_single(out: *mut *mut git_object,
                               repo: *mut git_repository,
                               spec: *const c_char) -> c_int;

    // object
    pub fn git_object_dup(dest: *mut *mut git_object,
                          source: *mut git_object) -> c_int;
    pub fn git_object_id(obj: *const git_object) -> *const git_oid;
    pub fn git_object_free(object: *mut git_object);

    // oid
    pub fn git_oid_fromraw(out: *mut git_oid, raw: *const c_uchar);
    pub fn git_oid_fromstrn(out: *mut git_oid, str: *const c_char,
                            len: size_t) -> c_int;
    pub fn git_oid_tostr(out: *mut c_char, n: size_t,
                         id: *const git_oid) -> *mut c_char;
    pub fn git_oid_cmp(a: *const git_oid, b: *const git_oid) -> c_int;
    pub fn git_oid_equal(a: *const git_oid, b: *const git_oid) -> c_int;
    pub fn git_oid_streq(id: *const git_oid, str: *const c_char) -> c_int;

    // giterr
    pub fn giterr_last() -> *const git_error;
    pub fn giterr_clear();
    pub fn giterr_detach(cpy: *mut git_error) -> c_int;

    // remote
    pub fn git_remote_create(out: *mut *mut git_remote,
                             repo: *mut git_repository,
                             name: *const c_char,
                             url: *const c_char) -> c_int;
    pub fn git_remote_load(out: *mut *mut git_remote,
                           repo: *mut git_repository,
                           name: *const c_char) -> c_int;
    pub fn git_remote_create_anonymous(out: *mut *mut git_remote,
                                       repo: *mut git_repository,
                                       url: *const c_char,
                                       fetch: *const c_char) -> c_int;
    pub fn git_remote_delete(remote: *mut git_remote) -> c_int;
    pub fn git_remote_free(remote: *mut git_remote);
    pub fn git_remote_name(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_owner(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_pushurl(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_refspec_count(remote: *const git_remote) -> size_t;
    pub fn git_remote_url(remote: *const git_remote) -> *const c_char;
    pub fn git_remote_connect(remote: *mut git_remote,
                              dir: git_direction) -> c_int;
    pub fn git_remote_connected(remote: *mut git_remote) -> c_int;
    pub fn git_remote_disconnect(remote: *mut git_remote);
    pub fn git_remote_save(remote: *const git_remote) -> c_int;
    pub fn git_remote_add_fetch(remote: *mut git_remote,
                                refspec: *const c_char) -> c_int;
    pub fn git_remote_add_push(remote: *mut git_remote,
                               refspec: *const c_char) -> c_int;
    pub fn git_remote_check_cert(remote: *mut git_remote, check: c_int);
    pub fn git_remote_clear_refspecs(remote: *mut git_remote);
    pub fn git_remote_download(remote: *mut git_remote) -> c_int;
    pub fn git_remote_dup(dest: *mut *mut git_remote,
                          source: *mut git_remote) -> c_int;
    pub fn git_remote_get_fetch_refspecs(array: *mut git_strarray,
                                         remote: *const git_remote) -> c_int;
    pub fn git_remote_get_refspec(remote: *const git_remote,
                                  n: size_t) -> *const git_refspec;
    pub fn git_remote_is_valid_name(remote_name: *const c_char) -> c_int;
    pub fn git_remote_valid_url(url: *const c_char) -> c_int;
    pub fn git_remote_supported_url(url: *const c_char) -> c_int;
    pub fn git_remote_list(out: *mut git_strarray,
                           repo: *mut git_repository) -> c_int;
    pub fn git_remote_rename(problems: *mut git_strarray,
                             remote: *mut git_remote,
                             new_name: *const c_char) -> c_int;
    pub fn git_remote_fetch(remote: *mut git_remote,
                            signature: *const git_signature,
                            reflog_message: *const c_char) -> c_int;
    pub fn git_remote_update_tips(remote: *mut git_remote,
                                  signature: *const git_signature,
                                  reflog_message: *const c_char) -> c_int;
    pub fn git_remote_update_fetchhead(remote: *mut git_remote) -> c_int;
    pub fn git_remote_set_url(remote: *mut git_remote,
                              url: *const c_char) -> c_int;
    pub fn git_remote_set_pushurl(remote: *mut git_remote,
                                  pushurl: *const c_char) -> c_int;
    pub fn git_remote_set_update_fetchhead(remote: *mut git_remote,
                                           update: c_int);
    pub fn git_remote_set_fetch_refspecs(remote: *mut git_remote,
                                         array: *mut git_strarray) -> c_int;
    pub fn git_remote_set_push_refspecs(remote: *mut git_remote,
                                        array: *mut git_strarray) -> c_int;

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

    // clone
    pub fn git_clone(out: *mut *mut git_repository,
                     url: *const c_char,
                     local_path: *const c_char,
                     options: *const git_clone_options) -> c_int;
    pub fn git_clone_init_options(opts: *mut git_clone_options,
                                  version: c_uint) -> c_int;
}
