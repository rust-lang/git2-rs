use std::str;
use libc;

use {raw, Repository, Error};

pub struct Signature {
    raw: *mut raw::git_signature,
}

impl Signature {
    /// Create a new action signature with default user and now timestamp.
    ///
    /// This looks up the user.name and user.email from the configuration and
    /// uses the current time as the timestamp, and creates a new signature
    /// based on that information. It will return `NotFound` if either the
    /// user.name or user.email are not set.
    pub fn default(repo: &Repository) -> Result<Signature, Error> {
        let mut ret = 0 as *mut raw::git_signature;
        try!(::doit(|| unsafe {
            raw::git_signature_default(&mut ret, repo.raw())
        }));
        Ok(Signature { raw: ret })
    }

    /// Create a new action signature with a timestamp of 'now'.
    ///
    /// See `new` for more information
    pub fn now(name: &str, email: &str) -> Result<Signature, Error> {
        ::init();
        let name = name.to_c_str();
        let email = email.to_c_str();
        let mut ret = 0 as *mut raw::git_signature;
        try!(::doit(|| unsafe {
            raw::git_signature_now(&mut ret, name.as_ptr(), email.as_ptr())
        }));
        Ok(Signature { raw: ret })
    }

    /// Create a new action signature.
    ///
    /// The `time` specified is in seconds since the epoch, and the `offset` is
    /// the time zone offset in minutes.
    ///
    /// Returns error if either `name` or `email` contain angle brackets.
    pub fn new(name: &str, email: &str, time: u64,
               offset: int) -> Result<Signature, Error> {
        ::init();
        let name = name.to_c_str();
        let email = email.to_c_str();
        let mut ret = 0 as *mut raw::git_signature;
        try!(::doit(|| unsafe {
            raw::git_signature_new(&mut ret, name.as_ptr(), email.as_ptr(),
                                   time as raw::git_time_t,
                                   offset as libc::c_int)
        }));
        Ok(Signature { raw: ret })
    }

    /// Gets the name on the signature.
    pub fn name(&self) -> &str {
        str::from_utf8(unsafe {
            ::opt_bytes(self, (*self.raw).name as *const _).unwrap()
        }).unwrap()
    }

    /// Gets the email on the signature.
    pub fn email(&self) -> &str {
        str::from_utf8(unsafe {
            ::opt_bytes(self, (*self.raw).email as *const _).unwrap()
        }).unwrap()
    }

    /// Get access to the underlying raw signature
    pub fn raw(&self) -> *mut raw::git_signature { self.raw }
}

impl Clone for Signature {
    fn clone(&self) -> Signature {
        let mut raw = 0 as *mut raw::git_signature;
        unsafe {
            ::doit(|| {
                raw::git_signature_dup(&mut raw, &*self.raw)
            }).unwrap();
        }
        Signature { raw: raw }
    }
}

impl Drop for Signature {
    fn drop(&mut self) {
        unsafe { raw::git_signature_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use Signature;

    #[test]
    fn smoke() {
        Signature::new("foo", "bar", 89, 0).unwrap();
        Signature::now("foo", "bar").unwrap();
        assert!(Signature::new("<foo>", "bar", 89, 0).is_err());
        assert!(Signature::now("<foo>", "bar").is_err());

        let s = Signature::now("foo", "bar").unwrap();
        assert_eq!(s.name(), "foo");
        assert_eq!(s.email(), "bar");

        drop(s.clone());
    }
}
