use std::kinds::marker;

use {raw, Oid, Repository};

pub struct Object<'a> {
    raw: *mut raw::git_object,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoShare,
}

impl<'a> Object<'a> {
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_object) -> Object {
        Object {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }

    pub fn id(&self) -> Oid {
        unsafe {
            Oid::from_raw(raw::git_object_id(&*self.raw))
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Object<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_object_free(self.raw) }
    }
}
