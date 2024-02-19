use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str;

use crate::{raw, Error, IntoCString, ObjectType};

use crate::util::{c_cmp_to_ordering, Binding};

/// Unique identity of any object (commit, tree, blob, tag).
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Oid {
    raw: raw::git_oid,
}

impl Oid {
    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty, is longer than 40 hex
    /// characters, or contains any non-hex characters.
    pub fn from_str(s: &str) -> Result<Oid, Error> {
        crate::init();
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_oid_fromstrn(
                &mut raw,
                s.as_bytes().as_ptr() as *const libc::c_char,
                s.len() as libc::size_t
            ));
        }
        Ok(Oid { raw })
    }

    /// Parse a raw object id into an Oid structure.
    ///
    /// If the array given is not 20 bytes in length, an error is returned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Oid, Error> {
        crate::init();
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        if bytes.len() != raw::GIT_OID_RAWSZ {
            Err(Error::from_str("raw byte array must be 20 bytes"))
        } else {
            unsafe {
                try_call!(raw::git_oid_fromraw(&mut raw, bytes.as_ptr()));
            }
            Ok(Oid { raw })
        }
    }

    /// Creates an all zero Oid structure.
    pub fn zero() -> Oid {
        let out = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        Oid { raw: out }
    }

    /// Hashes the provided data as an object of the provided type, and returns
    /// an Oid corresponding to the result. This does not store the object
    /// inside any object database or repository.
    pub fn hash_object(kind: ObjectType, bytes: &[u8]) -> Result<Oid, Error> {
        crate::init();

        let mut out = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_odb_hash(
                &mut out,
                bytes.as_ptr() as *const libc::c_void,
                bytes.len(),
                kind.raw()
            ));
        }

        Ok(Oid { raw: out })
    }

    /// Hashes the content of the provided file as an object of the provided type,
    /// and returns an Oid corresponding to the result. This does not store the object
    /// inside any object database or repository.
    pub fn hash_file<P: AsRef<Path>>(kind: ObjectType, path: P) -> Result<Oid, Error> {
        crate::init();

        // Normal file path OK (does not need Windows conversion).
        let rpath = path.as_ref().into_c_string()?;

        let mut out = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_odb_hashfile(&mut out, rpath, kind.raw()));
        }

        Ok(Oid { raw: out })
    }

    /// View this OID as a byte-slice 20 bytes in length.
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw.id
    }

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
    fn raw(&self) -> *const raw::git_oid {
        &self.raw as *const _
    }
}

impl fmt::Debug for Oid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl fmt::Display for Oid {
    /// Hex-encode this Oid into a formatter.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut dst = [0u8; raw::GIT_OID_HEXSZ + 1];
        unsafe {
            raw::git_oid_tostr(
                dst.as_mut_ptr() as *mut libc::c_char,
                dst.len() as libc::size_t,
                &self.raw,
            );
        }
        let s = &dst[..dst.iter().position(|&a| a == 0).unwrap()];
        str::from_utf8(s).unwrap().fmt(f)
    }
}

impl str::FromStr for Oid {
    type Err = Error;

    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty, is longer than 40 hex
    /// characters, or contains any non-hex characters.
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
        c_cmp_to_ordering(unsafe { raw::git_oid_cmp(&self.raw, &other.raw) })
    }
}

impl Hash for Oid {
    fn hash<H: Hasher>(&self, into: &mut H) {
        self.raw.id.hash(into)
    }
}

impl AsRef<[u8]> for Oid {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::io::prelude::*;

    use super::Error;
    use super::Oid;
    use crate::ObjectType;
    use tempfile::TempDir;

    #[test]
    fn conversions() {
        assert!(Oid::from_str("foo").is_err());
        assert!(Oid::from_str("decbf2be529ab6557d5429922251e5ee36519817").is_ok());
        assert!(Oid::from_bytes(b"foo").is_err());
        assert!(Oid::from_bytes(b"00000000000000000000").is_ok());
    }

    #[test]
    fn comparisons() -> Result<(), Error> {
        assert_eq!(Oid::from_str("decbf2b")?, Oid::from_str("decbf2b")?);
        assert!(Oid::from_str("decbf2b")? <= Oid::from_str("decbf2b")?);
        assert!(Oid::from_str("decbf2b")? >= Oid::from_str("decbf2b")?);
        {
            let o = Oid::from_str("decbf2b")?;
            assert_eq!(o, o);
            assert!(o <= o);
            assert!(o >= o);
        }
        assert_eq!(
            Oid::from_str("decbf2b")?,
            Oid::from_str("decbf2b000000000000000000000000000000000")?
        );
        assert!(
            Oid::from_bytes(b"00000000000000000000")? < Oid::from_bytes(b"00000000000000000001")?
        );
        assert!(Oid::from_bytes(b"00000000000000000000")? < Oid::from_str("decbf2b")?);
        assert_eq!(
            Oid::from_bytes(b"00000000000000000000")?,
            Oid::from_str("3030303030303030303030303030303030303030")?
        );
        Ok(())
    }

    #[test]
    fn zero_is_zero() {
        assert!(Oid::zero().is_zero());
    }

    #[test]
    fn hash_object() {
        let bytes = "Hello".as_bytes();
        assert!(Oid::hash_object(ObjectType::Blob, bytes).is_ok());
    }

    #[test]
    fn hash_file() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("hello.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all("Hello".as_bytes()).unwrap();
        assert!(Oid::hash_file(ObjectType::Blob, &path).is_ok());
    }
}
