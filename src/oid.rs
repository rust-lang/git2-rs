use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::str;

use crate::{raw, Error, IntoCString, ObjectType};

use crate::util::{c_cmp_to_ordering, Binding};

/// Object ID format (hash algorithm).
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
#[cfg_attr(not(feature = "unstable-sha256"), non_exhaustive)]
pub enum ObjectFormat {
    /// SHA1 object format (20-byte object IDs)
    Sha1,
    /// SHA256 object format (32-byte object IDs)
    #[cfg(feature = "unstable-sha256")]
    Sha256,
}

impl Binding for ObjectFormat {
    type Raw = raw::git_oid_t;

    unsafe fn from_raw(raw: raw::git_oid_t) -> Self {
        match raw {
            raw::GIT_OID_SHA1 => ObjectFormat::Sha1,
            #[cfg(feature = "unstable-sha256")]
            raw::GIT_OID_SHA256 => ObjectFormat::Sha256,
            _ => panic!("Unknown git oid type"),
        }
    }

    fn raw(&self) -> Self::Raw {
        match self {
            ObjectFormat::Sha1 => raw::GIT_OID_SHA1,
            #[cfg(feature = "unstable-sha256")]
            ObjectFormat::Sha256 => raw::GIT_OID_SHA256,
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
            #[cfg(not(feature = "unstable-sha256"))]
            {
                let _ = format;
                try_call!(raw::git_oid_fromstrn(&mut raw, data, len));
            }
            #[cfg(feature = "unstable-sha256")]
            try_call!(raw::git_oid_fromstrn(&mut raw, data, len, format.raw()));
        }
        Ok(Oid { raw })
    }

    /// Parse a raw object id into an Oid structure.
    ///
    /// If the array given is not 20 bytes in length, an error is returned.
    pub fn from_bytes(bytes: &[u8]) -> Result<Oid, Error> {
        crate::init();
        let mut raw = crate::util::zeroed_raw_oid();

        #[cfg(not(feature = "unstable-sha256"))]
        {
            if bytes.len() != raw::GIT_OID_SHA1_SIZE {
                return Err(Error::from_str(&format!(
                    "raw byte array must be 20 bytes, but got {}",
                    bytes.len()
                )));
            }
            unsafe {
                try_call!(raw::git_oid_fromraw(&mut raw, bytes.as_ptr()));
            }
        }

        #[cfg(feature = "unstable-sha256")]
        {
            let oid_type = match bytes.len() {
                raw::GIT_OID_SHA1_SIZE => raw::GIT_OID_SHA1,
                raw::GIT_OID_SHA256_SIZE => raw::GIT_OID_SHA256,
                _ => {
                    return Err(Error::from_str(&format!(
                        "raw byte array must be 20 bytes (SHA1) or 32 bytes (SHA256), but got {}",
                        bytes.len()
                    )));
                }
            };
            unsafe {
                try_call!(raw::git_oid_fromraw(&mut raw, bytes.as_ptr(), oid_type));
            }
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
            #[cfg(not(feature = "unstable-sha256"))]
            {
                let _ = format;
                try_call!(raw::git_odb_hash(&mut out, data, bytes.len(), kind.raw()));
            }
            #[cfg(feature = "unstable-sha256")]
            try_call!(raw::git_odb_hash(
                &mut out,
                data,
                bytes.len(),
                kind.raw(),
                format.raw()
            ));
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
            #[cfg(not(feature = "unstable-sha256"))]
            {
                let _ = format;
                try_call!(raw::git_odb_hashfile(&mut out, rpath, kind.raw()));
            }
            #[cfg(feature = "unstable-sha256")]
            try_call!(raw::git_odb_hashfile(
                &mut out,
                rpath,
                kind.raw(),
                format.raw()
            ));
        }

        Ok(Oid { raw: out })
    }

