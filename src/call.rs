#![macro_escape]
use libc;

use Error;

macro_rules! call( ($p:path($($e:expr),*)) => (
    $p($(::call::convert(&$e)),*)
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
        n if n < 0 => Err(Error::last_error().unwrap()),
        n => Ok(n),
    }
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
                ::Push => raw::GIT_DIRECTION_PUSH,
                ::Fetch => raw::GIT_DIRECTION_FETCH,
            }
        }
    }

    impl Convert<raw::git_otype> for ::ObjectKind {
        fn convert(&self) -> raw::git_otype {
            match *self {
                ::Any => raw::GIT_OBJ_ANY,
                ::Commit => raw::GIT_OBJ_COMMIT,
                ::Tree => raw::GIT_OBJ_TREE,
                ::Blob => raw::GIT_OBJ_BLOB,
                ::Tag => raw::GIT_OBJ_TAG,
            }
        }
    }

    impl Convert<raw::git_otype> for Option<::ObjectKind> {
        fn convert(&self) -> raw::git_otype {
            self.unwrap_or(::Any).convert()
        }
    }
}
