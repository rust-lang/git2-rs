//! Bindings to libgit2's raw `git_strarray` type

use std::iter::FusedIterator;
use std::ops::Range;
use std::str;

use crate::raw;
use crate::util::Binding;

/// A string array structure used by libgit2
///
/// Some APIs return arrays of strings which originate from libgit2. This
/// wrapper type behaves a little like `Vec<&str>` but does so without copying
/// the underlying strings until necessary.
pub struct StringArray {
    raw: raw::git_strarray,
}

/// A forward iterator over the strings of an array, casted to `&str`.
pub struct Iter<'a> {
    range: Range<usize>,
    arr: &'a StringArray,
}

/// A forward iterator over the strings of an array, casted to `&[u8]`.
pub struct IterBytes<'a> {
    range: Range<usize>,
    arr: &'a StringArray,
}

impl StringArray {
    /// Returns None if the i'th string is not utf8 or if i is out of bounds.
    pub fn get(&self, i: usize) -> Option<&str> {
        self.get_bytes(i).and_then(|s| str::from_utf8(s).ok())
    }

    /// Returns None if `i` is out of bounds.
    pub fn get_bytes(&self, i: usize) -> Option<&[u8]> {
        if i < self.raw.count as usize {
            unsafe {
                let ptr = *self.raw.strings.add(i) as *const _;
                Some(crate::opt_bytes(self, ptr).unwrap())
            }
        } else {
            None
        }
    }

    /// Returns an iterator over the strings contained within this array.
    ///
    /// The iterator yields `Option<&str>` as it is unknown whether the contents
    /// are utf-8 or not.
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            range: 0..self.len(),
            arr: self,
        }
    }

    /// Returns an iterator over the strings contained within this array,
    /// yielding byte slices.
    pub fn iter_bytes(&self) -> IterBytes<'_> {
        IterBytes {
            range: 0..self.len(),
            arr: self,
        }
    }

    /// Returns the number of strings in this array.
    pub fn len(&self) -> usize {
        self.raw.count as usize
    }

    /// Return `true` if this array is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl Binding for StringArray {
    type Raw = raw::git_strarray;
    unsafe fn from_raw(raw: raw::git_strarray) -> StringArray {
        StringArray { raw }
    }
    fn raw(&self) -> raw::git_strarray {
        self.raw
    }
}

impl<'a> IntoIterator for &'a StringArray {
    type Item = Option<&'a str>;
    type IntoIter = Iter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Option<&'a str>;
    fn next(&mut self) -> Option<Option<&'a str>> {
        self.range.next().map(|i| self.arr.get(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}
impl<'a> DoubleEndedIterator for Iter<'a> {
    fn next_back(&mut self) -> Option<Option<&'a str>> {
        self.range.next_back().map(|i| self.arr.get(i))
    }
}
impl<'a> FusedIterator for Iter<'a> {}
impl<'a> ExactSizeIterator for Iter<'a> {}

impl<'a> Iterator for IterBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        self.range.next().and_then(|i| self.arr.get_bytes(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}
impl<'a> DoubleEndedIterator for IterBytes<'a> {
    fn next_back(&mut self) -> Option<&'a [u8]> {
        self.range.next_back().and_then(|i| self.arr.get_bytes(i))
    }
}
impl<'a> FusedIterator for IterBytes<'a> {}
impl<'a> ExactSizeIterator for IterBytes<'a> {}

impl Drop for StringArray {
    fn drop(&mut self) {
        unsafe { raw::git_strarray_free(&mut self.raw) }
    }
}
