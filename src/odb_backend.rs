use {raw};

use util::Binding;

/// A structure to represent a git object database backend.
pub struct OdbBackend {
    raw: *mut raw::git_odb_backend
}

impl Binding for OdbBackend {
    type Raw = *mut raw::git_odb_backend;

    unsafe fn from_raw(raw: *mut raw::git_odb_backend) -> OdbBackend {
        OdbBackend {
            raw: raw
        }
    }

    fn raw(&self) -> *mut raw::git_odb_backend { self.raw }
}
