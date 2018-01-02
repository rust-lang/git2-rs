use {raw, Buf, Error, Repository};

use std::ptr;

use util::Binding;
use libc;

/// A structure to represent a git object database backend.
pub struct OdbBackendHolder {
    raw: *mut raw::git_odb_backend
}

impl Binding for OdbBackendHolder {
    type Raw = *mut raw::git_odb_backend;

    unsafe fn from_raw(raw: *mut raw::git_odb_backend) -> OdbBackendHolder {
        OdbBackendHolder {
            raw: raw
        }
    }

    fn raw(&self) -> *mut raw::git_odb_backend { self.raw }
}

impl OdbBackendHolder {
    /// Creates an object database backend for a loose object directory.
    pub fn loose(objects_dir: &str, compression_level: i32,
                 do_fsync: bool, dir_mode: u32, file_mode: u32) -> Result<OdbBackendHolder, Error> {
        ::init();
        unsafe {
            let mut out = ptr::null_mut();
            try_call!(raw::git_odb_backend_loose(
                &mut out as *mut _,
                objects_dir.as_ptr() as *const libc::c_char,
                compression_level,
                do_fsync,
                dir_mode,
                file_mode
            ));

            Ok(OdbBackendHolder::from_raw(out as *mut raw::git_odb_backend))
        }
    }

    /// Creates an object database backend that uses pack in a given directory.
    pub fn pack(objects_dir: &str) -> Result<OdbBackendHolder, Error> {
        ::init();
        unsafe {
            let mut out = ptr::null_mut();
            try_call!(raw::git_odb_backend_pack(
                &mut out as *mut _,
                objects_dir.as_ptr() as *const libc::c_char
            ));

            Ok(OdbBackendHolder::from_raw(out as *mut raw::git_odb_backend))
        }
    }

    /// Creates an object database backend from a single pack.
    pub fn one_pack(index_file: &str) -> Result<OdbBackendHolder, Error> {
        ::init();
        unsafe {
            let mut out = ptr::null_mut();
            try_call!(raw::git_odb_backend_one_pack(
                &mut out as *mut _,
                index_file.as_ptr() as *const libc::c_char
            ));

            Ok(OdbBackendHolder::from_raw(out as *mut raw::git_odb_backend))
        }
    }

    /// Resets the mempack backend.
    /// NOTE: Only to be used on mempack backends.
    pub fn mempack_reset(&self) {
        unsafe {
            raw::git_mempack_reset(self.raw);
        }
    }

    /// Dumps the generated pack into the given buffer.
    /// NOTE: Only to be used on mempack backends.
    pub fn mempack_dump(&self, buf: &Buf, repository: &Repository) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_mempack_dump(
                buf.raw(),
                repository.raw(),
                self.raw
            ));

            Ok(())
        }
    }
}
