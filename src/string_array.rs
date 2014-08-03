use std::str;

use raw;

pub struct StringArray {
    raw: raw::git_strarray,
}

pub struct StringArrayItems<'a> {
    cur: uint,
    arr: &'a StringArray,
}

pub struct StringArrayBytes<'a> {
    cur: uint,
    arr: &'a StringArray,
}

impl StringArray {
    pub unsafe fn from_raw(raw: raw::git_strarray) -> StringArray {
        StringArray { raw: raw }
    }

    /// Returns None if the i'th string is not utf8 or if i is out of bounds.
    pub fn get(&self, i: uint) -> Option<&str> {
        self.get_bytes(i).and_then(str::from_utf8)
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

impl<'a> Iterator<Option<&'a str>> for StringArrayItems<'a> {
    fn next(&mut self) -> Option<Option<&'a str>> {
        if self.cur < self.arr.len() {
            self.cur += 1;
            Some(self.arr.get(self.cur - 1))
        } else {
            None
        }
    }
}

impl<'a> Iterator<&'a [u8]> for StringArrayBytes<'a> {
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
