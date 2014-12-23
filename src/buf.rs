use std::mem;
use std::str;
use std::raw as stdraw;
use libc;

use raw;

/// A structure to wrap an intermediate buffer used by libgit2.
///
/// A buffer can be thought of a `Vec<u8>`, but the `Vec` type is not used to
/// avoid copying data back and forth.
pub struct Buf {
    raw: raw::git_buf,
}

impl Buf {
    /// Creates a new empty buffer.
    pub fn new() -> Buf {
        ::init();
        Buf { raw: raw::git_buf {
            ptr: 0 as *mut libc::c_char,
            size: 0,
            asize: 0,
        } }
    }

    /// Creates a buffer from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that the pointers inside
    /// the buffer are valid.
    pub unsafe fn from_raw(raw: raw::git_buf) -> Buf {
        ::init();
        Buf { raw: raw }
    }

    /// Attempt to view this buffer as a string slice.
    ///
    /// Returns `None` if the buffer is not valid utf-8.
    pub fn as_str(&self) -> Option<&str> { str::from_utf8(self.get()).ok() }

    /// Gain access to the contents of this buffer as a byte slice
    pub fn get(&self) -> &[u8] {
        unsafe {
            mem::transmute(stdraw::Slice {
                data: self.raw.ptr as *const u8,
                len: self.raw.size as uint,
            })
        }
    }

    /// Gain access to the underlying raw buffer.
    pub fn raw(&mut self) -> *mut raw::git_buf { &mut self.raw as *mut _ }
}

impl Drop for Buf {
    fn drop(&mut self) {
        unsafe { raw::git_buf_free(&mut self.raw) }
    }
}
