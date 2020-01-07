use libc::c_int;
use std::env::JoinPathsError;
use std::error;
use std::ffi::{CStr, NulError};
use std::fmt;
use std::str;

use crate::{raw, ErrorClass, ErrorCode};

/// A structure to represent errors coming out of libgit2.
#[derive(Debug, PartialEq)]
pub struct Error {
    code: c_int,
    klass: c_int,
    message: String,
}

impl Error {
    /// Returns the last error that happened with the code specified by `code`.
    ///
    /// The `code` argument typically comes from the return value of a function
    /// call. This code will later be returned from the `code` function.
    ///
    /// Historically this function returned `Some` or `None` based on the return
    /// value of `git_error_last` but nowadays it always returns `Some` so it's
    /// safe to unwrap the return value. This API will change in the next major
    /// version.
    pub fn last_error(code: c_int) -> Option<Error> {
        crate::init();
        unsafe {
            // Note that whenever libgit2 returns an error any negative value
            // indicates that an error happened. Auxiliary information is
            // *usually* in `git_error_last` but unfortunately that's not always
            // the case. Sometimes a negative error code is returned from
            // libgit2 *without* calling `git_error_set` internally to configure
            // the error.
            //
            // To handle this case and hopefully provide better error messages
            // on our end we unconditionally call `git_error_clear` when we're done
            // with an error. This is an attempt to clear it as aggressively as
            // possible when we can to ensure that error information from one
            // api invocation doesn't leak over to the next api invocation.
            //
            // Additionally if `git_error_last` returns null then we returned a
            // canned error out.
            let ptr = raw::git_error_last();
            let err = if ptr.is_null() {
                let mut error = Error::from_str("an unknown git error occurred");
                error.code = code;
                error
            } else {
                Error::from_raw(code, ptr)
            };
            raw::git_error_clear();
            Some(err)
        }
    }

    unsafe fn from_raw(code: c_int, ptr: *const raw::git_error) -> Error {
        let msg = CStr::from_ptr((*ptr).message as *const _).to_bytes();
        let msg = String::from_utf8_lossy(msg).into_owned();
        Error {
            code: code,
            klass: (*ptr).klass,
            message: msg,
        }
    }

    /// Creates a new error from the given string as the error.
    ///
    /// The error returned will have the code `GIT_ERROR` and the class
    /// `GIT_ERROR_NONE`.
    pub fn from_str(s: &str) -> Error {
        Error {
            code: raw::GIT_ERROR as c_int,
            klass: raw::GIT_ERROR_NONE as c_int,
            message: s.to_string(),
        }
    }

    /// Return the error code associated with this error.
    ///
    /// An error code is intended to be programmatically actionable most of the
    /// time. For example the code `GIT_EAGAIN` indicates that an error could be
    /// fixed by trying again, while the code `GIT_ERROR` is more bland and
    /// doesn't convey anything in particular.
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
            raw::GIT_ECONFLICT => super::ErrorCode::Conflict,
            raw::GIT_ELOCKED => super::ErrorCode::Locked,
            raw::GIT_EMODIFIED => super::ErrorCode::Modified,
            raw::GIT_PASSTHROUGH => super::ErrorCode::GenericError,
            raw::GIT_ITEROVER => super::ErrorCode::GenericError,
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
    ///
    /// Error classes are in general mostly just informative. For example the
    /// class will show up in the error message but otherwise an error class is
    /// typically not directly actionable.
    pub fn class(&self) -> ErrorClass {
        match self.raw_class() {
            raw::GIT_ERROR_NONE => super::ErrorClass::None,
            raw::GIT_ERROR_NOMEMORY => super::ErrorClass::NoMemory,
            raw::GIT_ERROR_OS => super::ErrorClass::Os,
            raw::GIT_ERROR_INVALID => super::ErrorClass::Invalid,
            raw::GIT_ERROR_REFERENCE => super::ErrorClass::Reference,
            raw::GIT_ERROR_ZLIB => super::ErrorClass::Zlib,
            raw::GIT_ERROR_REPOSITORY => super::ErrorClass::Repository,
            raw::GIT_ERROR_CONFIG => super::ErrorClass::Config,
            raw::GIT_ERROR_REGEX => super::ErrorClass::Regex,
            raw::GIT_ERROR_ODB => super::ErrorClass::Odb,
            raw::GIT_ERROR_INDEX => super::ErrorClass::Index,
            raw::GIT_ERROR_OBJECT => super::ErrorClass::Object,
            raw::GIT_ERROR_NET => super::ErrorClass::Net,
            raw::GIT_ERROR_TAG => super::ErrorClass::Tag,
            raw::GIT_ERROR_TREE => super::ErrorClass::Tree,
            raw::GIT_ERROR_INDEXER => super::ErrorClass::Indexer,
            raw::GIT_ERROR_SSL => super::ErrorClass::Ssl,
            raw::GIT_ERROR_SUBMODULE => super::ErrorClass::Submodule,
            raw::GIT_ERROR_THREAD => super::ErrorClass::Thread,
            raw::GIT_ERROR_STASH => super::ErrorClass::Stash,
            raw::GIT_ERROR_CHECKOUT => super::ErrorClass::Checkout,
            raw::GIT_ERROR_FETCHHEAD => super::ErrorClass::FetchHead,
            raw::GIT_ERROR_MERGE => super::ErrorClass::Merge,
            raw::GIT_ERROR_SSH => super::ErrorClass::Ssh,
            raw::GIT_ERROR_FILTER => super::ErrorClass::Filter,
            raw::GIT_ERROR_REVERT => super::ErrorClass::Revert,
            raw::GIT_ERROR_CALLBACK => super::ErrorClass::Callback,
            raw::GIT_ERROR_CHERRYPICK => super::ErrorClass::CherryPick,
            raw::GIT_ERROR_DESCRIBE => super::ErrorClass::Describe,
            raw::GIT_ERROR_REBASE => super::ErrorClass::Rebase,
            raw::GIT_ERROR_FILESYSTEM => super::ErrorClass::Filesystem,
            raw::GIT_ERROR_PATCH => super::ErrorClass::Patch,
            raw::GIT_ERROR_WORKTREE => super::ErrorClass::Worktree,
            raw::GIT_ERROR_SHA1 => super::ErrorClass::Sha1,
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
            GIT_RETRY,
            GIT_EMISMATCH,
            GIT_EINDEXDIRTY,
            GIT_EAPPLYFAIL,
        )
    }

