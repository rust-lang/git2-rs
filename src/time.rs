#![allow(missing_copy_implementations)]
use raw;

/// Time in a signature
pub struct Time {
    time: i64,
    offset: int,
}

/// Time structure used in a git index entry.
pub struct IndexTime {
    seconds: i64,
    nanoseconds: u32,
}

impl Time {
    /// Creates a new time structure from its components.
    pub fn new(time: i64, offset: int) -> Time {
        Time { time: time, offset: offset }
    }

    /// Construct a new `Time` from a raw component
    pub fn from_raw(raw: &raw::git_time) -> Time {
        Time::new(raw.time as i64, raw.offset as int)
    }

    /// Return the time, in seconds, from epoch
    pub fn seconds(&self) -> i64 { self.time }

    /// Return the timezone offset, in minutes
    pub fn offset_minutes(&self) -> int { self.offset }
}

impl IndexTime {
    /// Creates a new time structure from its components.
    pub fn new(seconds: i64, nanoseconds: u32) -> IndexTime {
        IndexTime { seconds: seconds, nanoseconds: nanoseconds }
    }

    /// Construct a new `Time` from a raw component
    pub fn from_raw(raw: &raw::git_index_time) -> IndexTime {
        IndexTime::new(raw.seconds as i64, raw.nanoseconds as u32)
    }

    /// Returns the number of seconds in the second component of this time.
    pub fn seconds(&self) -> i64 { self.seconds }
    /// Returns the nanosecond component of this time.
    pub fn nanoseconds(&self) -> u32 { self.nanoseconds }
}
