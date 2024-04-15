use crate::util::Binding;
use crate::{raw, Oid};
use std::marker;
use std::str;

/// Represents an update which will be performed on the remote during push.
pub struct PushUpdate<'a> {
    raw: *const raw::git_push_update,
    _marker: marker::PhantomData<&'a raw::git_push_update>,
}

impl<'a> Binding for PushUpdate<'a> {
    type Raw = *const raw::git_push_update;
    unsafe fn from_raw(raw: *const raw::git_push_update) -> PushUpdate<'a> {
        PushUpdate {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> Self::Raw {
        self.raw
    }
}

impl PushUpdate<'_> {
    /// Returns the source name of the reference as a byte slice.
    pub fn src_refname_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, (*self.raw).src_refname).unwrap() }
    }

    /// Returns the source name of the reference, or None if it is not valid UTF-8.
    pub fn src_refname(&self) -> Option<&str> {
        str::from_utf8(self.src_refname_bytes()).ok()
    }

    /// Returns the name of the reference to update on the server as a byte slice.
    pub fn dst_refname_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, (*self.raw).dst_refname).unwrap() }
    }

    /// Returns the name of the reference to update on the server, or None if it is not valid UTF-8.
    pub fn dst_refname(&self) -> Option<&str> {
        str::from_utf8(self.dst_refname_bytes()).ok()
    }

    /// Returns the current target of the reference.
    pub fn src(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).src as *const _) }
    }

    /// Returns the new target for the reference.
    pub fn dst(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).dst as *const _) }
    }
}
