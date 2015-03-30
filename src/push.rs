use std::ffi::{CStr, CString};
use std::marker;
use std::str;
use libc;

use {raw, Error, Signature};
use util::Binding;

/// A structure to represent a pending push operation to a remote.
///
/// Remotes can create a `Push` which is then used to push data to the upstream
/// repository.
pub struct Push<'remote> {
    raw: *mut raw::git_push,
    _marker: marker::PhantomData<&'remote raw::git_remote>,
}

/// A status representing the result of updating a remote reference.
pub struct PushStatus {
    /// The reference that was updated as part of a push.
    pub reference: String,
    /// If `None`, the reference was updated successfully, otherwise a message
    /// explaining why it could not be updated is provided.
    pub message: Option<String>,
}

impl<'remote> Push<'remote> {
    /// Add a refspec to be pushed
    pub fn add_refspec(&mut self, refspec: &str) -> Result<(), Error> {
        let refspec = try!(CString::new(refspec));
        unsafe {
            try_call!(raw::git_push_add_refspec(self.raw, refspec));
            Ok(())
        }
    }

    /// Actually push all given refspecs
    ///
    /// To check if the push was successful (i.e. all remote references have
    /// been updated as requested), you need to call
    /// `statuses`. The remote repository might have refused to
    /// update some or all of the references.
    pub fn finish(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_push_finish(self.raw));
            Ok(())
        }
    }

    /// Update remote tips after a push
    pub fn update_tips(&mut self, signature: Option<&Signature>,
                       reflog_message: Option<&str>) -> Result<(), Error> {
        let msg = try!(::opt_cstr(reflog_message));
        unsafe {
            try_call!(raw::git_push_update_tips(self.raw,
                                                signature.map(|s| s.raw()),
                                                msg));
            Ok(())
        }
    }

    /// Return each status entry
    pub fn statuses(&mut self) -> Result<Vec<PushStatus>, Error> {
        let mut ret: Vec<PushStatus> = Vec::new();
        unsafe {
            try_call!(raw::git_push_status_foreach(self.raw, cb,
                                                   &mut ret as *mut _
                                                            as *mut libc::c_void));
        }
        return Ok(ret);

        extern fn cb(git_ref: *const libc::c_char,
                     msg: *const libc::c_char,
                     data: *mut libc::c_void) -> libc::c_int {
            unsafe {
                let git_ref = match str::from_utf8(CStr::from_ptr(git_ref).to_bytes()) {
                    Ok(s) => s.to_string(),
                    Err(_) => return 0,
                };
                let msg = if !msg.is_null() {
                    match str::from_utf8(CStr::from_ptr(msg).to_bytes()) {
                        Ok(s) => Some(s.to_string()),
                        Err(_) => return 0,
                    }
                } else {
                    None
                };

                let data = &mut *(data as *mut Vec<PushStatus>);
                data.push(PushStatus { reference: git_ref, message: msg });
                return 0;
            }
        }
    }
}

impl<'remote> Binding for Push<'remote> {
    type Raw = *mut raw::git_push;
    unsafe fn from_raw(raw: *mut raw::git_push) -> Push<'remote> {
        Push { raw: raw, _marker: marker::PhantomData }
    }
    fn raw(&self) -> *mut raw::git_push { self.raw }
}

impl<'a> Drop for Push<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_push_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use tempdir::TempDir;
    use Repository;

    #[test]
    fn smoke() {
        let td = TempDir::new("test").unwrap();
        let remote = td.path().join("remote");
        Repository::init_bare(&remote).unwrap();

        let (_td, repo) = ::test::repo_init();
        let url = ::test::path2url(&remote);
        let mut remote = repo.remote("origin", &url).unwrap();

        let mut push = remote.push().unwrap();
        push.add_refspec("refs/heads/master").unwrap();
        push.finish().unwrap();
        push.update_tips(None, None).unwrap();
        let v = push.statuses().unwrap();
        assert!(v.len() > 0);
        assert_eq!(v[0].reference, "refs/heads/master");
        assert!(v[0].message.is_none());
    }
}
