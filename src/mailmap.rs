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
