//! Bindings to libgit2's raw git_strarray type

use std::ops::{Range,Deref};

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

/// A forward iterator over the Oids of an array, casted to `&Oid`.
pub struct Iter<'a> {
    range: Range<usize>,
    arr: &'a OidArray,
}

impl OidArray {
    /// Returns None if i is out of bounds.
    pub fn get(&self, i: usize) -> Option<Oid> {
        if i < self.raw.count as usize {
            Some(self[i])
        } else {
            None
        }
    }

    /// Returns an iterator over the Oids contained within this array.
    pub fn iter(&self) -> Iter {
        Iter { range: 0..self.len(), arr: self }
    }

    /// Returns the number of strings in this array.
    pub fn len(&self) -> usize { self.raw.count as usize }
}

impl Deref for OidArray {
    type Target = [Oid];

    fn deref(&self) -> &[Oid] {
        unsafe {
            slice::from_raw_parts(self.raw.ids as *const Oid, self.raw.count as usize)
        }
    }
}

impl Binding for OidArray {
    type Raw = raw::git_oidarray;
    unsafe fn from_raw(raw: raw::git_oidarray) -> OidArray {
        OidArray { raw: raw }
    }
    fn raw(&self) -> raw::git_oidarray { self.raw }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Oid;
    fn next(&mut self) -> Option<Oid> {
        self.range.next().and_then(|i| self.arr.get(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}
impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Oid> {
        self.range.next_back().and_then(|i| self.arr.get(i))
    }
}
impl<'a> ExactSizeIterator for Iter<'a> {}

impl Drop for OidArray {
    fn drop(&mut self) {
        unsafe { raw::git_oidarray_free(&mut self.raw) }
    }
}
