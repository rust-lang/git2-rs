use std::cmp::Ordering;

use libc::{c_char, c_int};

use crate::raw;
use crate::util::Binding;

/// Time in a signature
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct Time {
    raw: raw::git_time,
}

/// Time structure used in a git index entry.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct IndexTime {
    raw: raw::git_index_time,
}

impl Time {
    /// Creates a new time structure from its components.
    pub fn new(time: i64, offset: i32) -> Time {
        unsafe {
            Binding::from_raw(raw::git_time {
                time: time as raw::git_time_t,
                offset: offset as c_int,
                sign: if offset < 0 { '-' } else { '+' } as c_char,
            })
        }
    }

    /// Return the time, in seconds, from epoch
    pub fn seconds(&self) -> i64 {
        self.raw.time as i64
    }

    /// Return the timezone offset, in minutes
    pub fn offset_minutes(&self) -> i32 {
        self.raw.offset as i32
    }

    /// Return whether the offset was positive or negative. Primarily useful
    /// in case the offset is specified as a negative zero.
    pub fn sign(&self) -> char {
        self.raw.sign as u8 as char
    }
}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Time) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Time) -> Ordering {
        (self.raw.time, self.raw.offset).cmp(&(other.raw.time, other.raw.offset))
    }
}

impl Binding for Time {
    type Raw = raw::git_time;
    unsafe fn from_raw(raw: raw::git_time) -> Time {
        Time { raw }
    }
    fn raw(&self) -> raw::git_time {
        self.raw
    }
}

impl IndexTime {
    /// Creates a new time structure from its components.
    pub fn new(seconds: i32, nanoseconds: u32) -> IndexTime {
        unsafe {
            Binding::from_raw(raw::git_index_time {
                seconds,
                nanoseconds,
            })
        }
    }

    /// Returns the number of seconds in the second component of this time.
    pub fn seconds(&self) -> i32 {
        self.raw.seconds
    }
    /// Returns the nanosecond component of this time.
    pub fn nanoseconds(&self) -> u32 {
        self.raw.nanoseconds
    }
}

impl Binding for IndexTime {
    type Raw = raw::git_index_time;
    unsafe fn from_raw(raw: raw::git_index_time) -> IndexTime {
        IndexTime { raw }
    }
    fn raw(&self) -> raw::git_index_time {
        self.raw
    }
}

impl PartialOrd for IndexTime {
    fn partial_cmp(&self, other: &IndexTime) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for IndexTime {
    fn cmp(&self, other: &IndexTime) -> Ordering {
        let me = (self.raw.seconds, self.raw.nanoseconds);
        let other = (other.raw.seconds, other.raw.nanoseconds);
        me.cmp(&other)
    }
}

#[cfg(test)]
mod tests {
    use crate::Time;

    #[test]
    fn smoke() {
        assert_eq!(Time::new(1608839587, -300).seconds(), 1608839587);
        assert_eq!(Time::new(1608839587, -300).offset_minutes(), -300);
        assert_eq!(Time::new(1608839587, -300).sign(), '-');
        assert_eq!(Time::new(1608839587, 300).sign(), '+');
    }
}
