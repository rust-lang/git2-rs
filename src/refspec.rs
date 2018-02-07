use std::os::raw::c_int;
use std::ffi::CString;
use std::marker;
use std::ptr;
use std::str;

use {raw, Direction};
use util::Binding;

enum RefspecInner {
    Raw(*const raw::git_refspec),
    Owned(Box<raw::git_refspec>),
}

/// A structure to represent a git [refspec][1].
///
/// Refspecs are currently mainly accessed/created through a `Remote`.
///
/// [1]: http://git-scm.com/book/en/Git-Internals-The-Refspec
pub struct Refspec<'remote> {
    inner: RefspecInner,
    _marker: marker::PhantomData<&'remote raw::git_remote>,
}

impl<'remote> Drop for Refspec<'remote> {
    fn drop(&mut self) {
        match self.inner {
            RefspecInner::Owned(ref mut owned) => unsafe {
                raw::git_refspec__free(&mut **owned as *mut raw::git_refspec)
            },
            _ => {}
        }
    }
}

impl<'remote> Refspec<'remote> {
    /// Get a Refspec from a given refspec string.
    ///
    /// If the string could not be parsed, None is returned.
    pub fn try_parse(refspec_str: &str, is_fetch: bool) -> Option<Refspec> {
        let mut refspec = raw::git_refspec {
            string: ptr::null_mut(),
            src: ptr::null_mut(),
            dst: ptr::null_mut(),
            flags: 0,
        };
        let cstr = CString::new(refspec_str).unwrap();
        unsafe {
            if raw::git_refspec__parse(
                &mut refspec as *mut raw::git_refspec,
                cstr.as_ptr(),
                is_fetch as c_int,
            ) == 0
            {
                Some(Refspec {
                    inner: RefspecInner::Owned(Box::from(refspec)),
                    _marker: marker::PhantomData,
                })
            } else {
                None
            }
        }
    }

    fn get_handle(&self) -> *const raw::git_refspec {
        match self.inner {
            RefspecInner::Raw(raw) => raw,
            RefspecInner::Owned(ref owned) => &**owned as *const raw::git_refspec,
        }
    }

    /// Get the refspec's direction.
    pub fn direction(&self) -> Direction {
        match unsafe { raw::git_refspec_direction(self.get_handle()) } {
            raw::GIT_DIRECTION_FETCH => Direction::Fetch,
            raw::GIT_DIRECTION_PUSH => Direction::Push,
            n => panic!("unknown refspec direction: {}", n),
        }
    }

    /// Get the destination specifier.
    ///
    /// If the destination is not utf-8, None is returned.
    pub fn dst(&self) -> Option<&str> {
        str::from_utf8(self.dst_bytes()).ok()
    }

    /// Get the destination specifier, in bytes.
    pub fn dst_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_refspec_dst(self.get_handle())).unwrap() }
    }

    /// Check if a refspec's destination descriptor matches a reference
    pub fn dst_matches(&self, refname: &str) -> bool {
        let refname = CString::new(refname).unwrap();
        unsafe { raw::git_refspec_dst_matches(self.get_handle(), refname.as_ptr()) == 1 }
    }

    /// Get the source specifier.
    ///
    /// If the source is not utf-8, None is returned.
    pub fn src(&self) -> Option<&str> {
        str::from_utf8(self.src_bytes()).ok()
    }

    /// Get the source specifier, in bytes.
    pub fn src_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_refspec_src(self.get_handle())).unwrap() }
    }

    /// Check if a refspec's source descriptor matches a reference
    pub fn src_matches(&self, refname: &str) -> bool {
        let refname = CString::new(refname).unwrap();
        unsafe { raw::git_refspec_src_matches(self.get_handle(), refname.as_ptr()) == 1 }
    }

    /// Get the force update setting.
    pub fn is_force(&self) -> bool {
        unsafe { raw::git_refspec_force(self.get_handle()) == 1 }
    }

    /// Get the refspec's string.
    ///
    /// Returns None if the string is not valid utf8.
    pub fn str(&self) -> Option<&str> {
        str::from_utf8(self.bytes()).ok()
    }

    /// Get the refspec's string as a byte array
    pub fn bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_refspec_string(self.get_handle())).unwrap() }
    }
}

impl<'remote> Binding for Refspec<'remote> {
    type Raw = *const raw::git_refspec;

    unsafe fn from_raw(raw: *const raw::git_refspec) -> Refspec<'remote> {
        Refspec {
            inner: RefspecInner::Raw(raw),
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *const raw::git_refspec {
        match self.inner {
            RefspecInner::Raw(raw) => raw,
            RefspecInner::Owned(_) => panic!("this refspec contains owned data"),
        }
    }
}
