use std::marker;

use crate::util::Binding;
use crate::{raw, Buf, Error, Odb, Repository};

/// A structure to represent a mempack backend for the object database. The
/// Mempack is bound to the Odb that it was created from, and cannot outlive
/// that Odb.
pub struct Mempack<'odb> {
    raw: *mut raw::git_odb_backend,
    _marker: marker::PhantomData<&'odb Odb<'odb>>,
}

impl<'odb> Binding for Mempack<'odb> {
    type Raw = *mut raw::git_odb_backend;

    unsafe fn from_raw(raw: *mut raw::git_odb_backend) -> Mempack<'odb> {
        Mempack {
            raw,
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *mut raw::git_odb_backend {
        self.raw
    }
}

// We don't need to implement `Drop` for Mempack because it is owned by the
// odb to which it is attached, and that will take care of freeing the mempack
// and associated memory.

impl<'odb> Mempack<'odb> {
    /// Dumps the contents of the mempack into the provided buffer.
    pub fn dump(&self, repo: &Repository, buf: &mut Buf) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_mempack_dump(buf.raw(), repo.raw(), self.raw));
        }
        Ok(())
    }

    /// Clears all data in the mempack.
    pub fn reset(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_mempack_reset(self.raw));
        }
        Ok(())
    }
}