    /// Return the raw error class associated with this error.
    pub fn raw_class(&self) -> raw::git_error_t {
        macro_rules! check( ($($e:ident,)*) => (
            $(if self.klass == raw::$e as c_int { raw::$e }) else *
            else {
                raw::GIT_ERROR_NONE
            }
        ) );
        check!(
            GIT_ERROR_NONE,
            GIT_ERROR_NOMEMORY,
            GIT_ERROR_OS,
            GIT_ERROR_INVALID,
            GIT_ERROR_REFERENCE,
            GIT_ERROR_ZLIB,
            GIT_ERROR_REPOSITORY,
            GIT_ERROR_CONFIG,
            GIT_ERROR_REGEX,
            GIT_ERROR_ODB,
            GIT_ERROR_INDEX,
            GIT_ERROR_OBJECT,
            GIT_ERROR_NET,
            GIT_ERROR_TAG,
            GIT_ERROR_TREE,
            GIT_ERROR_INDEXER,
            GIT_ERROR_SSL,
            GIT_ERROR_SUBMODULE,
            GIT_ERROR_THREAD,
            GIT_ERROR_STASH,
            GIT_ERROR_CHECKOUT,
            GIT_ERROR_FETCHHEAD,
            GIT_ERROR_MERGE,
            GIT_ERROR_SSH,
            GIT_ERROR_FILTER,
            GIT_ERROR_REVERT,
            GIT_ERROR_CALLBACK,
            GIT_ERROR_CHERRYPICK,
            GIT_ERROR_DESCRIBE,
            GIT_ERROR_REBASE,
            GIT_ERROR_FILESYSTEM,
            GIT_ERROR_PATCH,
            GIT_ERROR_WORKTREE,
            GIT_ERROR_SHA1,
        )
    }

    /// Return the message associated with this error
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        match self.class() {
            ErrorClass::None => {}
            other => write!(f, "; class={:?} ({})", other, self.klass)?,
        }
        match self.code() {
            ErrorCode::GenericError => {}
            other => write!(f, "; code={:?} ({})", other, self.code)?,
        }
        Ok(())
    }
}

impl From<NulError> for Error {
    fn from(_: NulError) -> Error {
        Error::from_str(
            "data contained a nul byte that could not be \
             represented as a string",
        )
    }
}

impl From<JoinPathsError> for Error {
    fn from(e: JoinPathsError) -> Error {
        Error::from_str(&e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use crate::{ErrorClass, ErrorCode};

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();

        let err = repo.find_submodule("does_not_exist").err().unwrap();
        assert_eq!(err.code(), ErrorCode::NotFound);
        assert_eq!(err.class(), ErrorClass::Submodule);
    }
}
