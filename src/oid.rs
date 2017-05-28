use std::fmt;
use std::cmp::Ordering;
use std::hash::{Hasher, Hash};
use std::str;
use libc;

use {raw, Error};
use util::Binding;

/// Unique identity of any object (commit, tree, blob, tag).
#[derive(Copy, Clone)]
pub struct Oid {
    raw: raw::git_oid,
}

impl Oid {
    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// If the string is not a valid 40-character hex string, an error is
    /// returned.
    pub fn from_str(s: &str) -> Result<Oid, Error> {
        ::init();
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_oid_fromstrn(&mut raw,
                                            s.as_bytes().as_ptr()
                                                as *const libc::c_char,
                                            s.len() as libc::size_t));
        }
        Ok(Oid { raw: raw })
    }

    /// Parse a raw object id into an Oid structure.
    ///
    /// If the array given is not 20 bytes in length, an error is returned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Oid, Error> {
        ::init();
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        if bytes.len() != raw::GIT_OID_RAWSZ {
            Err(Error::from_str("raw byte array must be 20 bytes"))
        } else {
            unsafe { raw::git_oid_fromraw(&mut raw, bytes.as_ptr()) }
            Ok(Oid { raw: raw })
        }
    }

    /// View this OID as a byte-slice 20 bytes in length.
    pub fn as_bytes(&self) -> &[u8] { &self.raw.id }

    /// Test if this OID is all zeros.
    pub fn is_zero(&self) -> bool {
        unsafe { raw::git_oid_iszero(&self.raw) == 1 }
    }
}

impl Binding for Oid {
    type Raw = *const raw::git_oid;

    unsafe fn from_raw(oid: *const raw::git_oid) -> Oid {
        Oid { raw: *oid }
    }
    fn raw(&self) -> *const raw::git_oid { &self.raw as *const _ }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Oid {
    /// Hex-encode this Oid into a formatter.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dst = [0u8; raw::GIT_OID_HEXSZ + 1];
        unsafe {
            raw::git_oid_tostr(dst.as_mut_ptr() as *mut libc::c_char,
                               dst.len() as libc::size_t, &self.raw);
        }
        let s = &dst[..dst.iter().position(|&a| a == 0).unwrap()];
        str::from_utf8(s).unwrap().fmt(f)
    }
}

impl str::FromStr for Oid {
    type Err = Error;

    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// If the string is not a valid 40-character hex string, an error is
    /// returned.
    fn from_str(s: &str) -> Result<Oid, Error> {
        Oid::from_str(s)
    }
}

impl PartialEq for Oid {
    fn eq(&self, other: &Oid) -> bool {
        unsafe { raw::git_oid_equal(&self.raw, &other.raw) != 0 }
    }
}
impl Eq for Oid {}

impl PartialOrd for Oid {
    fn partial_cmp(&self, other: &Oid) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Oid {
    fn cmp(&self, other: &Oid) -> Ordering {
        match unsafe { raw::git_oid_cmp(&self.raw, &other.raw) } {
            0 => Ordering::Equal,
            n if n < 0 => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl Hash for Oid {
    fn hash<H: Hasher>(&self, into: &mut H) {
        self.raw.id.hash(into)
    }
}

impl AsRef<[u8]> for Oid {
    fn as_ref(&self) -> &[u8] { self.as_bytes() }
}

#[cfg(test)]
mod tests {
    use super::Oid;

    #[test]
    fn conversions() {
        assert!(Oid::from_str("foo").is_err());
        assert!(Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").is_ok());
        assert!(Oid::from_bytes(b"foo").is_err());
        assert!(Oid::from_bytes(b"00000000000000000000").is_ok());
    }
}
