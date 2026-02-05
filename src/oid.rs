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
    /// # Errors
    ///
    /// Returns an error if the string is empty, is longer than 40 hex
    /// characters (or 64 for SHA256), or contains any non-hex characters.
    pub fn from_str(
        s: &str,
        #[cfg(feature = "unstable-sha256")] format: crate::ObjectFormat,
    ) -> Result<Oid, Error> {
        crate::init();
        let mut raw = crate::util::zeroed_raw_oid();
        let data = s.as_bytes().as_ptr() as *const libc::c_char;
        let len = s.len() as libc::size_t;
        unsafe {
            #[cfg(not(feature = "unstable-sha256"))]
            try_call!(raw::git_oid_fromstrn(&mut raw, data, len));
            #[cfg(feature = "unstable-sha256")]
            try_call!(raw::git_oid_from_prefix(&mut raw, data, len, format.raw()));
        }
        Ok(Oid { raw })
    }

    /// Parse a raw object id into an Oid structure.
    ///
    /// When the `unstable-sha256` feature is enabled, this automatically detects
    /// the OID type based on byte length:
    ///
    /// - 20-byte arrays are parsed as SHA1
    /// - 32-byte arrays are parsed as SHA256
    ///
    /// Without the feature, only 20-byte SHA1 OIDs are supported.
    ///
    /// # Errors
    ///
    /// Returns an error if the byte array is not a valid OID length.
    pub fn from_bytes(bytes: &[u8]) -> Result<Oid, Error> {
        crate::init();
        let mut raw = crate::util::zeroed_raw_oid();

        #[cfg(not(feature = "unstable-sha256"))]
        {
            if bytes.len() != raw::GIT_OID_SHA1_SIZE {
                return Err(Error::from_str("raw byte array must be 20 bytes"));
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
                    return Err(Error::from_str(
                        "raw byte array must be 20 bytes (SHA1) or 32 bytes (SHA256)",
                    ))
                }
            };
            unsafe {
                try_call!(raw::git_oid_from_raw(&mut raw, bytes.as_ptr(), oid_type));
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
    pub fn hash_object(
        kind: ObjectType,
        bytes: &[u8],
        #[cfg(feature = "unstable-sha256")] format: crate::ObjectFormat,
    ) -> Result<Oid, Error> {
        crate::init();

        let mut out = crate::util::zeroed_raw_oid();
        let data = bytes.as_ptr() as *const libc::c_void;
        unsafe {
            #[cfg(not(feature = "unstable-sha256"))]
            try_call!(raw::git_odb_hash(&mut out, data, bytes.len(), kind.raw()));
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
    pub fn hash_file<P: AsRef<Path>>(
        kind: ObjectType,
        path: P,
        #[cfg(feature = "unstable-sha256")] format: crate::ObjectFormat,
    ) -> Result<Oid, Error> {
        crate::init();

        // Normal file path OK (does not need Windows conversion).
        let rpath = path.as_ref().into_c_string()?;

        let mut out = crate::util::zeroed_raw_oid();
        unsafe {
            #[cfg(not(feature = "unstable-sha256"))]
            try_call!(raw::git_odb_hashfile(&mut out, rpath, kind.raw()));
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

    /// View this OID as a byte-slice.
    ///
    /// * 20 bytes in length if the feature `unstable-sha256` is not enabled.
    /// * 32 bytes in length if the feature `unstable-sha256` is enabled.
    pub fn as_bytes(&self) -> &[u8] {
        &self.raw.id
    }

    /// Test if this OID is all zeros.
    pub fn is_zero(&self) -> bool {
        unsafe { raw::git_oid_is_zero(&self.raw) == 1 }
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

#[cfg(not(feature = "unstable-sha256"))]
impl str::FromStr for Oid {
    type Err = Error;

    /// Parse a hex-formatted object id into an Oid structure.
    ///
    /// # Errors
    ///
    /// Returns an error if the string is empty, is longer than 40 hex
    /// characters, or contains any non-hex characters.
    ///
    /// <div class="warning">
    ///
    /// # SHA1-only limitation
    ///
    /// This method **always** parses as SHA1 (up to 40 hex characters).
    /// It cannot parse SHA256 OIDs because [`str::FromStr::from_str`] lacks
    /// the object format parameter.
    ///
    /// In future releases, this will be removed entirely to avoid misuse.
    ///
    /// Consider these alternatives:
    ///
    /// * [`Oid::from_str`] with explicit [`ObjectFormat`](crate::ObjectFormat)
    /// * [`Oid::from_bytes`] if you have access to the underlying byte of the OID
    /// * [`Repository::revparse_single`](crate::Repository::revparse_single)
    ///   if you have repository context
    ///
    /// </div>
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
    #[cfg(not(feature = "unstable-sha256"))]
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

        assert!(Oid::from_str("foo", ObjectFormat::Sha1).is_err());
        assert!(Oid::from_str(
            "decbf2be529ab6557d5429922251e5ee36519817",
            ObjectFormat::Sha1
        )
        .is_ok());

        assert!(Oid::from_str("foo", ObjectFormat::Sha256).is_err());
        assert!(Oid::from_str(
            "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            ObjectFormat::Sha256
        )
        .is_ok());

        assert!(Oid::from_bytes(b"foo").is_err());

        let sha1_from_bytes = Oid::from_bytes(&[0u8; 20]).unwrap();
        let sha256_from_bytes = Oid::from_bytes(&[0u8; 32]).unwrap();

        // Both stored in 32-byte arrays when sha256 feature is enabled
        assert_eq!(sha1_from_bytes.as_bytes().len(), raw::GIT_OID_MAX_SIZE);
        assert_eq!(sha256_from_bytes.as_bytes().len(), raw::GIT_OID_MAX_SIZE);

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

        Ok(())
    }

    #[test]
    #[cfg(feature = "unstable-sha256")]
    fn comparisons_object_format() -> Result<(), Error> {
        use crate::ObjectFormat;

        // SHA1 OID comparisons with explicit format
        assert_eq!(
            Oid::from_str("decbf2b", ObjectFormat::Sha1)?,
            Oid::from_str("decbf2b", ObjectFormat::Sha1)?
        );
        assert!(
            Oid::from_str("decbf2b", ObjectFormat::Sha1)?
                <= Oid::from_str("decbf2b", ObjectFormat::Sha1)?
        );
        assert!(
            Oid::from_str("decbf2b", ObjectFormat::Sha1)?
                >= Oid::from_str("decbf2b", ObjectFormat::Sha1)?
        );
        {
            let o = Oid::from_str("decbf2b", ObjectFormat::Sha1)?;
            assert_eq!(o, o);
            assert!(o <= o);
            assert!(o >= o);
        }
        assert_eq!(
            Oid::from_str("decbf2b", ObjectFormat::Sha1)?,
            Oid::from_str(
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
                < Oid::from_str("decbf2b", ObjectFormat::Sha1)?
        );

        // SHA256 OID comparisons with explicit format (using full 64-char hex strings)
        assert_eq!(
            Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?,
            Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?
        );
        assert!(
            Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )? <= Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?
        );
        assert!(
            Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )? >= Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256
            )?
        );
        {
            let o = Oid::from_str(
                "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890",
                ObjectFormat::Sha256,
            )?;
            assert_eq!(o, o);
            assert!(o <= o);
            assert!(o >= o);
        }
        assert_eq!(
            Oid::from_str("abcdef12", ObjectFormat::Sha256)?,
            Oid::from_str(
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
                < Oid::from_str(
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
    #[cfg(not(feature = "unstable-sha256"))]
    fn hash_object() {
        let bytes = "Hello".as_bytes();
        let oid = Oid::hash_object(ObjectType::Blob, bytes).unwrap();
        assert_eq!(oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(oid.as_bytes().len(), raw::GIT_OID_MAX_SIZE);
    }

    #[test]
    #[cfg(feature = "unstable-sha256")]
    fn hash_object_with_format() -> Result<(), Error> {
        use crate::ObjectFormat;

        let bytes = b"hello world";

        let sha1_oid = Oid::hash_object(ObjectType::Blob, bytes, ObjectFormat::Sha1)?;
        assert_eq!(sha1_oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(sha1_oid.as_bytes().len(), raw::GIT_OID_MAX_SIZE);

        let sha256_oid = Oid::hash_object(ObjectType::Blob, bytes, ObjectFormat::Sha256)?;
        assert_eq!(sha256_oid.to_string().len(), raw::GIT_OID_SHA256_HEXSIZE);
        assert_eq!(sha256_oid.as_bytes().len(), raw::GIT_OID_MAX_SIZE);

        // Different formats produce different OIDs
        assert_ne!(sha1_oid, sha256_oid);

        Ok(())
    }

    #[test]
    #[cfg(not(feature = "unstable-sha256"))]
    fn hash_file() {
        let td = TempDir::new().unwrap();
        let path = td.path().join("hello.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all("Hello".as_bytes()).unwrap();
        let oid = Oid::hash_file(ObjectType::Blob, &path).unwrap();
        assert_eq!(oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(oid.as_bytes().len(), raw::GIT_OID_MAX_SIZE);
    }

    #[test]
    #[cfg(feature = "unstable-sha256")]
    fn hash_file_object_format() -> Result<(), Error> {
        use crate::ObjectFormat;

        let td = TempDir::new().unwrap();
        let path = td.path().join("test.txt");
        let mut file = File::create(&path).unwrap();
        file.write_all(b"test content").unwrap();

        let sha1_oid = Oid::hash_object(ObjectType::Blob, b"test content", ObjectFormat::Sha1)?;
        assert_eq!(sha1_oid.to_string().len(), raw::GIT_OID_SHA1_HEXSIZE);
        assert_eq!(sha1_oid.as_bytes().len(), raw::GIT_OID_MAX_SIZE);

        let sha256_oid = Oid::hash_object(ObjectType::Blob, b"test content", ObjectFormat::Sha256)?;
        assert_eq!(sha256_oid.to_string().len(), raw::GIT_OID_SHA256_HEXSIZE);
        assert_eq!(sha256_oid.as_bytes().len(), raw::GIT_OID_MAX_SIZE);

        // Different formats produce different OIDs
        assert_ne!(sha1_oid, sha256_oid);

        Ok(())
    }
}
