use {raw};

use util::Binding;

/// A structure to represent a git object database backend.
pub struct OdbBackendHolder {
    raw: *mut raw::git_odb_backend
}

impl Binding for OdbBackendHolder {
    type Raw = *mut raw::git_odb_backend;

    unsafe fn from_raw(raw: *mut raw::git_odb_backend) -> OdbBackendHolder {
        OdbBackendHolder {
            raw: raw
        }
    }

    fn raw(&self) -> *mut raw::git_odb_backend { self.raw }
}
