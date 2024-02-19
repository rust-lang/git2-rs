use std::ffi::CString;
use std::fmt;
use std::marker;
use std::mem;
use std::ptr;
use std::str;

use crate::util::Binding;
use crate::{raw, Error, Time};

/// A Signature is used to indicate authorship of various actions throughout the
/// library.
///
/// Signatures contain a name, email, and timestamp. All fields can be specified
/// with `new` while the `now` constructor omits the timestamp. The
/// [`Repository::signature`] method can be used to create a default signature
/// with name and email values read from the configuration.
///
/// [`Repository::signature`]: struct.Repository.html#method.signature
pub struct Signature<'a> {
    raw: *mut raw::git_signature,
    _marker: marker::PhantomData<&'a str>,
    owned: bool,
}

impl<'a> Signature<'a> {
    /// Create a new action signature with a timestamp of 'now'.
    ///
    /// See `new` for more information
    pub fn now(name: &str, email: &str) -> Result<Signature<'static>, Error> {
        crate::init();
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        let email = CString::new(email)?;
        unsafe {
            try_call!(raw::git_signature_now(&mut ret, name, email));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a new action signature.
    ///
    /// The `time` specified is in seconds since the epoch, and the `offset` is
    /// the time zone offset in minutes.
    ///
    /// Returns error if either `name` or `email` contain angle brackets.
    pub fn new(name: &str, email: &str, time: &Time) -> Result<Signature<'static>, Error> {
        crate::init();
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        let email = CString::new(email)?;
        unsafe {
            try_call!(raw::git_signature_new(
                &mut ret,
                name,
                email,
                time.seconds() as raw::git_time_t,
                time.offset_minutes() as libc::c_int
            ));
            Ok(Binding::from_raw(ret))
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
        unsafe { crate::opt_bytes(self, (*self.raw).name).unwrap() }
    }

    /// Gets the email on the signature.
    ///
    /// Returns `None` if the email is not valid utf-8
    pub fn email(&self) -> Option<&str> {
        str::from_utf8(self.email_bytes()).ok()
    }

    /// Gets the email on the signature as a byte slice.
    pub fn email_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, (*self.raw).email).unwrap() }
    }

    /// Get the `when` of this signature.
    pub fn when(&self) -> Time {
        unsafe { Binding::from_raw((*self.raw).when) }
    }

    /// Convert a signature of any lifetime into an owned signature with a
    /// static lifetime.
    pub fn to_owned(&self) -> Signature<'static> {
        unsafe {
            let me = mem::transmute::<&Signature<'a>, &Signature<'static>>(self);
            me.clone()
        }
    }
}

impl<'a> Binding for Signature<'a> {
    type Raw = *mut raw::git_signature;
    unsafe fn from_raw(raw: *mut raw::git_signature) -> Signature<'a> {
        Signature {
            raw,
            _marker: marker::PhantomData,
            owned: true,
        }
    }
    fn raw(&self) -> *mut raw::git_signature {
        self.raw
    }
}

/// Creates a new signature from the give raw pointer, tied to the lifetime
/// of the given object.
///
/// This function is unsafe as there is no guarantee that `raw` is valid for
/// `'a` nor if it's a valid pointer.
pub unsafe fn from_raw_const<'b, T>(_lt: &'b T, raw: *const raw::git_signature) -> Signature<'b> {
    Signature {
        raw: raw as *mut raw::git_signature,
        _marker: marker::PhantomData,
        owned: false,
    }
}

impl Clone for Signature<'static> {
    fn clone(&self) -> Signature<'static> {
        // TODO: can this be defined for 'a and just do a plain old copy if the
        //       lifetime isn't static?
        let mut raw = ptr::null_mut();
        let rc = unsafe { raw::git_signature_dup(&mut raw, &*self.raw) };
        assert_eq!(rc, 0);
        unsafe { Binding::from_raw(raw) }
    }
}

impl<'a> Drop for Signature<'a> {
    fn drop(&mut self) {
        if self.owned {
            unsafe { raw::git_signature_free(self.raw) }
        }
    }
}

impl<'a> fmt::Display for Signature<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} <{}>",
            String::from_utf8_lossy(self.name_bytes()),
            String::from_utf8_lossy(self.email_bytes())
        )
    }
}

impl PartialEq for Signature<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.when() == other.when()
            && self.email_bytes() == other.email_bytes()
            && self.name_bytes() == other.name_bytes()
    }
}

impl Eq for Signature<'_> {}

#[cfg(test)]
mod tests {
    use crate::{Signature, Time};

    #[test]
    fn smoke() {
        Signature::new("foo", "bar", &Time::new(89, 0)).unwrap();
        Signature::now("foo", "bar").unwrap();
        assert!(Signature::new("<foo>", "bar", &Time::new(89, 0)).is_err());
        assert!(Signature::now("<foo>", "bar").is_err());

        let s = Signature::now("foo", "bar").unwrap();
        assert_eq!(s.name(), Some("foo"));
        assert_eq!(s.email(), Some("bar"));

        drop(s.clone());
        drop(s.to_owned());
    }
}
