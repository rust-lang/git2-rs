use std::kinds::marker;
use std::str;

use {raw, Remote, Direction} ;

/// A structure to represent a git [refspec][1].
///
/// Refspecs are currently mainly accessed/created through a `Remote`.
///
/// [1]: http://git-scm.com/book/en/Git-Internals-The-Refspec
pub struct Refspec<'a> {
    raw: *const raw::git_refspec,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
    marker4: marker::NoCopy,
}

impl<'a> Refspec<'a> {
    /// Creates a new refspec from the raw components.
    ///
    /// This is unsafe as the `raw` pointer is not guaranteed to be valid.
    pub unsafe fn from_raw<'a>(_repo: &'a Remote,
                               raw: *const raw::git_refspec) -> Refspec<'a> {
        Refspec {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
            marker4: marker::NoCopy,
        }
    }

    /// Get the refspec's direction.
    pub fn direction(&self) -> Direction {
        match unsafe { raw::git_refspec_direction(self.raw) } {
            raw::GIT_DIRECTION_FETCH => Direction::Fetch,
            raw::GIT_DIRECTION_PUSH => Direction::Push,
        }
    }

    /// Get the destination specifier.
    ///
    /// If the destination is not utf-8, None is returned.
    pub fn dst(&self) -> Option<&str> {
        str::from_utf8(self.dst_bytes())
    }

    /// Get the destination specifier, in bytes.
    pub fn dst_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_refspec_dst(self.raw)).unwrap() }
    }

    /// Check if a refspec's destination descriptor matches a reference
    pub fn dst_matches(&self, refname: &str) -> bool {
        let refname = refname.to_c_str();
        unsafe { raw::git_refspec_dst_matches(self.raw, refname.as_ptr()) == 1 }
    }

    /// Get the source specifier.
    ///
    /// If the source is not utf-8, None is returned.
    pub fn src(&self) -> Option<&str> {
        str::from_utf8(self.src_bytes())
    }

    /// Get the source specifier, in bytes.
    pub fn src_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_refspec_src(self.raw)).unwrap() }
    }

    /// Check if a refspec's source descriptor matches a reference
    pub fn src_matches(&self, refname: &str) -> bool {
        let refname = refname.to_c_str();
        unsafe { raw::git_refspec_src_matches(self.raw, refname.as_ptr()) == 1 }
    }

    /// Get the force update setting.
    pub fn is_force(&self) -> bool {
        unsafe { raw::git_refspec_force(self.raw) == 1 }
    }

    /// Get the refspec's string.
    ///
    /// Returns None if the string is not valid utf8.
    pub fn str(&self) -> Option<&str> {
        str::from_utf8(self.bytes())
    }

    /// Get the refspec's string as a byte array
    pub fn bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_refspec_string(self.raw)).unwrap() }
    }
}
