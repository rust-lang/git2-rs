use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str;

use crate::{raw, Error, IntoCString, ObjectType};

use crate::util::{c_cmp_to_ordering, Binding};

/// Object ID format (hash algorithm).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ObjectFormat {
    /// SHA1 object format (20-byte object IDs)
    Sha1,
}

impl Binding for ObjectFormat {
    type Raw = raw::git_oid_t;

    unsafe fn from_raw(raw: raw::git_oid_t) -> Self {
        match raw {
            raw::GIT_OID_SHA1 => ObjectFormat::Sha1,
            _ => panic!("Unknown git oid type"),
        }
    }

    fn raw(&self) -> Self::Raw {
        match self {
            ObjectFormat::Sha1 => raw::GIT_OID_SHA1,
        }
    }
}

/// Unique identity of any object (commit, tree, blob, tag).
#[derive(Copy, Clone)]
#[repr(C)]
pub struct Oid {
    raw: raw::git_oid,
}

impl Oid {
    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// This always parses as SHA1 (up to 40 hex characters). Use
    /// [`Oid::from_str_ext`] to parse with a specific format.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty, is longer than 40 hex
    /// characters, or contains any non-hex characters.
    pub fn from_str(s: &str) -> Result<Oid, Error> {
        Self::from_str_ext(s, ObjectFormat::Sha1)
    }

    /// Parses a hex-formatted object id with a specific object format.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is
    ///
    /// * is empty
    /// * is longer than 40 hex with SHA1 object format
    /// * is longer than 64 hex with SHA256 object format
    /// * contains any non-hex characters
    pub fn from_str_ext(s: &str, format: ObjectFormat) -> Result<Oid, Error> {
        crate::init();
        let mut raw = crate::util::zeroed_raw_oid();
        let data = s.as_bytes().as_ptr() as *const libc::c_char;
        let len = s.len() as libc::size_t;
        unsafe {
            let _ = format;
            try_call!(raw::git_oid_fromstrn(&mut raw, data, len));
        }
        Ok(Oid { raw })
    }

    /// Parse a raw object id into an Oid structure.
    ///
    /// If the array given is not 20 bytes in length, an error is returned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Oid, Error> {
        crate::init();
        let mut raw = crate::util::zeroed_raw_oid();

        if bytes.len() != raw::GIT_OID_SHA1_SIZE {
            return Err(Error::from_str("raw byte array must be 20 bytes"));
        }
        unsafe {
            try_call!(raw::git_oid_fromraw(&mut raw, bytes.as_ptr()));
        }

        Ok(Oid { raw })
    }

    /// Creates an all zero Oid structure.
    pub fn zero() -> Oid {
        Oid {
            raw: crate::util::zeroed_raw_oid(),
        }
    }

    /// Hashes the provided data as an object of the provided type, and returns
    /// an Oid corresponding to the result. This does not store the object
    /// inside any object database or repository.
    ///
    /// This always hashes using SHA1. Use [`Oid::hash_object_ext`]
    /// to hash with a specific format.
    pub fn hash_object(kind: ObjectType, bytes: &[u8]) -> Result<Oid, Error> {
        Self::hash_object_ext(kind, bytes, ObjectFormat::Sha1)
    }

    /// Hashes the provided data as an object of the provided type,
    /// with a specific object format.
    ///
    /// See [`Oid::hash_object`] for more details.
    pub fn hash_object_ext(
        kind: ObjectType,
        bytes: &[u8],
        format: ObjectFormat,
    ) -> Result<Oid, Error> {
        crate::init();

        let mut out = crate::util::zeroed_raw_oid();
        let data = bytes.as_ptr() as *const libc::c_void;
        unsafe {
            let _ = format;
            try_call!(raw::git_odb_hash(&mut out, data, bytes.len(), kind.raw()));
        }

        Ok(Oid { raw: out })
    }

    /// Hashes the content of the provided file as an object of the provided type,
    /// and returns an Oid corresponding to the result. This does not store the object
    /// inside any object database or repository.
    ///
    /// This always hashes using SHA1. Use [`Oid::hash_file_ext`]
    /// to hash with a specific format.
    pub fn hash_file<P: AsRef<Path>>(kind: ObjectType, path: P) -> Result<Oid, Error> {
        Self::hash_file_ext(kind, path, ObjectFormat::Sha1)
    }

    /// Hashes the content of a file as an object of the provided type,
    /// with a specific object format.
    ///
    /// See [`Oid::hash_file`] for more details.
    pub fn hash_file_ext<P: AsRef<Path>>(
        kind: ObjectType,
        path: P,
        format: ObjectFormat,
    ) -> Result<Oid, Error> {
        crate::init();

        // Normal file path OK (does not need Windows conversion).
        let rpath = path.as_ref().into_c_string()?;

        let mut out = crate::util::zeroed_raw_oid();
        unsafe {
            let _ = format;
            try_call!(raw::git_odb_hashfile(&mut out, rpath, kind.raw()));
        }

        Ok(Oid { raw: out })
    }

    /// View this OID as a byte-slice in its logical length:
    /// 20 bytes for SHA1, 32 bytes for SHA256.
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw.id
    }

    /// View the full underlying byte buffer of this OID.
    ///
    /// The buffer is always `GIT_OID_MAX_SIZE` bytes long:
    ///
    /// * 20 bytes if the feature `unstable-sha256` is not enabled.
    /// * 32 bytes if the feature `unstable-sha256` is enabled,
    ///   even when the OID is SHA1. The trailing bytes are zero-padding.
    pub fn raw_bytes(&self) -> &[u8] {
        &self.raw.id
    }

    /// Test if this OID is all zeros.
    pub fn is_zero(&self) -> bool {
        unsafe { raw::git_oid_is_zero(&self.raw) == 1 }
    }

    /// Returns the [`ObjectFormat`] of this OID.
    ///
    /// Without the `unstable-sha256` feature, this always returns
    /// [`ObjectFormat::Sha1`].
    pub fn object_format(&self) -> ObjectFormat {
        ObjectFormat::Sha1
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
        let mut dst = [0u8; raw::GIT_OID_MAX_HEXSIZE + 1];
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
    /// This always parses as SHA1.
    /// Use [`Oid::from_str_ext`] for format-aware parsing.
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

    use libgit2_sys as raw;

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
    fn object_format_always_sha1() {
        let oid = Oid::from_bytes(&[0u8; 20]).unwrap();
        assert_eq!(oid.object_format(), crate::ObjectFormat::Sha1);
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
        let oid = Oid::hash_object(ObjectType::Blob, bytes).unwrap();
        assert_eq!(oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(oid.as_bytes().len(), raw::GIT_OID_SHA1_SIZE);
    }

    #[test]
    fn hash_file() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("hello.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all("Hello".as_bytes()).unwrap();
        let oid = Oid::hash_file(ObjectType::Blob, &path).unwrap();
        assert_eq!(oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(oid.as_bytes().len(), raw::GIT_OID_SHA1_SIZE);
    }
}
