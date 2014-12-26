use std::kinds::marker;
use std::mem;

use {raw, Oid, ObjectType, Error, Buf};

/// A structure to represent a git [object][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Object<'repo> {
    raw: *mut raw::git_object,
    marker1: marker::ContravariantLifetime<'repo>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'repo> Object<'repo> {
    /// Create a new object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_object) -> Object<'repo> {
        Object {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Get the id (SHA1) of a repository object
    pub fn id(&self) -> Oid {
        unsafe {
            Oid::from_raw(raw::git_object_id(&*self.raw))
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_object { self.raw }

    /// Get the object type of an object.
    ///
    /// If the type is unknown, then `None` is returned.
    pub fn kind(&self) -> Option<ObjectType> {
        ObjectType::from_raw(unsafe { raw::git_object_type(&*self.raw) })
    }

    /// Recursively peel an object until an object of the specified type is met.
    ///
    /// If you pass `Any` as the target type, then the object will be
    /// peeled until the type changes (e.g. a tag will be chased until the
    /// referenced object is no longer a tag).
    pub fn peel(&self, kind: ObjectType) -> Result<Object, Error> {
        let mut raw = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_object_peel(&mut raw, &*self.raw(), kind));
        }
        Ok(Object {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        })
    }

    /// Get a short abbreviated OID string for the object
    ///
    /// This starts at the "core.abbrev" length (default 7 characters) and
    /// iteratively extends to a longer string if that length is ambiguous. The
    /// result will be unambiguous (at least until new objects are added to the
    /// repository).
    pub fn short_id(&self) -> Result<Buf, Error> {
        unsafe {
            let mut raw: raw::git_buf = mem::zeroed();
            try_call!(raw::git_object_short_id(&mut raw, &*self.raw()));
            Ok(Buf::from_raw(raw))
        }
    }
}

impl<'a> Clone for Object<'a> {
    fn clone(&self) -> Object<'a> {
        let mut raw = 0 as *mut raw::git_object;
        let rc = unsafe { raw::git_object_dup(&mut raw, self.raw) };
        assert_eq!(rc, 0);
        Object {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Object<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_object_free(self.raw) }
    }
}
