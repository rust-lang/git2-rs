use std::marker;
use std::ffi::CString;
use libc::c_uint;

use {raw, Error, Sort, Oid, Repository};
use util::Binding;

/// A revwalk allows traversal of the commit graph defined by including one or
/// more leaves and excluding one or more roots.
pub struct Revwalk<'repo> {
    raw: *mut raw::git_revwalk,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> Revwalk<'repo> {
    /// Reset a revwalk to allow re-configuring it.
    ///
    /// The revwalk is automatically reset when iteration of its commits
    /// completes.
    pub fn reset(&mut self) {
        unsafe { raw::git_revwalk_reset(self.raw()) }
    }

    /// Set the order in which commits are visited.
    pub fn set_sorting(&mut self, sort_mode: Sort) {
        unsafe {
            raw::git_revwalk_sorting(self.raw(), sort_mode.bits() as c_uint)
        }
    }

    /// Simplify the history by first-parent
    ///
    /// No parents other than the first for each commit will be enqueued.
    pub fn simplify_first_parent(&mut self) {
        unsafe { raw::git_revwalk_simplify_first_parent(self.raw) }
    }

    /// Mark a commit to start traversal from.
    ///
    /// The given OID must belong to a committish on the walked repository.
    ///
    /// The given commit will be used as one of the roots when starting the
    /// revision walk. At least one commit must be pushed onto the walker before
    /// a walk can be started.
    pub fn push(&mut self, oid: Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_push(self.raw(), oid.raw()));
        }
        Ok(())
    }

    /// Push the repository's HEAD
    ///
    /// For more information, see `push`.
    pub fn push_head(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_push_head(self.raw()));
        }
        Ok(())
    }

    /// Push matching references
    ///
    /// The OIDs pointed to by the references that match the given glob pattern
    /// will be pushed to the revision walker.
    ///
    /// A leading 'refs/' is implied if not present as well as a trailing `/ \
    /// *` if the glob lacks '?', ' \ *' or '['.
    ///
    /// Any references matching this glob which do not point to a committish
    /// will be ignored.
    pub fn push_glob(&mut self, glob: &str) -> Result<(), Error> {
        let glob = try!(CString::new(glob));
        unsafe {
            try_call!(raw::git_revwalk_push_glob(self.raw, glob));
        }
        Ok(())
    }

    /// Push and hide the respective endpoints of the given range.
    ///
    /// The range should be of the form `<commit>..<commit>` where each
    /// `<commit>` is in the form accepted by `revparse_single`. The left-hand
    /// commit will be hidden and the right-hand commit pushed.
    pub fn push_range(&mut self, range: &str) -> Result<(), Error> {
        let range = try!(CString::new(range));
        unsafe {
            try_call!(raw::git_revwalk_push_range(self.raw, range));
        }
        Ok(())
    }

    /// Push the OID pointed to by a reference
    ///
    /// The reference must point to a committish.
    pub fn push_ref(&mut self, reference: &str) -> Result<(), Error> {
        let reference = try!(CString::new(reference));
        unsafe {
            try_call!(raw::git_revwalk_push_ref(self.raw, reference));
        }
        Ok(())
    }

    /// Mark a commit as not of interest to this revwalk.
    pub fn hide(&mut self, oid: Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_hide(self.raw(), oid.raw()));
        }
        Ok(())
    }

    /// Hide the repository's HEAD
    ///
    /// For more information, see `hide`.
    pub fn hide_head(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_hide_head(self.raw()));
        }
        Ok(())
    }

    /// Hide matching references.
    ///
    /// The OIDs pointed to by the references that match the given glob pattern
    /// and their ancestors will be hidden from the output on the revision walk.
    ///
    /// A leading 'refs/' is implied if not present as well as a trailing `/ \
    /// *` if the glob lacks '?', ' \ *' or '['.
    ///
    /// Any references matching this glob which do not point to a committish
    /// will be ignored.
    pub fn hide_glob(&mut self, glob: &str) -> Result<(), Error> {
        let glob = try!(CString::new(glob));
        unsafe {
            try_call!(raw::git_revwalk_hide_glob(self.raw, glob));
        }
        Ok(())
    }

    /// Hide the OID pointed to by a reference.
    ///
    /// The reference must point to a committish.
    pub fn hide_ref(&mut self, reference: &str) -> Result<(), Error> {
        let reference = try!(CString::new(reference));
        unsafe {
            try_call!(raw::git_revwalk_hide_ref(self.raw, reference));
        }
        Ok(())
    }
}

impl<'repo> Binding for Revwalk<'repo> {
    type Raw = *mut raw::git_revwalk;
    unsafe fn from_raw(raw: *mut raw::git_revwalk) -> Revwalk<'repo> {
        Revwalk { raw: raw, _marker: marker::PhantomData }
    }
    fn raw(&self) -> *mut raw::git_revwalk { self.raw }
}

impl<'repo> Drop for Revwalk<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_revwalk_free(self.raw) }
    }
}

impl<'repo> Iterator for Revwalk<'repo> {
    type Item = Oid;
    fn next(&mut self) -> Option<Oid> {
        let mut out: raw::git_oid = raw::git_oid{ id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            match raw::git_revwalk_next(&mut out, self.raw()) {
                0 => (),
                _ => return None,
            }

            Some(Binding::from_raw(&out as *const _))
        }
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use std::vec::{Vec};

    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();

        let mut walk = repo.revwalk().unwrap();
        walk.push(target).unwrap();

        let oids: Vec<::Oid> = walk.by_ref().collect();

        assert_eq!(oids.len(), 1);
        assert_eq!(oids[0], target);

        walk.reset();
        walk.push_head().unwrap();
        assert_eq!(walk.by_ref().count(), 1);

        walk.reset();
        walk.push_head().unwrap();
        walk.hide_head().unwrap();
        assert_eq!(walk.by_ref().count(), 0);
    }
}
