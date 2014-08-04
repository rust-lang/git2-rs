use std::kinds::marker;

use {raw, Oid, Repository, ObjectKind, Error};

/// A structure to represent a git [object][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Object<'a> {
    raw: *mut raw::git_object,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoShare,
}

impl<'a> Object<'a> {
    /// Create a new object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_object) -> Object {
        Object {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn lookup(repo: &Repository, oid: Oid,
                  kind: Option<ObjectKind>) -> Result<Object, Error> {
        let mut raw = 0 as *mut raw::git_object;
        let kind = match kind {
            Some(::Any) => raw::GIT_OBJ_ANY,
            Some(::Commit) => raw::GIT_OBJ_COMMIT,
            Some(::Tree) => raw::GIT_OBJ_TREE,
            Some(::Blob) => raw::GIT_OBJ_BLOB,
            Some(::Tag) => raw::GIT_OBJ_TAG,
            None => raw::GIT_OBJ_ANY,
        };

        try!(::doit(|| unsafe {
            raw::git_object_lookup(&mut raw, repo.raw(), oid.raw(), kind)
        }));
        Ok(unsafe { Object::from_raw(repo, raw) })
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
    pub fn kind(&self) -> Option<ObjectKind> {
        match unsafe { raw::git_object_type(&*self.raw) } {
            raw::GIT_OBJ_ANY => Some(::Any),
            raw::GIT_OBJ_BAD => None,
            raw::GIT_OBJ__EXT1 => None,
            raw::GIT_OBJ_COMMIT => Some(::Commit),
            raw::GIT_OBJ_TREE => Some(::Tree),
            raw::GIT_OBJ_BLOB => Some(::Blob),
            raw::GIT_OBJ_TAG => Some(::Tag),
            raw::GIT_OBJ__EXT2 => None,
            raw::GIT_OBJ_OFS_DELTA => None,
            raw::GIT_OBJ_REF_DELTA => None,
        }
    }
}

impl<'a> Clone for Object<'a> {
    fn clone(&self) -> Object<'a> {
        let mut raw = 0 as *mut raw::git_object;
        ::doit(|| unsafe { raw::git_object_dup(&mut raw, self.raw) }).unwrap();
        Object {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Object<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_object_free(self.raw) }
    }
}
