extern crate libc;

use std::kinds::marker;

use {raw, Error, Repository, Sort, Oid};

/// A revwalk allows traversal of the commit graph defined by including one or
/// more leaves and excluding one or more roots.
pub struct Revwalk<'a> {
    raw: *mut raw::git_revwalk,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'a> Revwalk<'a> {
    /// Creates a new revwalk from its raw pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_revwalk) -> Revwalk {
        Revwalk {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_revwalk { self.raw }

    /// Reset a revwalk to allow re-configuring it.
    ///
    /// The revwalk is automatically reset when iteration of its commits
    /// completes.
    pub fn reset(&mut self) {
        unsafe { raw::git_revwalk_reset(self.raw()) }
    }

    /// Set the order in which commits are visited.
    pub fn set_sorting(&mut self, sort_mode: Sort) {
        unsafe { raw::git_revwalk_sorting(self.raw(), sort_mode.bits() as libc::c_uint) }
    }

    /// Mark a commit as of interest to this revwalk.
    pub fn push(&mut self, oid: &Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_push(self.raw(), oid.raw()));
        }
        Ok(())
    }

    /// Mark a commit as not of interest to this revwalk.
    pub fn hide(&mut self, oid: &Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_revwalk_hide(self.raw(), oid.raw()));
        }
        Ok(())
    }
}

#[unsafe_destructor]
impl<'a> Drop for Revwalk<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_revwalk_free(self.raw) }
    }
}

impl<'a> Iterator<Oid> for Revwalk<'a> {
    fn next(&mut self) -> Option<Oid> {
        let mut out: raw::git_oid = raw::git_oid{ id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            match raw::git_revwalk_next(&mut out, self.raw()) {
                0 => (),
                _ => return None,
            }

            Some(Oid::from_raw(&out))
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
        walk.push(&target).unwrap();

        let oids: Vec<::Oid> = walk.collect();

        assert_eq!(oids.len(), 1);
        assert_eq!(oids[0], target);
    }
}
