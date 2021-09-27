use crate::raw;
use libc::c_int;
use std::fmt;

/// Version information about libgit2 and the capabilities it supports.
pub struct Version {
    major: c_int,
    minor: c_int,
    rev: c_int,
    features: c_int,
}

macro_rules! flag_test {
    ($features:expr, $flag:expr) => {
        ($features as u32 & $flag as u32) != 0
    };
}

impl Version {
    /// Returns a [`Version`] which provides information about libgit2.
    pub fn get() -> Version {
        let mut v = Version {
            major: 0,
            minor: 0,
            rev: 0,
            features: 0,
        };
        unsafe {
            raw::git_libgit2_version(&mut v.major, &mut v.minor, &mut v.rev);
            v.features = raw::git_libgit2_features();
        }
        v
    }

    /// Returns the version of libgit2.
    ///
    /// The return value is a tuple of `(major, minor, rev)`
    pub fn libgit2_version(&self) -> (u32, u32, u32) {
        (self.major as u32, self.minor as u32, self.rev as u32)
    }

    /// Returns the version of the libgit2-sys crate.
    pub fn crate_version(&self) -> &'static str {
        env!("CARGO_PKG_VERSION")
    }

    /// Returns true if this was built with the vendored version of libgit2.
    pub fn vendored(&self) -> bool {
        raw::vendored()
    }

    /// Returns true if libgit2 was built thread-aware and can be safely used
    /// from multiple threads.
    pub fn threads(&self) -> bool {
        flag_test!(self.features, raw::GIT_FEATURE_THREADS)
    }

    /// Returns true if libgit2 was built with and linked against a TLS implementation.
    ///
    /// Custom TLS streams may still be added by the user to support HTTPS
    /// regardless of this.
    pub fn https(&self) -> bool {
        flag_test!(self.features, raw::GIT_FEATURE_HTTPS)
    }

    /// Returns true if libgit2 was built with and linked against libssh2.
    ///
    /// A custom transport may still be added by the user to support libssh2
    /// regardless of this.
    pub fn ssh(&self) -> bool {
        flag_test!(self.features, raw::GIT_FEATURE_SSH)
    }

    /// Returns true if libgit2 was built with support for sub-second
    /// resolution in file modification times.
    pub fn nsec(&self) -> bool {
        flag_test!(self.features, raw::GIT_FEATURE_NSEC)
    }
}

impl fmt::Debug for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> Result<(), fmt::Error> {
        let mut f = f.debug_struct("Version");
        f.field("major", &self.major)
            .field("minor", &self.minor)
            .field("rev", &self.rev)
            .field("crate_version", &self.crate_version())
            .field("vendored", &self.vendored())
            .field("threads", &self.threads())
            .field("https", &self.https())
            .field("ssh", &self.ssh())
            .field("nsec", &self.nsec());
        f.finish()
    }
}
