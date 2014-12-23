#![macro_escape]
use libc;

use Error;

macro_rules! call( (raw::$p:ident ($($e:expr),*)) => (
    raw::$p($(::call::convert(&$e)),*)
) );

macro_rules! try_call( ($($arg:tt)*) => ({
    try!(::call::try(call!($($arg)*)))
}) );

#[doc(hidden)]
pub trait Convert<T> {
    fn convert(&self) -> T;
}

pub fn convert<T, U: Convert<T>>(u: &U) -> T { u.convert() }

pub fn try(ret: libc::c_int) -> Result<libc::c_int, Error> {
    match ret {
        n if n < 0 => Err(last_error()),
        n => Ok(n),
    }
}

fn last_error() -> Error {
    // Apparently libgit2 isn't necessarily guaranteed to set the last error
    // whenever a function returns a negative value!
    Error::last_error().unwrap_or_else(|| {
        Error::from_str("an unknown error occurred")
    })
}

mod impls {
    use std::c_str::CString;
    use libc;

    use {raw, ConfigLevel, ResetType, ObjectType, BranchType, Direction};
    use call::Convert;

    impl<T: Copy> Convert<T> for T {
        fn convert(&self) -> T { *self }
    }

    impl Convert<libc::c_int> for bool {
        fn convert(&self) -> libc::c_int { *self as libc::c_int }
    }
    impl<'a, T> Convert<*const T> for &'a T {
        fn convert(&self) -> *const T { *self as *const T }
    }
    impl<'a, T> Convert<*mut T> for &'a mut T {
        fn convert(&self) -> *mut T { &**self as *const T as *mut T }
    }

    impl Convert<*const libc::c_char> for CString {
        fn convert(&self) -> *const libc::c_char { self.as_ptr() }
    }

    impl Convert<*const libc::c_char> for Option<CString> {
        fn convert(&self) -> *const libc::c_char {
            self.as_ref().map(|s| s.convert()).unwrap_or(0 as *const _)
        }
    }

    impl<'a, T> Convert<*const T> for Option<&'a T> {
        fn convert(&self) -> *const T {
            self.as_ref().map(|s| s.convert()).unwrap_or(0 as *const _)
        }
    }

    impl<'a, T> Convert<*mut T> for Option<&'a mut T> {
        fn convert(&self) -> *mut T {
            self.as_ref().map(|s| s.convert()).unwrap_or(0 as *mut _)
        }
    }

    impl Convert<raw::git_reset_t> for ResetType {
        fn convert(&self) -> raw::git_reset_t {
            match *self {
                ResetType::Soft => raw::GIT_RESET_SOFT,
                ResetType::Hard => raw::GIT_RESET_HARD,
                ResetType::Mixed => raw::GIT_RESET_MIXED,
            }
        }
    }

    impl Convert<raw::git_direction> for Direction {
        fn convert(&self) -> raw::git_direction {
            match *self {
                Direction::Push => raw::GIT_DIRECTION_PUSH,
                Direction::Fetch => raw::GIT_DIRECTION_FETCH,
            }
        }
    }

    impl Convert<raw::git_otype> for ObjectType {
        fn convert(&self) -> raw::git_otype {
            match *self {
                ObjectType::Any => raw::GIT_OBJ_ANY,
                ObjectType::Commit => raw::GIT_OBJ_COMMIT,
                ObjectType::Tree => raw::GIT_OBJ_TREE,
                ObjectType::Blob => raw::GIT_OBJ_BLOB,
                ObjectType::Tag => raw::GIT_OBJ_TAG,
            }
        }
    }

    impl Convert<raw::git_otype> for Option<ObjectType> {
        fn convert(&self) -> raw::git_otype {
            self.unwrap_or(ObjectType::Any).convert()
        }
    }

    impl Convert<raw::git_branch_t> for BranchType {
        fn convert(&self) -> raw::git_branch_t {
            match *self {
                BranchType::Remote => raw::GIT_BRANCH_REMOTE,
                BranchType::Local => raw::GIT_BRANCH_LOCAL,
            }
        }
    }

    impl Convert<raw::git_branch_t> for Option<BranchType> {
        fn convert(&self) -> raw::git_branch_t {
            self.map(|s| s.convert()).unwrap_or(raw::GIT_BRANCH_ALL)
        }
    }

    impl Convert<raw::git_config_level_t> for ConfigLevel {
        fn convert(&self) -> raw::git_config_level_t {
            match *self {
                ConfigLevel::System => raw::GIT_CONFIG_LEVEL_SYSTEM,
                ConfigLevel::XDG => raw::GIT_CONFIG_LEVEL_XDG,
                ConfigLevel::Global => raw::GIT_CONFIG_LEVEL_GLOBAL,
                ConfigLevel::Local => raw::GIT_CONFIG_LEVEL_LOCAL,
                ConfigLevel::App => raw::GIT_CONFIG_LEVEL_APP,
                ConfigLevel::Highest => raw::GIT_CONFIG_HIGHEST_LEVEL,
            }
        }
    }
}
