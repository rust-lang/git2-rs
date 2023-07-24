use libc::{c_int, c_uint, c_void};
use std::ffi::CString;
use std::marker;

use crate::util::Binding;
use crate::{panic, raw, Error, Oid, Repository, Sort};

/// A revwalk allows traversal of the commit graph defined by including one or
/// more leaves and excluding one or more roots.
pub struct Revwalk<'repo> {
    raw: *mut raw::git_revwalk,
    _marker: marker::PhantomData<&'repo Repository>,
}

/// A `Revwalk` with an associated "hide callback", see `with_hide_callback`
pub struct RevwalkWithHideCb<'repo, 'cb, C>
where
    C: FnMut(Oid) -> bool,
{
    revwalk: Revwalk<'repo>,
    _marker: marker::PhantomData<&'cb C>,
}

extern "C" fn revwalk_hide_cb<C>(commit_id: *const raw::git_oid, payload: *mut c_void) -> c_int
where
    C: FnMut(Oid) -> bool,
{
    panic::wrap(|| unsafe {
        let hide_cb = payload as *mut C;
        if (*hide_cb)(Oid::from_raw(commit_id)) {
            1
        } else {
            0
        }
    })
    .unwrap_or(-1)
}

impl<'repo, 'cb, C: FnMut(Oid) -> bool> RevwalkWithHideCb<'repo, 'cb, C> {
    /// Consumes the `RevwalkWithHideCb` and returns the contained `Revwalk`.
    ///
    /// Note that this will reset the `Revwalk`.
    pub fn into_inner(mut self) -> Result<Revwalk<'repo>, Error> {
        self.revwalk.reset()?;
        Ok(self.revwalk)
    }
}

impl<'repo> Revwalk<'repo> {
    /// Reset a revwalk to allow re-configuring it.
    ///
    /// The revwalk is automatically reset when iteration of its commits
    /// completes.
    pub fn reset(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_reset(self.raw()));
        }
        Ok(())
    }

    /// Set the order in which commits are visited.
    pub fn set_sorting(&mut self, sort_mode: Sort) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_sorting(
                self.raw(),
                sort_mode.bits() as c_uint
            ));
        }
        Ok(())
    }

    /// Simplify the history by first-parent
    ///
    /// No parents other than the first for each commit will be enqueued.
    pub fn simplify_first_parent(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_simplify_first_parent(self.raw));
        }
        Ok(())
    }

    /// Mark a commit to start traversal from.
    ///
    /// The given OID must belong to a commitish on the walked repository.
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
    /// Any references matching this glob which do not point to a commitish
    /// will be ignored.
    pub fn push_glob(&mut self, glob: &str) -> Result<(), Error> {
        let glob = CString::new(glob)?;
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
        let range = CString::new(range)?;
        unsafe {
            try_call!(raw::git_revwalk_push_range(self.raw, range));
        }
        Ok(())
    }

    /// Push the OID pointed to by a reference
    ///
    /// The reference must point to a commitish.
    pub fn push_ref(&mut self, reference: &str) -> Result<(), Error> {
        let reference = CString::new(reference)?;
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

    /// Hide all commits for which the callback returns true from
    /// the walk.
    pub fn with_hide_callback<'cb, C>(
        self,
        callback: &'cb mut C,
    ) -> Result<RevwalkWithHideCb<'repo, 'cb, C>, Error>
    where
        C: FnMut(Oid) -> bool,
    {
        let r = RevwalkWithHideCb {
            revwalk: self,
            _marker: marker::PhantomData,
        };
        unsafe {
            raw::git_revwalk_add_hide_cb(
                r.revwalk.raw(),
                Some(revwalk_hide_cb::<C>),
                callback as *mut _ as *mut c_void,
            );
        };
        Ok(r)
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
    /// Any references matching this glob which do not point to a commitish
    /// will be ignored.
    pub fn hide_glob(&mut self, glob: &str) -> Result<(), Error> {
        let glob = CString::new(glob)?;
        unsafe {
            try_call!(raw::git_revwalk_hide_glob(self.raw, glob));
        }
        Ok(())
    }

    /// Hide the OID pointed to by a reference.
    ///
    /// The reference must point to a commitish.
    pub fn hide_ref(&mut self, reference: &str) -> Result<(), Error> {
        let reference = CString::new(reference)?;
        unsafe {
            try_call!(raw::git_revwalk_hide_ref(self.raw, reference));
        }
        Ok(())
    }
}

impl<'repo> Binding for Revwalk<'repo> {
    type Raw = *mut raw::git_revwalk;
    unsafe fn from_raw(raw: *mut raw::git_revwalk) -> Revwalk<'repo> {
        Revwalk {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_revwalk {
        self.raw
    }
}

impl<'repo> Drop for Revwalk<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_revwalk_free(self.raw) }
    }
}

impl<'repo> Iterator for Revwalk<'repo> {
    type Item = Result<Oid, Error>;
    fn next(&mut self) -> Option<Result<Oid, Error>> {
        let mut out: raw::git_oid = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call_iter!(raw::git_revwalk_next(&mut out, self.raw()));
            Some(Ok(Binding::from_raw(&out as *const _)))
        }
    }
}

impl<'repo, 'cb, C: FnMut(Oid) -> bool> Iterator for RevwalkWithHideCb<'repo, 'cb, C> {
    type Item = Result<Oid, Error>;
    fn next(&mut self) -> Option<Result<Oid, Error>> {
        let out = self.revwalk.next();
        crate::panic::check();
        out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();

        let mut walk = repo.revwalk().unwrap();
        walk.push(target).unwrap();

        let oids: Vec<crate::Oid> = walk.by_ref().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(oids.len(), 1);
        assert_eq!(oids[0], target);

        walk.reset().unwrap();
        walk.push_head().unwrap();
        assert_eq!(walk.by_ref().count(), 1);

        walk.reset().unwrap();
        walk.push_head().unwrap();
        walk.hide_head().unwrap();
        assert_eq!(walk.by_ref().count(), 0);
    }

    #[test]
    fn smoke_hide_cb() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();

        let mut walk = repo.revwalk().unwrap();
        walk.push(target).unwrap();

        let oids: Vec<crate::Oid> = walk.by_ref().collect::<Result<Vec<_>, _>>().unwrap();

        assert_eq!(oids.len(), 1);
        assert_eq!(oids[0], target);

        walk.reset().unwrap();
        walk.push_head().unwrap();
        assert_eq!(walk.by_ref().count(), 1);

        walk.reset().unwrap();
        walk.push_head().unwrap();

        let mut hide_cb = |oid| oid == target;
        let mut walk = walk.with_hide_callback(&mut hide_cb).unwrap();

        assert_eq!(walk.by_ref().count(), 0);

        let mut walk = walk.into_inner().unwrap();
        walk.push_head().unwrap();
        assert_eq!(walk.by_ref().count(), 1);
    }
}