    /// View this OID as a byte-slice in its logical length:
    /// 20 bytes for SHA1, 32 bytes for SHA256.
    pub fn as_bytes(&self) -> &[u8] {
        #[cfg(not(feature = "unstable-sha256"))]
        {
            &self.raw.id
        }
        #[cfg(feature = "unstable-sha256")]
        {
            let size = match self.raw.kind as raw::git_oid_t {
                raw::GIT_OID_SHA1 => raw::GIT_OID_SHA1_SIZE,
                raw::GIT_OID_SHA256 => raw::GIT_OID_SHA256_SIZE,
                _ => panic!("Unknown git oid type"),
            };
            &self.raw.id[..size]
        }
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
        #[cfg(not(feature = "unstable-sha256"))]
        {
            ObjectFormat::Sha1
        }
        #[cfg(feature = "unstable-sha256")]
        {
            unsafe { Binding::from_raw(self.raw.kind as raw::git_oid_t) }
        }
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
        #[cfg(feature = "unstable-sha256")]
        self.raw.kind.hash(into);
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
    #[cfg(feature = "unstable-sha256")]
    fn conversions_object_format() {
        use crate::ObjectFormat;

        assert!(Oid::from_str_ext("foo", ObjectFormat::Sha1).is_err());
        assert!(Oid::from_str_ext(
            "decbf2be529ab6557d5429922251e5ee36519817",
            ObjectFormat::Sha1
        )
        .is_ok());

        assert!(Oid::from_str_ext("foo", ObjectFormat::Sha256).is_err());
        assert!(Oid::from_str_ext(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            ObjectFormat::Sha256
        )
        .is_ok());

        assert!(Oid::from_bytes(b"foo").is_err());

        let sha1_from_bytes = Oid::from_bytes(&[0u8; 20]).unwrap();
        let sha256_from_bytes = Oid::from_bytes(&[0u8; 32]).unwrap();

        // as_bytes() returns logical length per OID type
        assert_eq!(sha1_from_bytes.as_bytes().len(), raw::GIT_OID_SHA1_SIZE);
        assert_eq!(sha256_from_bytes.as_bytes().len(), raw::GIT_OID_SHA256_SIZE);

        // raw_bytes() always returns the full buffer
        assert_eq!(sha1_from_bytes.raw_bytes().len(), raw::GIT_OID_MAX_SIZE);
        assert_eq!(sha256_from_bytes.raw_bytes().len(), raw::GIT_OID_MAX_SIZE);

        // Hex string output should differ based on OID type
        assert_eq!(sha1_from_bytes.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(
            sha256_from_bytes.to_string().len(),
            raw::GIT_OID_SHA256_HEXSIZE
        );

        // Verify they're not equal despite being all zeros
        assert_ne!(sha1_from_bytes, sha256_from_bytes);
    }

    #[test]
    fn object_format_always_sha1() {
        let oid = Oid::from_bytes(&[0u8; 20]).unwrap();
        assert_eq!(oid.object_format(), crate::ObjectFormat::Sha1);
    }

    #[test]
    #[cfg(feature = "unstable-sha256")]
    fn object_format_from_oid() {
        use crate::ObjectFormat;

        let sha1 = Oid::from_bytes(&[0u8; 20]).unwrap();
        assert_eq!(sha1.object_format(), ObjectFormat::Sha1);

        let sha256 = Oid::from_bytes(&[0u8; 32]).unwrap();
        assert_eq!(sha256.object_format(), ObjectFormat::Sha256);

        let sha1_from_str = Oid::from_str_ext(
            "decbf2be529ab6557d5429922251e5ee36519817",
            ObjectFormat::Sha1,
        )
        .unwrap();
        assert_eq!(sha1_from_str.object_format(), ObjectFormat::Sha1);

        let sha256_from_str = Oid::from_str_ext(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            ObjectFormat::Sha256,
        )
        .unwrap();
        assert_eq!(sha256_from_str.object_format(), ObjectFormat::Sha256);
    }

    #[test]
    #[cfg(not(feature = "unstable-sha256"))]
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
    #[cfg(feature = "unstable-sha256")]
    fn comparisons_object_format() -> Result<(), Error> {
        use crate::ObjectFormat;

        // SHA1 OID comparisons with explicit format
        assert_eq!(
            Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?,
            Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?
        );
        assert!(
            Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?
                <= Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?
        );
        assert!(
            Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?
                >= Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?
        );
        {
            let o = Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?;
            assert_eq!(o, o);
            assert!(o <= o);
            assert!(o >= o);
        }
        assert_eq!(
            Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?,
            Oid::from_str_ext(
                "decbf2b000000000000000000000000000000000",
                ObjectFormat::Sha1
            )?
        );

        // SHA1 byte comparisons (20 bytes)
        assert!(
            Oid::from_bytes(b"00000000000000000000")? < Oid::from_bytes(b"00000000000000000001")?
        );
        assert!(
            Oid::from_bytes(b"00000000000000000000")?
                < Oid::from_str_ext("decbf2b", ObjectFormat::Sha1)?
        );

        // SHA256 OID comparisons with explicit format (using full 64-char hex strings)
        assert_eq!(
            Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?,
            Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?
        );
        assert!(
            Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )? <= Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?
        );
        assert!(
            Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )? >= Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?
        );
        {
            let o = Oid::from_str_ext(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256,
            )?;
            assert_eq!(o, o);
            assert!(o <= o);
            assert!(o >= o);
        }
        assert_eq!(
            Oid::from_str_ext("abcdef12", ObjectFormat::Sha256)?,
            Oid::from_str_ext(
                "abcdef1200000000000000000000000000000000000000000000000000000000",
                ObjectFormat::Sha256
            )?
        );

        // SHA256 byte comparisons (32 bytes)
        assert!(
            Oid::from_bytes(b"00000000000000000000000000000000")?
                < Oid::from_bytes(b"00000000000000000000000000000001")?
        );
        assert!(
            Oid::from_bytes(b"00000000000000000000000000000000")?
                < Oid::from_str_ext(
                    "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                    ObjectFormat::Sha256
                )?
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
    #[cfg(feature = "unstable-sha256")]
    fn hash_object_with_format() -> Result<(), Error> {
        use crate::ObjectFormat;

        let bytes = b"hello world";

        let sha1_oid = Oid::hash_object_ext(ObjectType::Blob, bytes, ObjectFormat::Sha1)?;
        assert_eq!(sha1_oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(sha1_oid.as_bytes().len(), raw::GIT_OID_SHA1_SIZE);

        let sha256_oid = Oid::hash_object_ext(ObjectType::Blob, bytes, ObjectFormat::Sha256)?;
        assert_eq!(sha256_oid.to_string().len(), raw::GIT_OID_SHA256_HEXSIZE);
        assert_eq!(sha256_oid.as_bytes().len(), raw::GIT_OID_SHA256_SIZE);

        // Different formats produce different OIDs
        assert_ne!(sha1_oid, sha256_oid);

        Ok(())
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

    #[test]
    #[cfg(feature = "unstable-sha256")]
    fn hash_file_object_format() -> Result<(), Error> {
        use crate::ObjectFormat;

        let td = TempDir::new().unwrap();
        let path = td.path().join("test.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();

        let sha1_oid = Oid::hash_file_ext(ObjectType::Blob, &path, ObjectFormat::Sha1)?;
        assert_eq!(sha1_oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(sha1_oid.as_bytes().len(), raw::GIT_OID_SHA1_SIZE);

        let sha256_oid = Oid::hash_file_ext(ObjectType::Blob, &path, ObjectFormat::Sha256)?;
        assert_eq!(sha256_oid.to_string().len(), raw::GIT_OID_SHA256_HEXSIZE);
        assert_eq!(sha256_oid.as_bytes().len(), raw::GIT_OID_SHA256_SIZE);

        // Different formats produce different OIDs
        assert_ne!(sha1_oid, sha256_oid);

        Ok(())
    }
}
