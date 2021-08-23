use std::ffi::CString;
use std::marker;
use std::ptr;
use std::str;

use crate::util::Binding;
use crate::{raw, BranchType, Error, Reference, References};

/// A structure to represent a git [branch][1]
///
/// A branch is currently just a wrapper to an underlying `Reference`. The
/// reference can be accessed through the `get` and `into_reference` methods.
///
/// [1]: http://git-scm.com/book/en/Git-Branching-What-a-Branch-Is
pub struct Branch<'repo> {
    inner: Reference<'repo>,
}

/// An iterator over the branches inside of a repository.
pub struct Branches<'repo> {
    raw: *mut raw::git_branch_iterator,
    _marker: marker::PhantomData<References<'repo>>,
}

impl<'repo> Branch<'repo> {
    /// Creates Branch type from a Reference
    pub fn wrap(reference: Reference<'_>) -> Branch<'_> {
        Branch { inner: reference }
    }

    /// Ensure the branch name is well-formed.
    pub fn name_is_valid(name: &str) -> Result<bool, Error> {
        crate::init();
        let name = CString::new(name)?;
        let mut valid: libc::c_int = 0;
        unsafe {
            try_call!(raw::git_branch_name_is_valid(&mut valid, name.as_ptr()));
        }
        Ok(valid == 1)
    }

    /// Gain access to the reference that is this branch
    pub fn get(&self) -> &Reference<'repo> {
        &self.inner
    }

    /// Gain mutable access to the reference that is this branch
    pub fn get_mut(&mut self) -> &mut Reference<'repo> {
        &mut self.inner
    }

    /// Take ownership of the underlying reference.
    pub fn into_reference(self) -> Reference<'repo> {
        self.inner
    }

    /// Delete an existing branch reference.
    pub fn delete(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_branch_delete(self.get().raw()));
        }
        Ok(())
    }

    /// Determine if the current local branch is pointed at by HEAD.
    pub fn is_head(&self) -> bool {
        unsafe { raw::git_branch_is_head(&*self.get().raw()) == 1 }
    }

    /// Move/rename an existing local branch reference.
    pub fn rename(&mut self, new_branch_name: &str, force: bool) -> Result<Branch<'repo>, Error> {
        let mut ret = ptr::null_mut();
        let new_branch_name = CString::new(new_branch_name)?;
        unsafe {
            try_call!(raw::git_branch_move(
                &mut ret,
                self.get().raw(),
                new_branch_name,
                force
            ));
            Ok(Branch::wrap(Binding::from_raw(ret)))
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
        let mut ret = ptr::null();
        unsafe {
            try_call!(raw::git_branch_name(&mut ret, &*self.get().raw()));
            Ok(crate::opt_bytes(self, ret).unwrap())
        }
    }

    /// Return the reference supporting the remote tracking branch, given a
    /// local branch reference.
    pub fn upstream(&self) -> Result<Branch<'repo>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_branch_upstream(&mut ret, &*self.get().raw()));
            Ok(Branch::wrap(Binding::from_raw(ret)))
        }
    }

    /// Set the upstream configuration for a given local branch.
    ///
    /// If `None` is specified, then the upstream branch is unset. The name
    /// provided is the name of the branch to set as upstream.
    pub fn set_upstream(&mut self, upstream_name: Option<&str>) -> Result<(), Error> {
        let upstream_name = crate::opt_cstr(upstream_name)?;
        unsafe {
            try_call!(raw::git_branch_set_upstream(
                self.get().raw(),
                upstream_name
            ));
            Ok(())
        }
    }
}

impl<'repo> Branches<'repo> {
    /// Creates a new iterator from the raw pointer given.
    ///
    /// This function is unsafe as it is not guaranteed that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_branch_iterator) -> Branches<'repo> {
        Branches {
            raw,
            _marker: marker::PhantomData,
        }
    }
}

impl<'repo> Iterator for Branches<'repo> {
    type Item = Result<(Branch<'repo>, BranchType), Error>;
    fn next(&mut self) -> Option<Result<(Branch<'repo>, BranchType), Error>> {
        let mut ret = ptr::null_mut();
        let mut typ = raw::GIT_BRANCH_LOCAL;
        unsafe {
            try_call_iter!(raw::git_branch_next(&mut ret, &mut typ, self.raw));
            let typ = match typ {
                raw::GIT_BRANCH_LOCAL => BranchType::Local,
                raw::GIT_BRANCH_REMOTE => BranchType::Remote,
                n => panic!("unexected branch type: {}", n),
            };
            Some(Ok((Branch::wrap(Binding::from_raw(ret)), typ)))
        }
    }
}

impl<'repo> Drop for Branches<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_branch_iterator_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Branch, BranchType};

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let mut b1 = repo.branch("foo", &commit, false).unwrap();
        assert!(!b1.is_head());
        repo.branch("foo2", &commit, false).unwrap();

        assert_eq!(repo.branches(None).unwrap().count(), 3);
        repo.find_branch("foo", BranchType::Local).unwrap();
        let mut b1 = b1.rename("bar", false).unwrap();
        assert_eq!(b1.name().unwrap(), Some("bar"));
        assert!(b1.upstream().is_err());
        b1.set_upstream(Some("main")).unwrap();
        b1.upstream().unwrap();
        b1.set_upstream(None).unwrap();

        b1.delete().unwrap();
    }

    #[test]
    fn name_is_valid() {
        assert!(Branch::name_is_valid("foo").unwrap());
        assert!(!Branch::name_is_valid("").unwrap());
        assert!(!Branch::name_is_valid("with spaces").unwrap());
        assert!(!Branch::name_is_valid("~tilde").unwrap());
    }
}
