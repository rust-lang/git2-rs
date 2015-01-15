use std::ffi::CString;
use std::path::BytesContainer;
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
    where T: BytesContainer, I: Iterator<Item=T>
{
    let cstrs = iter.map(|i| CString::from_slice(i.container_as_bytes()))
                    .collect::<Vec<_>>();
    let ptrs = cstrs.iter().map(|i| i.as_ptr()).collect::<Vec<_>>();
    let raw = raw::git_strarray {
        strings: ptrs.as_ptr() as *mut _,
        count: ptrs.len() as size_t,
    };
    (cstrs, ptrs, raw)
}
