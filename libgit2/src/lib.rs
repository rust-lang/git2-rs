#![feature(phase)]
#![allow(non_camel_case_types)]

#[phase(plugin)]
extern crate "link-config" as link_config;

extern crate "openssl-static-sys" as openssl_static_sys;
extern crate libc;

use libc::{c_int, c_char, c_uint, size_t, c_uchar, c_void, c_ushort};

pub const GIT_OID_RAWSZ: uint = 20;
pub const GIT_OID_HEXSZ: uint = GIT_OID_RAWSZ * 2;
pub const GIT_CLONE_OPTIONS_VERSION: c_uint = 1;
pub const GIT_CHECKOUT_OPTIONS_VERSION: c_uint = 1;
pub const GIT_REMOTE_CALLBACKS_VERSION: c_uint = 1;

pub enum git_blob {}
pub enum git_branch_iterator {}
pub enum git_commit {}
pub enum git_config {}
pub enum git_config_iterator {}
pub enum git_index {}
pub enum git_object {}
pub enum git_reference {}
pub enum git_reference_iterator {}
pub enum git_refspec {}
pub enum git_remote {}
pub enum git_repository {}
pub enum git_submodule {}
pub enum git_tag {}
pub enum git_tree {}
pub enum git_tree_entry {}
pub enum git_push {}

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
    #[repr(C)]
    flags git_revparse_mode_t: c_uint {
        const GIT_REVPARSE_SINGLE = 1 << 0,
        const GIT_REVPARSE_RANGE = 1 << 1,
        const GIT_REVPARSE_MERGE_BASE = 1 << 2,
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

#[repr(C)]
pub enum git_reset_t {
    GIT_RESET_SOFT = 1,
    GIT_RESET_MIXED = 2,
    GIT_RESET_HARD = 3,
}

#[repr(C)]
pub enum git_otype {
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

#[repr(C)]
pub enum git_ref_t {
    GIT_REF_INVALID = 0,
    GIT_REF_OID = 1,
    GIT_REF_SYMBOLIC = 2,
    GIT_REF_LISTALL = GIT_REF_OID as int | GIT_REF_SYMBOLIC as int,
}

#[repr(C)]
pub enum git_filemode_t {
    GIT_FILEMODE_UNREADABLE          = 0000000,
    GIT_FILEMODE_TREE                = 0040000,
    GIT_FILEMODE_BLOB                = 0100644,
    GIT_FILEMODE_BLOB_EXECUTABLE     = 0100755,
    GIT_FILEMODE_LINK                = 0120000,
    GIT_FILEMODE_COMMIT              = 0160000,
}

#[repr(C)]
pub enum git_treewalk_mode {
    GIT_TREEWALK_PRE = 0,
    GIT_TREEWALK_POST = 1,
}

pub type git_treewalk_cb = extern fn(*const c_char, *const git_tree_entry,
                                     *mut c_void) -> c_int;

#[repr(C)]
pub struct git_buf {
    pub ptr: *mut c_char,
    pub asize: size_t,
    pub size: size_t,
}

#[repr(C)]
pub enum git_branch_t {
    GIT_BRANCH_LOCAL = 1,
    GIT_BRANCH_REMOTE = 2,
    GIT_BRANCH_ALL = GIT_BRANCH_LOCAL as int | GIT_BRANCH_REMOTE as int,
}

pub type git_index_matched_path_cb = extern fn(*const c_char, *const c_char,
                                               *mut c_void) -> c_int;

#[repr(C)]
pub struct git_index_entry {
    pub ctime: git_index_time,
    pub mtime: git_index_time,
    pub dev: c_uint,
    pub ino: c_uint,
    pub mode: c_uint,
    pub uid: c_uint,
    pub gid: c_uint,
    pub file_size: git_off_t,
    pub id: git_oid,
    pub flags: c_ushort,
    pub flags_extended: c_ushort,
    pub path: *const c_char,
}

#[repr(C)]
pub struct git_index_time {
    pub seconds: git_time_t,
    pub nanoseconds: c_uint,
}

#[repr(C)]
pub struct git_config_entry {
    pub name: *const c_char,
    pub value: *const c_char,
    pub level: git_config_level_t,
}

#[repr(C)]
pub enum git_config_level_t {
    GIT_CONFIG_LEVEL_SYSTEM = 1,
    GIT_CONFIG_LEVEL_XDG = 2,
    GIT_CONFIG_LEVEL_GLOBAL = 3,
    GIT_CONFIG_LEVEL_LOCAL = 4,
    GIT_CONFIG_LEVEL_APP = 5,
    GIT_CONFIG_HIGHEST_LEVEL = -1,
}

#[repr(C)]
pub enum git_submodule_update_t {
    GIT_SUBMODULE_UPDATE_RESET    = -1,
    GIT_SUBMODULE_UPDATE_CHECKOUT = 1,
    GIT_SUBMODULE_UPDATE_REBASE   = 2,
    GIT_SUBMODULE_UPDATE_MERGE    = 3,
    GIT_SUBMODULE_UPDATE_NONE     = 4,
    GIT_SUBMODULE_UPDATE_DEFAULT  = 0
}

#[repr(C)]
pub enum git_submodule_ignore_t {
    GIT_SUBMODULE_IGNORE_RESET     = -1,

    GIT_SUBMODULE_IGNORE_NONE      = 1,
    GIT_SUBMODULE_IGNORE_UNTRACKED = 2,
    GIT_SUBMODULE_IGNORE_DIRTY     = 3,
    GIT_SUBMODULE_IGNORE_ALL       = 4,

    GIT_SUBMODULE_IGNORE_DEFAULT   = 0
}

#[repr(C)]
pub struct git_cred {
    pub credtype: git_credtype_t,
    pub free: extern fn(*mut git_cred),
}

#[repr(C)]
pub enum git_credtype_t {
    GIT_CREDTYPE_USERPASS_PLAINTEXT = 1 << 0,
    GIT_CREDTYPE_SSH_KEY = 1 << 1,
    GIT_CREDTYPE_SSH_CUSTOM = 1 << 2,
    GIT_CREDTYPE_DEFAULT = 1 << 3,
    GIT_CREDTYPE_SSH_INTERACTIVE = 1 << 4,
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
}

pub type git_tag_foreach_cb = extern fn(name: *const c_char,
                                        oid: *mut git_oid,
                                        payload: *mut c_void) -> c_int;

#[repr(C)]
pub enum git_index_add_option_t {
    GIT_INDEX_ADD_DEFAULT = 0,
    GIT_INDEX_ADD_FORCE = 1 << 0,
    GIT_INDEX_ADD_DISABLE_PATHSPEC_MATCH = 1 << 1,
    GIT_INDEX_ADD_CHECK_PATHSPEC = 1 << 2,
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

#[repr(C)]
pub enum git_repository_init_flag_t {
    GIT_REPOSITORY_INIT_BARE              = (1 << 0),
    GIT_REPOSITORY_INIT_NO_REINIT         = (1 << 1),
    GIT_REPOSITORY_INIT_NO_DOTGIT_DIR     = (1 << 2),
    GIT_REPOSITORY_INIT_MKDIR             = (1 << 3),
    GIT_REPOSITORY_INIT_MKPATH            = (1 << 4),
    GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE = (1 << 5),
}

#[repr(C)]
pub enum git_repository_init_mode_t {
    GIT_REPOSITORY_INIT_SHARED_UMASK = 0,
    GIT_REPOSITORY_INIT_SHARED_GROUP = 0o002775,
    GIT_REPOSITORY_INIT_SHARED_ALL   = 0o002777,
}

// We cannot rely on pkg-config on Windows, but it isn't like we need it anyway
#[cfg(not(windows))]
link_config!("libgit2", ["only_static"])

#[cfg(windows)]
#[link(name = "winhttp")] // needed by git2
#[link(name = "rpcrt4")]  // needed by git2
#[link(name = "ole32")]   // needed by git2
#[link(name = "ws2_32")]  // needed by ssh2
#[link(name = "bcrypt")]  // needed by ssh2
#[link(name = "crypt32")] // needed by ssh2
#[link(name = "ssh2", kind = "static")]
#[link(name = "git2", kind = "static")]
extern {}

/// Initialize openssl for the libgit2 library
pub fn openssl_init() {
    if !cfg!(target_os = "linux") && !cfg!(target_os = "freebsd") { return }

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
    openssl_static_sys::init_ssl_cert_env_vars();
}

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
    pub fn git_repository_is_bare(repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_empty(repo: *mut git_repository) -> c_int;
    pub fn git_repository_is_shallow(repo: *mut git_repository) -> c_int;
    pub fn git_repository_path(repo: *mut git_repository) -> *const c_char;
    pub fn git_repository_state(repo: *mut git_repository) -> c_int;
    pub fn git_repository_workdir(repo: *mut git_repository) -> *const c_char;
    pub fn git_repository_index(out: *mut *mut git_index,
                                repo: *mut git_repository) -> c_int;
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
    pub fn git_remote_set_callbacks(remote: *mut git_remote,
                                    callbacks: *const git_remote_callbacks)
                                    -> c_int;
    pub fn git_remote_init_callbacks(opts: *mut git_remote_callbacks,
                                     version: c_uint) -> c_int;

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

    // reset
    pub fn git_reset(repo: *mut git_repository,
                     target: *mut git_object,
                     reset_type: git_reset_t,
                     signature: *mut git_signature,
                     log_message: *const c_char) -> c_int;
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
    pub fn git_reference_rename(new_ref: *mut *mut git_reference,
                                r: *mut git_reference,
                                new_name: *const c_char,
                                force: c_int,
                                sig: *const git_signature,
                                log_message: *const c_char) -> c_int;
    pub fn git_reference_resolve(out: *mut *mut git_reference,
                                 r: *const git_reference) -> c_int;
    pub fn git_reference_shorthand(r: *const git_reference) -> *const c_char;
    pub fn git_reference_symbolic_target(r: *const git_reference) -> *const c_char;
    pub fn git_reference_target(r: *const git_reference) -> *const git_oid;
    pub fn git_reference_target_peel(r: *const git_reference) -> *const git_oid;
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
                                sig: *const git_signature,
                                log_message: *const c_char) -> c_int;
    pub fn git_reference_symbolic_create(out: *mut *mut git_reference,
                                         repo: *mut git_repository,
                                         name: *const c_char,
                                         target: *const c_char,
                                         force: c_int,
                                         sig: *const git_signature,
                                         log_message: *const c_char) -> c_int;

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
                                 callback: extern fn(*mut git_submodule,
                                                     *const c_char,
                                                     *mut c_void) -> c_int,
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
    pub fn git_submodule_reload_all(repo: *mut git_repository,
                                    force: c_int) -> c_int;
    pub fn git_submodule_save(submodule: *mut git_submodule) -> c_int;
    pub fn git_submodule_set_ignore(submodule: *mut git_submodule,
                                    ignore: git_submodule_ignore_t)
                                    -> git_submodule_ignore_t;
    pub fn git_submodule_set_update(submodule: *mut git_submodule,
                                    update: git_submodule_update_t)
                                    -> git_submodule_update_t;
    pub fn git_submodule_set_url(submodule: *mut git_submodule,
                                 url: *const c_char) -> c_int;
    pub fn git_submodule_sync(submodule: *mut git_submodule) -> c_int;
    pub fn git_submodule_update(submodule: *mut git_submodule)
                                -> git_submodule_update_t;
    pub fn git_submodule_url(submodule: *mut git_submodule) -> *const c_char;
    pub fn git_submodule_wd_id(submodule: *mut git_submodule) -> *const git_oid;

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
                           commit: *const git_commit) -> c_uint;
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
                             parents: *const *const git_commit) -> c_int;

    // branch
    pub fn git_branch_create(out: *mut *mut git_reference,
                             repo: *mut git_repository,
                             branch_name: *const c_char,
                             target: *const git_commit,
                             force: c_int,
                             signature: *const git_signature,
                             log_message: *const c_char) -> c_int;
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
                           force: c_int,
                           signature: *const git_signature,
                           log_message: *const c_char) -> c_int;
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
    pub fn git_config_find_global(out: *mut git_buf) -> c_int;
    pub fn git_config_find_system(out: *mut git_buf) -> c_int;
    pub fn git_config_find_xdg(out: *mut git_buf) -> c_int;
    pub fn git_config_free(cfg: *mut git_config);
    pub fn git_config_get_bool(out: *mut c_int,
                               cfg: *const git_config,
                               name: *const c_char) -> c_int;
    pub fn git_config_get_entry(out: *mut *const git_config_entry,
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
    pub fn git_config_refresh(cfg: *mut git_config) -> c_int;
    pub fn git_config_set_bool(cfg: *mut git_config,
                               name: *const c_char,
                               value: c_int) -> c_int;
    pub fn git_config_set_int32(cfg: *mut git_config,
                                name: *const c_char,
                                value: i32) -> c_int;
    pub fn git_config_set_int64(cfg: *mut git_config,
                                name: *const c_char,
                                value: i64) -> c_int;
    pub fn git_config_set_string(cfg: *mut git_config,
                                 name: *const c_char,
                                 value: *const c_char) -> c_int;
    pub fn git_config_snapshot(out: *mut *mut git_config,
                               config: *mut git_config) -> c_int;

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
    pub fn git_cred_userpass(cred: *mut *mut git_cred,
                             url: *const c_char,
                             user_from_url: *const c_char,
                             allowed_types: c_uint,
                             payload: *mut c_void) -> c_int;
    pub fn git_cred_userpass_plaintext_new(out: *mut *mut git_cred,
                                           username: *const c_char,
                                           password: *const c_char) -> c_int;

    // push
    pub fn git_push_add_refspec(push: *mut git_push,
                                refspec: *const c_char) -> c_int;
    pub fn git_push_finish(push: *mut git_push) -> c_int;
    pub fn git_push_free(push: *mut git_push);
    pub fn git_push_init_options(opts: *mut git_push_options,
                                 version: c_uint) -> c_int;
    pub fn git_push_new(out: *mut *mut git_push,
                        remote: *mut git_remote) -> c_int;
    pub fn git_push_set_options(push: *mut git_push,
                                opts: *const git_push_options) -> c_int;
    pub fn git_push_unpack_ok(push: *const git_push) -> c_int;
    pub fn git_push_update_tips(push: *mut git_push,
                                signature: *const git_signature,
                                reflog_message: *const c_char) -> c_int;
    pub fn git_push_status_foreach(push: *mut git_push,
                                   cb: extern fn(*const c_char,
                                                 *const c_char,
                                                 *mut c_void) -> c_int,
                                   data: *mut c_void) -> c_int;

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
}
