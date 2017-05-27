use std::env::JoinPathsError;
use std::ffi::{CStr, NulError};
use std::error;
use std::fmt;
use std::str;
use libc::c_int;

use {raw, ErrorClass, ErrorCode};

/// A structure to represent errors coming out of libgit2.
#[derive(Debug,PartialEq)]
pub struct Error {
    code: c_int,
    klass: c_int,
    message: String,
}

impl Error {
    /// Returns the last error, or `None` if one is not available.
    pub fn last_error(code: c_int) -> Option<Error> {
        ::init();
        unsafe {
            let ptr = raw::giterr_last();
            if ptr.is_null() {
                None
            } else {
                Some(Error::from_raw(code, ptr))
            }
        }
    }

    unsafe fn from_raw(code: c_int, ptr: *const raw::git_error) -> Error {
        let msg = CStr::from_ptr((*ptr).message as *const _).to_bytes();
        let msg = str::from_utf8(msg).unwrap();
        Error { code: code, klass: (*ptr).klass, message: msg.to_string() }
    }

    /// Creates a new error from the given string as the error.
    pub fn from_str(s: &str) -> Error {
        Error {
            code: raw::GIT_ERROR as c_int,
            klass: raw::GITERR_NONE as c_int,
            message: s.to_string(),
        }
    }

    /// Return the error code associated with this error.
    pub fn code(&self) -> ErrorCode {
        match self.raw_code() {
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
            raw::GIT_ECONFLICT => super::ErrorCode::Conflict,
            raw::GIT_ELOCKED => super::ErrorCode::Locked,
            raw::GIT_EMODIFIED => super::ErrorCode::Modified,
            raw::GIT_EAUTH => super::ErrorCode::Auth,
            raw::GIT_ECERTIFICATE => super::ErrorCode::Certificate,
            raw::GIT_EAPPLIED => super::ErrorCode::Applied,
            raw::GIT_EPEEL => super::ErrorCode::Peel,
            raw::GIT_EEOF => super::ErrorCode::Eof,
            raw::GIT_EINVALID => super::ErrorCode::Invalid,
            raw::GIT_EUNCOMMITTED => super::ErrorCode::Uncommitted,
            raw::GIT_EDIRECTORY => super::ErrorCode::Directory,
            _ => super::ErrorCode::GenericError,
        }
    }

    /// Return the error class associated with this error.
    pub fn class(&self) -> ErrorClass {
        match self.raw_class() {
            raw::GITERR_NOMEMORY => super::ErrorClass::NoMemory,
            raw::GITERR_OS => super::ErrorClass::Os,
            raw::GITERR_INVALID => super::ErrorClass::Invalid,
            raw::GITERR_REFERENCE => super::ErrorClass::Reference,
            raw::GITERR_ZLIB => super::ErrorClass::Zlib,
            raw::GITERR_REPOSITORY => super::ErrorClass::Repository,
            raw::GITERR_CONFIG => super::ErrorClass::Config,
            raw::GITERR_REGEX => super::ErrorClass::Regex,
            raw::GITERR_ODB => super::ErrorClass::Odb,
            raw::GITERR_INDEX => super::ErrorClass::Index,
            raw::GITERR_OBJECT => super::ErrorClass::Object,
            raw::GITERR_NET => super::ErrorClass::Net,
            raw::GITERR_TAG => super::ErrorClass::Tag,
            raw::GITERR_TREE => super::ErrorClass::Tree,
            raw::GITERR_INDEXER => super::ErrorClass::Indexer,
            raw::GITERR_SSL => super::ErrorClass::Ssl,
            raw::GITERR_SUBMODULE => super::ErrorClass::Submodule,
            raw::GITERR_THREAD => super::ErrorClass::Thread,
            raw::GITERR_STASH => super::ErrorClass::Stash,
            raw::GITERR_CHECKOUT => super::ErrorClass::Checkout,
            raw::GITERR_FETCHHEAD => super::ErrorClass::FetchHead,
            raw::GITERR_MERGE => super::ErrorClass::Merge,
            raw::GITERR_SSH => super::ErrorClass::Ssh,
            raw::GITERR_FILTER => super::ErrorClass::Filter,
            raw::GITERR_REVERT => super::ErrorClass::Revert,
            raw::GITERR_CALLBACK => super::ErrorClass::Callback,
            raw::GITERR_CHERRYPICK => super::ErrorClass::CherryPick,
            raw::GITERR_DESCRIBE => super::ErrorClass::Describe,
            raw::GITERR_REBASE => super::ErrorClass::Rebase,
            raw::GITERR_FILESYSTEM => super::ErrorClass::Filesystem,
            _ => super::ErrorClass::None,
        }
    }

    /// Return the raw error code associated with this error.
    pub fn raw_code(&self) -> raw::git_error_code {
        macro_rules! check( ($($e:ident,)*) => (
            $(if self.code == raw::$e as c_int { raw::$e }) else *
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
            GIT_ECONFLICT,
            GIT_ELOCKED,
            GIT_EMODIFIED,
            GIT_EAUTH,
            GIT_ECERTIFICATE,
            GIT_EAPPLIED,
            GIT_EPEEL,
            GIT_EEOF,
            GIT_EINVALID,
            GIT_EUNCOMMITTED,
            GIT_PASSTHROUGH,
            GIT_ITEROVER,
        )
    }

    /// Return the raw error class associated with this error.
    pub fn raw_class(&self) -> raw::git_error_t {
        macro_rules! check( ($($e:ident,)*) => (
            $(if self.klass == raw::$e as c_int { raw::$e }) else *
            else {
                raw::GITERR_NONE
            }
        ) );
        check!(
            GITERR_NONE,
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
        try!(write!(f, "[{}/{}] ", self.klass, self.code));
        f.write_str(&self.message)
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Error {
        Error::from_str("data contained a nul byte that could not be \
                         represented as a string")
    }
}

impl From<JoinPathsError> for Error {
    fn from(e: JoinPathsError) -> Error {
        Error::from_str(error::Error::description(&e))
    }
}


#[cfg(test)]
mod tests {
    use {ErrorClass, ErrorCode};

    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();

        let err = repo.find_submodule("does_not_exist").err().unwrap();
        assert_eq!(err.code(), ErrorCode::NotFound);
        assert_eq!(err.class(), ErrorClass::Submodule);
    }
}
