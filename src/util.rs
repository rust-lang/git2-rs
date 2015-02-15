use std::ffi::{CString, AsOsStr, OsStr, OsString};
use std::path::Path as NewPath;
use std::path::PathBuf;
use libc::{c_char, size_t};

use raw;

#[doc(hidden)]
pub trait Binding: Sized {
    type Raw;

    unsafe fn from_raw(raw: Self::Raw) -> Self;
    fn raw(&self) -> Self::Raw;

    unsafe fn from_raw_opt<T>(raw: T) -> Option<Self>
        where T: PtrExt + Copy, Self: Binding<Raw=T>
    {
        if raw.is_null() {
            None
        } else {
            Some(Binding::from_raw(raw))
        }
    }
}

pub fn iter2cstrs<T, I>(iter: I) -> (Vec<CString>, Vec<*const c_char>,
                                     raw::git_strarray)
    where T: IntoCString, I: Iterator<Item=T>
{
    let cstrs = iter.map(|i| i.into_c_string()).collect::<Vec<_>>();
    let ptrs = cstrs.iter().map(|i| i.as_ptr()).collect::<Vec<_>>();
    let raw = raw::git_strarray {
        strings: ptrs.as_ptr() as *mut _,
        count: ptrs.len() as size_t,
    };
    (cstrs, ptrs, raw)
}

/// A class of types that can be converted to C strings.
///
/// These types are represented internally as byte slices and it is quite rare
/// for them to contain an interior 0 byte.
pub trait IntoCString {
    /// Consume this container, converting it into a CString
    fn into_c_string(self) -> CString;
}

impl<'a, T: IntoCString + Clone> IntoCString for &'a T {
    fn into_c_string(self) -> CString {
        self.clone().into_c_string()
    }
}

impl<'a> IntoCString for &'a str {
    fn into_c_string(self) -> CString { CString::from_slice(self.as_bytes()) }
}

impl IntoCString for String {
    fn into_c_string(self) -> CString {
        CString::from_vec(self.into_bytes())
    }
}

impl IntoCString for CString {
    fn into_c_string(self) -> CString { self }
}

impl IntoCString for Path {
    fn into_c_string(self) -> CString { CString::from_vec(self.into_vec()) }
}

impl<'a> IntoCString for &'a NewPath {
    fn into_c_string(self) -> CString { self.as_os_str().into_c_string() }
}

impl IntoCString for PathBuf {
    fn into_c_string(self) -> CString { self.as_os_str().into_c_string() }
}

impl<'a> IntoCString for &'a OsStr {
    fn into_c_string(self) -> CString { self.to_os_string().into_c_string() }
}

impl IntoCString for OsString {
    #[cfg(unix)]
    fn into_c_string(self) -> CString {
        use std::os::unix::OsStrExt;
        CString::from_slice(self.as_os_str().as_bytes())
    }
    #[cfg(windows)]
    fn into_c_string(self) -> CString {
        CString::from_slice(self.to_str().expect("only valid unicode paths \
                                                  are accepted on windows")
                                .as_bytes())
    }
}
