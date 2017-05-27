use std::path::Path;
use std::ptr;
use libc::{c_char, c_int, c_void};

use {raw, Blob, Buf, Diff, DiffDelta, DiffHunk, DiffLine, DiffOptions, Error};
use diff::{LineCb, print_cb};
use util::{Binding, into_opt_c_string};

/// A structure representing the text changes in a single diff delta.
///
/// This is an opaque structure.
pub struct Patch {
    raw: *mut raw::git_patch,
}

unsafe impl Send for Patch {}

impl Binding for Patch {
    type Raw = *mut raw::git_patch;
    unsafe fn from_raw(raw: Self::Raw) -> Patch {
        Patch { raw: raw }
    }
    fn raw(&self) -> Self::Raw { self.raw }
}

impl Drop for Patch {
    fn drop(&mut self) {
        unsafe { raw::git_patch_free(self.raw) }
    }
}

impl Patch {
    /// Return a Patch for one file in a Diff.
    ///
    /// Returns Ok(None) for an unchanged or binary file.
    pub fn from_diff(diff: &Diff, idx: usize) -> Result<Option<Patch>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_patch_from_diff(&mut ret, diff.raw(), idx));
            Ok(Binding::from_raw_opt(ret))
        }
    }

    /// Generate a Patch by diffing two blobs.
    pub fn from_blobs(old_blob: &Blob,
                      old_path: Option<&Path>,
                      new_blob: &Blob,
                      new_path: Option<&Path>,
                      opts: Option<&mut DiffOptions>)
                      -> Result<Patch, Error>
    {
        let mut ret = ptr::null_mut();
        let old_path = try!(into_opt_c_string(old_path));
        let new_path = try!(into_opt_c_string(new_path));
        unsafe {
            try_call!(raw::git_patch_from_blobs(&mut ret,
                                                old_blob.raw(),
                                                old_path,
                                                new_blob.raw(),
                                                new_path,
                                                opts.map(|s| s.raw())));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Generate a Patch by diffing a blob and a buffer.
    pub fn from_blob_and_buffer(old_blob: &Blob,
                                old_path: Option<&Path>,
                                new_buffer: &[u8],
                                new_path: Option<&Path>,
                                opts: Option<&mut DiffOptions>)
                                -> Result<Patch, Error>
    {
        let mut ret = ptr::null_mut();
        let old_path = try!(into_opt_c_string(old_path));
        let new_path = try!(into_opt_c_string(new_path));
        unsafe {
            try_call!(raw::git_patch_from_blob_and_buffer(&mut ret,
                                                          old_blob.raw(),
                                                          old_path,
                                                          new_buffer.as_ptr() as *const c_char,
                                                          new_buffer.len(),
                                                          new_path,
                                                          opts.map(|s| s.raw())));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Generate a Patch by diffing two buffers.
    pub fn from_buffers(old_buffer: &[u8],
                        old_path: Option<&Path>,
                        new_buffer: &[u8],
                        new_path: Option<&Path>,
                        opts: Option<&mut DiffOptions>)
                        -> Result<Patch, Error>
    {
        let mut ret = ptr::null_mut();
        let old_path = try!(into_opt_c_string(old_path));
        let new_path = try!(into_opt_c_string(new_path));
        unsafe {
            try_call!(raw::git_patch_from_buffers(&mut ret,
                                                 old_buffer.as_ptr() as *const c_void,
                                                 old_buffer.len(),
                                                 old_path,
                                                 new_buffer.as_ptr() as *const c_char,
                                                 new_buffer.len(),
                                                 new_path,
                                                 opts.map(|s| s.raw())));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get the DiffDelta associated with the Patch.
    pub fn delta(&self) -> DiffDelta {
        unsafe {
            Binding::from_raw(raw::git_patch_get_delta(self.raw) as *mut _)
        }
    }

    /// Get the number of hunks in the Patch.
    pub fn num_hunks(&self) -> usize {
        unsafe {
            raw::git_patch_num_hunks(self.raw)
        }
    }

    /// Get the number of lines of context, additions, and deletions in the Patch.
    pub fn line_stats(&self) -> Result<(usize, usize, usize), Error> {
        let mut context = 0;
        let mut additions = 0;
        let mut deletions = 0;
        unsafe {
            try_call!(raw::git_patch_line_stats(&mut context,
                                                &mut additions,
                                                &mut deletions,
                                                self.raw));
        }
        Ok((context, additions, deletions))
    }

    /// Get a DiffHunk and its total line count from the Patch.
    pub fn hunk(&mut self, hunk_idx: usize) -> Result<(DiffHunk, usize), Error> {
        let mut ret = ptr::null();
        let mut lines = 0;
        unsafe {
            try_call!(raw::git_patch_get_hunk(&mut ret, &mut lines, self.raw, hunk_idx));
            Ok((Binding::from_raw(ret), lines))
        }
    }

    /// Get the number of lines in a hunk.
    pub fn num_lines_in_hunk(&self, hunk_idx: usize) -> Result<usize, Error> {
        unsafe {
            Ok(try_call!(raw::git_patch_num_lines_in_hunk(self.raw, hunk_idx)) as usize)
        }
    }

    /// Get a DiffLine from a hunk of the Patch.
    pub fn line_in_hunk(&mut self,
                        hunk_idx: usize,
                        line_of_hunk: usize) -> Result<DiffLine, Error> {
        let mut ret = ptr::null();
        unsafe {
            try_call!(raw::git_patch_get_line_in_hunk(&mut ret,
                                                      self.raw,
                                                      hunk_idx,
                                                      line_of_hunk));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get the size of a Patch's diff data in bytes.
    pub fn size(&self,
                include_context: bool,
                include_hunk_headers: bool,
                include_file_headers: bool) -> usize {
        unsafe {
            raw::git_patch_size(self.raw,
                                include_context as c_int,
                                include_hunk_headers as c_int,
                                include_file_headers as c_int)
        }
    }

    /// Print the Patch to text via a callback.
    pub fn print(&mut self, mut line_cb: &mut LineCb) -> Result<(), Error> {
        let ptr = &mut line_cb as *mut _ as *mut c_void;
        unsafe {
            try_call!(raw::git_patch_print(self.raw, print_cb, ptr));
            return Ok(())
        }
    }

    /// Get the Patch text as a Buf.
    pub fn to_buf(&mut self) -> Result<Buf, Error> {
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_patch_to_buf(buf.raw(), self.raw));
        }
        Ok(buf)
    }
}
