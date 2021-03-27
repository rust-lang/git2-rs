use std::ffi::CString;
use std::ptr;

use crate::util::Binding;
use crate::{raw, Error, Signature};

/// A structure to represent a repository's .mailmap file.
///
/// The representation cannot be written to disk.
pub struct Mailmap {
    raw: *mut raw::git_mailmap,
}

impl Binding for Mailmap {
    type Raw = *mut raw::git_mailmap;

    unsafe fn from_raw(ptr: *mut raw::git_mailmap) -> Mailmap {
        Mailmap { raw: ptr }
    }

    fn raw(&self) -> *mut raw::git_mailmap {
        self.raw
    }
}

impl Drop for Mailmap {
    fn drop(&mut self) {
        unsafe {
            raw::git_mailmap_free(self.raw);
        }
    }
}

impl Mailmap {
    /// Creates an empty, in-memory mailmap object.
    pub fn new() -> Result<Mailmap, Error> {
        crate::init();
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_mailmap_new(&mut ret));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Creates an in-memory mailmap object representing the given buffer.
    pub fn from_buffer(buf: &str) -> Result<Mailmap, Error> {
        crate::init();
        let mut ret = ptr::null_mut();
        let len = buf.len();
        let buf = CString::new(buf)?;
        unsafe {
            try_call!(raw::git_mailmap_from_buffer(&mut ret, buf, len));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Adds a new entry to this in-memory mailmap object.
    pub fn add_entry(
        &mut self,
        real_name: Option<&str>,
        real_email: Option<&str>,
        replace_name: Option<&str>,
        replace_email: &str,
    ) -> Result<(), Error> {
        let real_name = crate::opt_cstr(real_name)?;
        let real_email = crate::opt_cstr(real_email)?;
        let replace_name = crate::opt_cstr(replace_name)?;
        let replace_email = CString::new(replace_email)?;
        unsafe {
            try_call!(raw::git_mailmap_add_entry(
                self.raw,
                real_name,
                real_email,
                replace_name,
                replace_email
            ));
            Ok(())
        }
    }

    /// Resolves a signature to its real name and email address.
    pub fn resolve_signature(&self, sig: &Signature<'_>) -> Result<Signature<'static>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_mailmap_resolve_signature(
                &mut ret,
                &*self.raw,
                sig.raw()
            ));
            Ok(Binding::from_raw(ret))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke() {
        let sig_name = "name";
        let sig_email = "email";
        let sig = t!(Signature::now(sig_name, sig_email));

        let mut mm = t!(Mailmap::new());

        let mailmapped_sig = t!(mm.resolve_signature(&sig));
        assert_eq!(mailmapped_sig.name(), Some(sig_name));
        assert_eq!(mailmapped_sig.email(), Some(sig_email));

        t!(mm.add_entry(None, None, None, sig_email));
        t!(mm.add_entry(
            Some("real name"),
            Some("real@email"),
            Some(sig_name),
            sig_email,
        ));

        let mailmapped_sig = t!(mm.resolve_signature(&sig));
        assert_eq!(mailmapped_sig.name(), Some("real name"));
        assert_eq!(mailmapped_sig.email(), Some("real@email"));
    }

    #[test]
    fn from_buffer() {
        let buf = "<prøper@emæil> <email>";
        let mm = t!(Mailmap::from_buffer(&buf));

        let sig = t!(Signature::now("name", "email"));
        let mailmapped_sig = t!(mm.resolve_signature(&sig));
        assert_eq!(mailmapped_sig.name(), Some("name"));
        assert_eq!(mailmapped_sig.email(), Some("prøper@emæil"));
    }
}
