#![macro_escape]
use libc;

use Error;

macro_rules! call( (raw::$p:ident ($($e:expr),*)) => (
    raw::$p($(::call::convert(&$e)),*)
) )

macro_rules! try_call( ($($arg:tt)*) => ({
    try!(::call::try(call!($($arg)*)))
}) )

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

    use raw;
    use call::Convert;

    macro_rules! primitive( ($($p:ident)*) => (
        $(impl Convert<$p> for $p { fn convert(&self) -> $p { *self } })*
    ) )

    primitive!(i8 i16 i32 i64 int u8 u16 u32 u64 uint)

    macro_rules! peel(
        ($macro:ident, ) => ();
        ($macro:ident, $_a:ident $($arg:ident)*) => ($macro!($($arg)*))
    )

    macro_rules! externfn( ($($arg:ident)*) => (
        impl<R $(,$arg)*> Convert<extern fn($($arg),*) -> R>
            for extern fn($($arg),*) -> R
        {
            fn convert(&self) -> extern fn($($arg),*) -> R { *self }
        }
        impl<R $(,$arg)*> Convert<Option<extern fn($($arg),*) -> R>>
            for Option<extern fn($($arg),*) -> R>
        {
            fn convert(&self) -> Option<extern fn($($arg),*) -> R> { *self }
        }
        peel!(externfn, $($arg)*)
    ) )
    externfn!(A B C D E F G)

    impl Convert<libc::c_int> for bool {
        fn convert(&self) -> libc::c_int { *self as libc::c_int }
    }
    impl<T> Convert<*const T> for *const T {
        fn convert(&self) -> *const T { *self }
    }
    impl<T> Convert<*mut T> for *mut T {
        fn convert(&self) -> *mut T { *self }
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

    impl Convert<raw::git_reset_t> for ::ResetType {
        fn convert(&self) -> raw::git_reset_t {
            match *self {
                ::Soft => raw::GIT_RESET_SOFT,
                ::Hard => raw::GIT_RESET_HARD,
                ::Mixed => raw::GIT_RESET_MIXED,
            }
        }
    }

    impl Convert<raw::git_direction> for ::Direction {
        fn convert(&self) -> raw::git_direction {
            match *self {
                ::DirPush => raw::GIT_DIRECTION_PUSH,
                ::DirFetch => raw::GIT_DIRECTION_FETCH,
            }
        }
    }

    impl Convert<raw::git_otype> for ::ObjectType {
        fn convert(&self) -> raw::git_otype {
            match *self {
                ::ObjectAny => raw::GIT_OBJ_ANY,
                ::ObjectCommit => raw::GIT_OBJ_COMMIT,
                ::ObjectTree => raw::GIT_OBJ_TREE,
                ::ObjectBlob => raw::GIT_OBJ_BLOB,
                ::ObjectTag => raw::GIT_OBJ_TAG,
            }
        }
    }

    impl Convert<raw::git_otype> for Option<::ObjectType> {
        fn convert(&self) -> raw::git_otype {
            self.unwrap_or(::ObjectAny).convert()
        }
    }

    impl Convert<raw::git_branch_t> for ::BranchType {
        fn convert(&self) -> raw::git_branch_t {
            match *self {
                ::BranchRemote => raw::GIT_BRANCH_REMOTE,
                ::BranchLocal => raw::GIT_BRANCH_LOCAL,
            }
        }
    }

    impl Convert<raw::git_branch_t> for Option<::BranchType> {
        fn convert(&self) -> raw::git_branch_t {
            self.map(|s| s.convert()).unwrap_or(raw::GIT_BRANCH_ALL)
        }
    }

    impl Convert<raw::git_config_level_t> for ::ConfigLevel {
        fn convert(&self) -> raw::git_config_level_t {
            match *self {
                ::ConfigSystem => raw::GIT_CONFIG_LEVEL_SYSTEM,
                ::ConfigXDG => raw::GIT_CONFIG_LEVEL_XDG,
                ::ConfigGlobal => raw::GIT_CONFIG_LEVEL_GLOBAL,
                ::ConfigLocal => raw::GIT_CONFIG_LEVEL_LOCAL,
                ::ConfigApp => raw::GIT_CONFIG_LEVEL_APP,
                ::ConfigHighest => raw::GIT_CONFIG_HIGHEST_LEVEL,
            }
        }
    }
}
