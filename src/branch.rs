use std::str;
use libc;

use {raw, Repository, Error, Reference, Signature, BranchType};

/// A structure to represent a git [branch][1]
///
/// A branch is currently just a wrapper to an underlying `Reference`. The
/// reference can be accessed through the `get` and `unwrap` methods.
///
/// [1]: http://git-scm.com/book/en/Git-Branching-What-a-Branch-Is
pub struct Branch<'repo> {
    inner: Reference<'repo>,
}

/// An iterator over the branches inside of a repository.
pub struct Branches<'repo> {
    repo: &'repo Repository,
    raw: *mut raw::git_branch_iterator,
}

impl<'repo> Branch<'repo> {
    /// Creates a new branch from a reference
    pub fn wrap(reference: Reference) -> Branch { Branch { inner: reference } }

    /// Gain access to the reference that is this branch
    pub fn get(&self) -> &Reference<'repo> { &self.inner }

    /// Take ownership of the underlying reference.
    pub fn into_reference(self) -> Reference<'repo> { self.inner }

    /// Delete an existing branch reference.
    pub fn delete(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_branch_delete(self.get().raw())); }
        Ok(())
    }

    /// Determine if the current local branch is pointed at by HEAD.
    pub fn is_head(&self) -> bool {
        unsafe { raw::git_branch_is_head(&*self.get().raw()) == 1 }
    }

    /// Move/rename an existing local branch reference.
    pub fn rename(&mut self, new_branch_name: &str, force: bool,
                  signature: Option<&Signature>,
                  log_message: &str) -> Result<Branch<'repo>, Error> {
        let mut ret = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_branch_move(&mut ret, self.get().raw(),
                                           new_branch_name.to_c_str(),
                                           force,
                                           &*signature.map(|s| s.raw())
                                                      .unwrap_or(0 as *mut _),
                                           log_message.to_c_str()));
            Ok(Branch::wrap(Reference::from_raw_ptr(ret)))
        }
    }

    /// Return the name of the given local or remote branch.
    ///
    /// May return `Ok(None)` if the name is not valid utf-8.
    pub fn name(&self) -> Result<Option<&str>, Error> {
        self.name_bytes().map(|s| str::from_utf8(s).ok())
    }

    /// Return the name of the given local or remote branch.
    pub fn name_bytes(&self) -> Result<&[u8], Error> {
        let mut ret = 0 as *const libc::c_char;
        unsafe {
            try_call!(raw::git_branch_name(&mut ret, &*self.get().raw()));
            Ok(::opt_bytes(self, ret).unwrap())
        }
    }

    /// Return the reference supporting the remote tracking branch, given a
    /// local branch reference.
    pub fn upstream<'a>(&'a self) -> Result<Branch<'a>, Error> {
        let mut ret = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_branch_upstream(&mut ret, &*self.get().raw()));
            Ok(Branch::wrap(Reference::from_raw_ptr(ret)))
        }
    }

    /// Set the upstream configuration for a given local branch.
    ///
    /// If `None` is specified, then the upstream branch is unset. The name
    /// provided is the name of the branch to set as upstream.
    pub fn set_upstream(&mut self,
                        upstream_name: Option<&str>) -> Result<(), Error> {
        let upstream_name = upstream_name.map(|s| s.to_c_str());
        unsafe {
            try_call!(raw::git_branch_set_upstream(self.get().raw(),
                                                   upstream_name));
            Ok(())
        }
    }
}

impl<'a> Branches<'a> {
    /// Creates a new iterator from the raw pointer given.
    ///
    /// This function is unsafe as it is not guaranteed that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(repo: &Repository,
                           raw: *mut raw::git_branch_iterator) -> Branches {
        Branches { repo: repo, raw: raw }
    }
}

impl<'a> Iterator<(Branch<'a>, BranchType)> for Branches<'a> {
    fn next(&mut self) -> Option<(Branch<'a>, BranchType)> {
        let mut ret = 0 as *mut raw::git_reference;
        let mut typ = raw::GIT_BRANCH_LOCAL;
        unsafe {
            let rc = raw::git_branch_next(&mut ret, &mut typ, self.raw);
            if rc == raw::GIT_ITEROVER as libc::c_int {
                return None
            }
            assert_eq!(rc, 0);
            let typ = match typ {
                raw::GIT_BRANCH_LOCAL => BranchType::Local,
                raw::GIT_BRANCH_REMOTE => BranchType::Remote,
                raw::GIT_BRANCH_ALL => panic!("unexected branch type"),
            };
            Some((Branch::wrap(Reference::from_raw(self.repo, ret)), typ))
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Branches<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_branch_iterator_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use BranchType;

    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let sig = repo.signature().unwrap();
        let mut b1 = repo.branch("foo", &commit, false, None, Some("bar")).unwrap();
        assert!(!b1.is_head());
        repo.branch("foo2", &commit, false, None, None).unwrap();

        assert_eq!(repo.branches(None).unwrap().count(), 3);
        repo.find_branch("foo", BranchType::Local).unwrap();
        let mut b1 = b1.rename("bar", false, Some(&sig), "bar2").unwrap();
        assert_eq!(b1.name().unwrap(), Some("bar"));
        assert!(b1.upstream().is_err());
        b1.set_upstream(Some("master")).unwrap();
        b1.upstream().unwrap();
        b1.set_upstream(None).unwrap();

        b1.delete().unwrap();
    }
}
