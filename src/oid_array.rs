//! Bindings to libgit2's raw `git_oidarray` type

use std::ops::Deref;

use oid::Oid;
use raw;
use util::Binding;
use std::slice;

/// An oid array structure used by libgit2
///
/// Some apis return arrays of oids which originate from libgit2. This
/// wrapper type behaves a little like `Vec<&Oid>` but does so without copying
/// the underlying Oids until necessary.
pub struct OidArray {
    raw: raw::git_oidarray,
}

impl OidArray {
    /// Get the length of this array.
    pub fn len(&self) -> usize {
        self.raw.count as usize
    }

    /// Get an iterator over the `Oid`s in this array.
    pub fn iter<'a>(&'a self) -> OidIter<'a> {
        self.into_iter()
    }

    fn as_raw_slice(&self) -> &[raw::git_oid] {
        unsafe { slice::from_raw_parts(self.raw.ids, self.raw.count as usize) }
    }

}

/// An iterator over `OidArray`.
pub struct OidIter<'a> {
    arr: &'a OidArray,
    idx: usize,
}

impl<'a> ::std::iter::Iterator for OidIter<'a> {
    type Item = Oid;
    fn next(&mut self) -> Option<Oid> {
        if self.idx >= self.arr.raw.count {
            return None;
        }
        let arr: &[raw::git_oid] = self.arr.as_raw_slice();
        let ret = unsafe { Oid::from_raw(&arr[self.idx] as * const _) };
        self.idx += 1;

        Some(ret)
    }
}

impl<'a> ::std::iter::IntoIterator for &'a OidArray {
    type Item = Oid;
    type IntoIter = OidIter<'a>;

    fn into_iter(self) -> OidIter<'a> {
        OidIter { arr: self, idx: 0 }
    }
}

impl Binding for OidArray {
    type Raw = raw::git_oidarray;
    unsafe fn from_raw(raw: raw::git_oidarray) -> OidArray {
        OidArray { raw: raw }
    }
    fn raw(&self) -> raw::git_oidarray { self.raw }
}

impl<'repo> ::std::fmt::Debug for OidArray {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> Result<(), ::std::fmt::Error> {
		f.debug_tuple("OidArray").field(&self.deref()).finish()
    }
}

impl Drop for OidArray {
    fn drop(&mut self) {
        unsafe { raw::git_oidarray_free(&mut self.raw) }
    }
}
