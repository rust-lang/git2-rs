use std::ffi::{CStr, NulError};
use std::error;
use std::fmt;
use std::str;
use libc::c_int;

use {raw, ErrorCode};

/// A structure to represent errors coming out of libgit2.
#[derive(Debug)]
pub struct Error {
    klass: c_int,
    message: String,
}

impl Error {
    /// Returns the last error, or `None` if one is not available.
    pub fn last_error() -> Option<Error> {
        ::init();
        unsafe {
            let ptr = raw::giterr_last();
            if ptr.is_null() {
                None
            } else {
                Some(Error::from_raw(ptr))
            }
        }
    }

    unsafe fn from_raw(ptr: *const raw::git_error) -> Error {
        let msg = CStr::from_ptr((*ptr).message as *const _).to_bytes();
        let msg = str::from_utf8(msg).unwrap();
        Error { klass: (*ptr).klass, message: msg.to_string() }
    }

    /// Creates a new error from the given string as the error.
    pub fn from_str(s: &str) -> Error {
        Error { klass: raw::GIT_ERROR as c_int, message: s.to_string() }
    }

    /// Return the error code associated with this error.
    pub fn code(&self) -> ErrorCode {
        match self.raw_code() {
            raw::GIT_OK => super::ErrorCode::GenericError,
            raw::GIT_ERROR => super::ErrorCode::GenericError,
            raw::GIT_ENOTFOUND => super::ErrorCode::NotFound,
            raw::GIT_EEXISTS => super::ErrorCode::Exists,
            raw::GIT_EAMBIGUOUS => super::ErrorCode::Ambiguous,
            raw::GIT_EBUFS => super::ErrorCode::BufSize,
            raw::GIT_EUSER => super::ErrorCode::User,
            raw::GIT_EBAREREPO => super::ErrorCode::BareRepo,
            raw::GIT_EUNBORNBRANCH => super::ErrorCode::UnbornBranch,
            raw::GIT_EUNMERGED => super::ErrorCode::Unmerged,
            raw::GIT_ENONFASTFORWARD => super::ErrorCode::NotFastForward,
            raw::GIT_EINVALIDSPEC => super::ErrorCode::InvalidSpec,
            raw::GIT_EMERGECONFLICT => super::ErrorCode::MergeConflict,
            raw::GIT_ELOCKED => super::ErrorCode::Locked,
            raw::GIT_EMODIFIED => super::ErrorCode::Modified,
            raw::GIT_PASSTHROUGH => super::ErrorCode::GenericError,
            raw::GIT_ITEROVER => super::ErrorCode::GenericError,
        }
    }

    /// Return the raw error code associated with this error.
    pub fn raw_code(&self) -> raw::git_error_code {
        macro_rules! check( ($($e:ident),*) => (
            $(if self.klass == raw::$e as c_int { raw::$e }) else *
            else {
                raw::GIT_ERROR
            }
        ) );
        check!(
            GIT_OK,
            GIT_ERROR,
            GIT_ENOTFOUND,
            GIT_EEXISTS,
            GIT_EAMBIGUOUS,
            GIT_EBUFS,
            GIT_EUSER,
            GIT_EBAREREPO,
            GIT_EUNBORNBRANCH,
            GIT_EUNMERGED,
            GIT_ENONFASTFORWARD,
            GIT_EINVALIDSPEC,
            GIT_EMERGECONFLICT,
            GIT_ELOCKED,
            GIT_EMODIFIED,
            GIT_PASSTHROUGH,
            GIT_ITEROVER
        )
    }

    /// Return the message associated with this error
    pub fn message(&self) -> &str { &self.message }
}

impl error::Error for Error {
    fn description(&self) -> &str { &self.message }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "[{}] ", self.klass));
        f.write_str(&self.message)
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Error {
        Error::from_str("data contained a nul byte that could not be \
                         represented as a string")
    }
}
