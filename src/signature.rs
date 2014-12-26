use std::str;
use std::kinds::marker;
use libc;

use {raw, Error, Time};

/// A Signature is used to indicate authorship of various actions throughout the
/// library.
///
/// Signatures contain a name, email, and timestamp. All fields can be specified
/// with `new`, the `now` constructor omits the timestamp, and the `default`
/// constructor reads configuration from the given repository.
pub struct Signature<'a> {
    raw: *mut raw::git_signature,
    marker: marker::ContravariantLifetime<'a>,
    owned: bool,
}

impl<'a> Signature<'a> {
    /// Create a new action signature with a timestamp of 'now'.
    ///
    /// See `new` for more information
    pub fn now(name: &str, email: &str) -> Result<Signature<'static>, Error> {
        ::init();
        let mut ret = 0 as *mut raw::git_signature;
        unsafe {
            try_call!(raw::git_signature_now(&mut ret, name.to_c_str(),
                                             email.to_c_str()));
            Ok(Signature::from_raw(ret))
        }
    }

    /// Create a new action signature.
    ///
    /// The `time` specified is in seconds since the epoch, and the `offset` is
    /// the time zone offset in minutes.
    ///
    /// Returns error if either `name` or `email` contain angle brackets.
    pub fn new(name: &str, email: &str, time: u64,
               offset: int) -> Result<Signature<'static>, Error> {
        ::init();
        let mut ret = 0 as *mut raw::git_signature;
        unsafe {
            try_call!(raw::git_signature_new(&mut ret, name.to_c_str(),
                                             email.to_c_str(),
                                             time as raw::git_time_t,
                                             offset as libc::c_int));
            Ok(Signature::from_raw(ret))
        }
    }

    /// Consumes ownership of a raw signature pointer
    ///
    /// This function is unsafe as the pointer is not guranteed to be valid.
    pub unsafe fn from_raw(raw: *mut raw::git_signature) -> Signature<'static> {
        Signature {
            raw: raw,
            marker: marker::ContravariantLifetime,
            owned: true,
        }
    }

    /// Creates a new signature from the give raw pointer, tied to the lifetime
    /// of the given object.
    ///
    /// This function is unsafe as there is no guarantee that `raw` is valid for
    /// `'a` nor if it's a valid pointer.
    pub unsafe fn from_raw_const<'b, T>(_lt: &'b T,
                                        raw: *const raw::git_signature)
                                        -> Signature<'b> {
        Signature {
            raw: raw as *mut raw::git_signature,
            marker: marker::ContravariantLifetime,
            owned: false,
        }
    }

    /// Gets the name on the signature.
    ///
    /// Returns `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes()).ok()
    }

    /// Gets the name on the signature as a byte slice.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, (*self.raw).name as *const _).unwrap() }
    }

    /// Gets the email on the signature.
    ///
    /// Returns `None` if the email is not valid utf-8
    pub fn email(&self) -> Option<&str> {
        str::from_utf8(self.email_bytes()).ok()
    }

    /// Gets the email on the signature as a byte slice.
    pub fn email_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, (*self.raw).email as *const _).unwrap() }
    }

    /// Get the `when` of this signature.
    pub fn when(&self) -> Time {
        unsafe { Time::from_raw(&(*self.raw).when) }
    }

    /// Get access to the underlying raw signature
    pub fn raw(&self) -> *mut raw::git_signature { self.raw }
}

impl Clone for Signature<'static> {
    fn clone(&self) -> Signature<'static> {
        let mut raw = 0 as *mut raw::git_signature;
        let rc = unsafe { raw::git_signature_dup(&mut raw, &*self.raw) };
        assert_eq!(rc, 0);
        unsafe { Signature::from_raw(raw) }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Signature<'a> {
    fn drop(&mut self) {
        if self.owned {
            unsafe { raw::git_signature_free(self.raw) }
        }
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
        assert_eq!(s.name(), Some("foo"));
        assert_eq!(s.email(), Some("bar"));

        drop(s.clone());
    }
}
