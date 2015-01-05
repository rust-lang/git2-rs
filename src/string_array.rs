use std::str;

use raw;

/// A string array structure used by libgit2
///
/// Some apis return arrays of strings which originate from libgit2. This
/// wrapper type behaves a little like `Vec<&str>` but does so without copying
/// the underlying strings until necessary.
pub struct StringArray {
    raw: raw::git_strarray,
}

/// A forward iterator over the strings of an array, casted to `&str`.
pub struct StringArrayItems<'a> {
    cur: uint,
    arr: &'a StringArray,
}

/// A forward iterator over the strings of an array, casted to `&[u8]`.
pub struct StringArrayBytes<'a> {
    cur: uint,
    arr: &'a StringArray,
}

impl StringArray {
    /// Creates a new string array from the raw representation.
    ///
    /// This is unsafe because it consumes ownership of the array and there is
    /// no guarantee that the array itself is valid or that no one else is using
    /// it.
    pub unsafe fn from_raw(raw: raw::git_strarray) -> StringArray {
        StringArray { raw: raw }
    }

    /// Returns None if the i'th string is not utf8 or if i is out of bounds.
    pub fn get(&self, i: uint) -> Option<&str> {
        self.get_bytes(i).and_then(|s| str::from_utf8(s).ok())
    }

    /// Returns None if `i` is out of bounds.
    pub fn get_bytes(&self, i: uint) -> Option<&[u8]> {
        if i < self.raw.count as uint {
            unsafe {
                let ptr = *self.raw.strings.offset(i as int) as *const _;
                Some(::opt_bytes(self, ptr).unwrap())
            }
        } else {
            None
        }
    }

    /// Returns an iterator over the strings contained within this array.
    ///
    /// The iterator yields `Option<&str>` as it is unknown whether the contents
    /// are utf-8 or not.
    pub fn iter(&self) -> StringArrayItems {
        StringArrayItems { cur: 0, arr: self }
    }

    /// Returns an iterator over the strings contained within this array,
    /// yielding byte slices.
    pub fn iter_bytes(&self) -> StringArrayBytes {
        StringArrayBytes { cur: 0, arr: self }
    }

    /// Returns the number of strings in this array.
    pub fn len(&self) -> uint { self.raw.count as uint }
}

impl<'a> Iterator for StringArrayItems<'a> {
    type Item = Option<&'a str>;
    fn next(&mut self) -> Option<Option<&'a str>> {
        if self.cur < self.arr.len() {
            self.cur += 1;
            Some(self.arr.get(self.cur - 1))
        } else {
            None
        }
    }
}

impl<'a> Iterator for StringArrayBytes<'a> {
    type Item = &'a [u8];
    fn next(&mut self) -> Option<&'a [u8]> {
        if self.cur < self.arr.len() {
            self.cur += 1;
            self.arr.get_bytes(self.cur - 1)
        } else {
            None
        }
    }
}

impl Drop for StringArray {
    fn drop(&mut self) {
        unsafe { raw::git_strarray_free(&mut self.raw) }
    }
}
