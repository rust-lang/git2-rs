use {raw, Oid};

pub struct Object {
    raw: *mut raw::git_object,
}

impl Object {
    pub unsafe fn from_raw(raw: *mut raw::git_object) -> Object {
        Object { raw: raw }
    }

    pub fn id(&self) -> Oid {
        unsafe {
            Oid::from_raw(raw::git_object_id(&*self.raw))
        }
    }
}

impl Drop for Object {
    fn drop(&mut self) {
        unsafe {
            raw::git_object_free(self.raw);
        }
    }
}
