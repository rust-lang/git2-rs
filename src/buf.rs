use std::ops::{Deref, DerefMut};
use std::ptr;
use std::slice;
use std::str;

use crate::raw;
use crate::util::Binding;

/// A structure to wrap an intermediate buffer used by libgit2.
///
/// A buffer can be thought of a `Vec<u8>`, but the `Vec` type is not used to
/// avoid copying data back and forth.
pub struct Buf {
    raw: raw::git_buf,
}

impl Default for Buf {
    fn default() -> Self {
        Self::new()
    }
}

impl Buf {
    /// Creates a new empty buffer.
    pub fn new() -> Buf {
        crate::init();
        unsafe {
            Binding::from_raw(&mut raw::git_buf {
                ptr: ptr::null_mut(),
                size: 0,
                reserved: 0,
            } as *mut _)
        }
    }

    /// Attempt to view this buffer as a string slice.
    ///
    /// Returns `None` if the buffer is not valid utf-8.
    pub fn as_str(&self) -> Option<&str> {
        str::from_utf8(&**self).ok()
    }
}

impl Deref for Buf {
    type Target = [u8];
    fn deref(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.raw.ptr as *const u8, self.raw.size as usize) }
    }
}

impl DerefMut for Buf {
    fn deref_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.raw.ptr as *mut u8, self.raw.size as usize) }
    }
}

impl Binding for Buf {
    type Raw = *mut raw::git_buf;
    unsafe fn from_raw(raw: *mut raw::git_buf) -> Buf {
        Buf { raw: *raw }
    }
    fn raw(&self) -> *mut raw::git_buf {
        &self.raw as *const _ as *mut _
    }
}

impl Drop for Buf {
    fn drop(&mut self) {
        unsafe { raw::git_buf_dispose(&mut self.raw) }
    }
}
