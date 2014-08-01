use std::fmt;
use std::c_str::CString;
use libc;
use libc::c_int;

use {raw, ErrorCode};

/// A structure to represent errors coming out of libgit2.
pub struct Error {
    raw: raw::git_error,
}

impl Error {
    /// Returns the last error, or `None` if one is not available.
    pub fn last_error() -> Option<Error> {
        let mut ret = Error {
            raw: raw::git_error {
                message: 0 as *mut libc::c_char,
                klass: 0,
            }
        };
        if unsafe { raw::giterr_detach(&mut ret.raw) } == 0 {
            Some(ret)
        } else {
            None
        }
    }

    pub fn from_str(s: &'static str) -> Error {
        assert_eq!(*s.as_bytes().last().unwrap(), 0);
        Error {
            raw: raw::git_error {
                message: s.as_bytes().as_ptr() as *mut libc::c_char,
                klass: raw::GIT_ERROR as libc::c_int,
            }
        }
    }

    /// Return the error code associated with this error.
    pub fn code(&self) -> ErrorCode {
        match self.raw_code() {
            raw::GIT_OK => super::Error,
            raw::GIT_ERROR => super::Error,
            raw::GIT_ENOTFOUND => super::NotFound,
            raw::GIT_EEXISTS => super::Exists,
            raw::GIT_EAMBIGUOUS => super::Ambiguous,
            raw::GIT_EBUFS => super::BufSize,
            raw::GIT_EUSER => super::User,
            raw::GIT_EBAREREPO => super::BareRepo,
            raw::GIT_EUNBORNBRANCH => super::UnbornBranch,
            raw::GIT_EUNMERGED => super::Unmerged,
            raw::GIT_ENONFASTFORWARD => super::NotFastForward,
            raw::GIT_EINVALIDSPEC => super::InvalidSpec,
            raw::GIT_EMERGECONFLICT => super::MergeConflict,
            raw::GIT_ELOCKED => super::Locked,
            raw::GIT_EMODIFIED => super::Modified,
            raw::GIT_PASSTHROUGH => super::Error,
            raw::GIT_ITEROVER => super::Error,
        }
    }

    /// Return the raw error code associated with this error.
    pub fn raw_code(&self) -> raw::git_error_code {
        macro_rules! check( ($($e:ident),*) => (
            $(if self.raw.klass == raw::$e as c_int { raw::$e }) else *
            else {
                raw::GIT_ERROR
            }
        ) )
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
    pub fn message(&self) -> String {
        let cstr = unsafe { CString::new(self.raw.message as *const _, false) };
        String::from_utf8_lossy(cstr.as_bytes_no_nul()).to_string()
    }
}

impl fmt::Show for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        try!(write!(f, "[{}] ", self.raw.klass));
        let cstr = unsafe { CString::new(self.raw.message as *const _, false) };
        f.write(cstr.as_bytes_no_nul())
    }
}
