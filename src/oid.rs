use std::fmt;
use std::hash::{sip, Hash};
use libc;

use {raw, Error};

/// Unique identity of any object (commit, tree, blob, tag).
pub struct Oid {
    raw: raw::git_oid,
}

impl Oid {
    /// Create a new Oid from a raw libgit2 oid structure.
    ///
    /// This function is unsafe as it does not know if the memory pointed to by
    /// `oid` is valid or not.
    pub unsafe fn from_raw(oid: *const raw::git_oid) -> Oid {
        ::init();
        Oid { raw: *oid }
    }

    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// If the string is not a valid 40-character hex string, an error is
    /// returned.
    pub fn from_str(s: &str) -> Result<Oid, Error> {
        ::init();
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        try!(::doit(|| unsafe {
            raw::git_oid_fromstrn(&mut raw,
                                  s.as_bytes().as_ptr() as *const libc::c_char,
                                  s.len() as libc::size_t)
        }));
        Ok(Oid { raw: raw })
    }

    /// Parse a raw object id into an Oid structure.
    ///
    /// If the array given is not 20 bytes in length, an error is returned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Oid, Error> {
        ::init();
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        if bytes.len() != raw::GIT_OID_RAWSZ {
            Err(Error::from_str("raw byte array must be 20 bytes"))
        } else {
            unsafe { raw::git_oid_fromraw(&mut raw, bytes.as_ptr()) }
            Ok(Oid { raw: raw })
        }
    }

    /// Gain access to the underlying raw oid pointer
    pub fn raw(&self) -> *const raw::git_oid { &self.raw as *const _ }
}

impl fmt::Show for Oid {
    /// Hex-encode this Oid into a formatter.
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let mut dst = [0u8, ..raw::GIT_OID_HEXSZ + 1];
        unsafe {
            raw::git_oid_tostr(dst.as_mut_ptr() as *mut libc::c_char,
                               dst.len() as libc::size_t, &self.raw);
        }
        f.write(dst.slice_to(dst.iter().position(|&a| a == 0).unwrap()))
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
            0 => Equal,
            n if n < 0 => Less,
            _ => Greater,
        }
    }
}

impl Clone for Oid {
    fn clone(&self) -> Oid { *self }
}

impl Hash for Oid {
    fn hash(&self, into: &mut sip::SipState) {
        self.raw.id.as_slice().hash(into)
    }
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
