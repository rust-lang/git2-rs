//! Bindings to libgit2's raw `git_oidarray` type

use std::ops::Deref;

use crate::oid::Oid;
use crate::raw;
use crate::util::Binding;
use std::mem;
use std::slice;

/// An oid array structure used by libgit2
///
/// Some APIs return arrays of OIDs which originate from libgit2. This
/// wrapper type behaves a little like `Vec<&Oid>` but does so without copying
/// the underlying Oids until necessary.
pub struct OidArray {
    raw: raw::git_oidarray,
}

impl Deref for OidArray {
    type Target = [Oid];

    fn deref(&self) -> &[Oid] {
        unsafe {
            debug_assert_eq!(mem::size_of::<Oid>(), mem::size_of_val(&*self.raw.ids));

            slice::from_raw_parts(self.raw.ids as *const Oid, self.raw.count as usize)
        }
    }
}

impl Binding for OidArray {
    type Raw = raw::git_oidarray;
    unsafe fn from_raw(raw: raw::git_oidarray) -> OidArray {
        OidArray { raw }
    }
    fn raw(&self) -> raw::git_oidarray {
        self.raw
    }
}

impl<'repo> std::fmt::Debug for OidArray {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_tuple("OidArray").field(&self.deref()).finish()
    }
}

impl Drop for OidArray {
    fn drop(&mut self) {
        unsafe { raw::git_oidarray_free(&mut self.raw) }
    }
}
