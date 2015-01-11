use std::marker;
use {raw, Oid, ObjectType, Error, Buf};
use util::Binding;

/// A structure to represent a git [object][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Object<'repo> {
    raw: *mut raw::git_object,
    marker: marker::ContravariantLifetime<'repo>,
}

impl<'repo> Object<'repo> {
    /// Get the id (SHA1) of a repository object
    pub fn id(&self) -> Oid {
        unsafe {
            Binding::from_raw(raw::git_object_id(&*self.raw))
        }
    }

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
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get a short abbreviated OID string for the object
    ///
    /// This starts at the "core.abbrev" length (default 7 characters) and
    /// iteratively extends to a longer string if that length is ambiguous. The
    /// result will be unambiguous (at least until new objects are added to the
    /// repository).
    pub fn short_id(&self) -> Result<Buf, Error> {
        unsafe {
            let buf = Buf::new();
            try_call!(raw::git_object_short_id(buf.raw(), &*self.raw()));
            Ok(buf)
        }
    }
}

impl<'repo> Clone for Object<'repo> {
    fn clone(&self) -> Object<'repo> {
        let mut raw = 0 as *mut raw::git_object;
        let rc = unsafe { raw::git_object_dup(&mut raw, self.raw) };
        assert_eq!(rc, 0);
        Object {
            raw: raw,
            marker: marker::ContravariantLifetime,
        }
    }
}

impl<'repo> Binding for Object<'repo> {
    type Raw = *mut raw::git_object;

    unsafe fn from_raw(raw: *mut raw::git_object) -> Object<'repo> {
        Object {
            raw: raw,
            marker: marker::ContravariantLifetime,
        }
    }
    fn raw(&self) -> *mut raw::git_object { self.raw }
}

#[unsafe_destructor]
impl<'repo> Drop for Object<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_object_free(self.raw) }
    }
}
