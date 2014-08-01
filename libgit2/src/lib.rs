extern crate libc;
extern crate openssl;

use libc::{c_int, c_char, c_uint, size_t};

pub static GIT_OID_RAWSZ: uint = 20;
pub static GIT_OID_HEXSZ: uint = GIT_OID_RAWSZ * 2;

pub enum git_repository {}
pub enum git_object {}
pub enum git_tag {}

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

bitflags!(
    flags git_revparse_mode_t: c_uint {
        static GIT_REVPARSE_SINGLE = 1 << 0,
        static GIT_REVPARSE_RANGE = 1 << 1,
        static GIT_REVPARSE_MERGE_BASE = 1 << 2
    }
)

/** Generic return codes */
#[repr(C)]
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

#[link(name = "git2", kind = "static")]
#[link(name = "z")]
extern {
    pub fn git_threads_init() -> c_int;
    pub fn git_threads_shutdown();

    pub fn git_repository_free(repo: *mut git_repository);
    pub fn git_repository_open(repo: *mut *mut git_repository,
                               path: *const c_char) -> c_int;
    pub fn git_repository_init(repo: *mut *mut git_repository,
                               path: *const c_char,
                               is_bare: c_uint) -> c_int;

    pub fn git_revparse(revspec: *mut git_revspec,
                        repo: *mut git_repository,
                        spec: *const c_char) -> c_int;

    pub fn git_object_dup(dest: *mut *mut git_object,
                          source: *mut git_object) -> c_int;
    pub fn git_object_id(obj: *const git_object) -> *const git_oid;
    pub fn git_object_free(object: *mut git_object);

    pub fn git_oid_tostr(out: *mut c_char, n: size_t,
                         id: *const git_oid) -> *mut c_char;
    pub fn git_oid_cmp(a: *const git_oid, b: *const git_oid) -> c_int;
    pub fn git_oid_equal(a: *const git_oid, b: *const git_oid) -> c_int;

    pub fn giterr_last() -> *const git_error;
    pub fn giterr_clear();
    pub fn giterr_detach(cpy: *mut git_error) -> c_int;
}
