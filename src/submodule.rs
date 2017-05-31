use std::marker;
use std::ptr;
use std::str;
use std::path::Path;

use {raw, Oid, Repository, Error};
use util::{self, Binding};

/// A structure to represent a git [submodule][1]
///
/// [1]: http://git-scm.com/book/en/Git-Tools-Submodules
pub struct Submodule<'repo> {
    raw: *mut raw::git_submodule,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> Submodule<'repo> {
    /// Get the submodule's branch.
    ///
    /// Returns `None` if the branch is not valid utf-8 or if the branch is not
    /// yet available.
    pub fn branch(&self) -> Option<&str> {
        self.branch_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the branch for the submodule.
    ///
    /// Returns `None` if the branch is not yet available.
    pub fn branch_bytes(&self) -> Option<&[u8]> {
        unsafe {
            ::opt_bytes(self, raw::git_submodule_branch(self.raw))
        }
    }

    /// Get the submodule's url.
    ///
    /// Returns `None` if the url is not valid utf-8
    pub fn url(&self) -> Option<&str> { str::from_utf8(self.url_bytes()).ok() }

    /// Get the url for the submodule.
    pub fn url_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_submodule_url(self.raw)).unwrap()
        }
    }

    /// Get the submodule's name.
    ///
    /// Returns `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> { str::from_utf8(self.name_bytes()).ok() }

    /// Get the name for the submodule.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_submodule_name(self.raw)).unwrap()
        }
    }

    /// Get the path for the submodule.
    pub fn path(&self) -> &Path {
        util::bytes2path(unsafe {
            ::opt_bytes(self, raw::git_submodule_path(self.raw)).unwrap()
        })
    }

    /// Get the OID for the submodule in the current HEAD tree.
    pub fn head_id(&self) -> Option<Oid> {
        unsafe {
            Binding::from_raw_opt(raw::git_submodule_head_id(self.raw))
        }
    }

    /// Get the OID for the submodule in the index.
    pub fn index_id(&self) -> Option<Oid> {
        unsafe {
            Binding::from_raw_opt(raw::git_submodule_index_id(self.raw))
        }
    }

    /// Get the OID for the submodule in the current working directory.
    ///
    /// This returns the OID that corresponds to looking up 'HEAD' in the
    /// checked out submodule. If there are pending changes in the index or
    /// anything else, this won't notice that.
    pub fn workdir_id(&self) -> Option<Oid> {
        unsafe {
            Binding::from_raw_opt(raw::git_submodule_wd_id(self.raw))
        }
    }

    /// Copy submodule info into ".git/config" file.
    ///
    /// Just like "git submodule init", this copies information about the
    /// submodule into ".git/config". You can use the accessor functions above
    /// to alter the in-memory git_submodule object and control what is written
    /// to the config, overriding what is in .gitmodules.
    ///
    /// By default, existing entries will not be overwritten, but passing `true`
    /// for `overwrite` forces them to be updated.
    pub fn init(&mut self, overwrite: bool) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_submodule_init(self.raw, overwrite));
        }
        Ok(())
    }

    /// Open the repository for a submodule.
    ///
    /// This will only work if the submodule is checked out into the working
    /// directory.
    pub fn open(&self) -> Result<Repository, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_submodule_open(&mut raw, self.raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Reread submodule info from config, index, and HEAD.
    ///
    /// Call this to reread cached submodule information for this submodule if
    /// you have reason to believe that it has changed.
    ///
    /// If `force` is `true`, then data will be reloaded even if it doesn't seem
    /// out of date
    pub fn reload(&mut self, force: bool) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_submodule_reload(self.raw, force));
        }
        Ok(())
    }

    /// Copy submodule remote info into submodule repo.
    ///
    /// This copies the information about the submodules URL into the checked
    /// out submodule config, acting like "git submodule sync". This is useful
    /// if you have altered the URL for the submodule (or it has been altered
    /// by a fetch of upstream changes) and you need to update your local repo.
    pub fn sync(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_submodule_sync(self.raw)); }
        Ok(())
    }

    /// Add current submodule HEAD commit to index of superproject.
    ///
    /// If `write_index` is true, then the index file will be immediately
    /// written. Otherwise you must explicitly call `write()` on an `Index`
    /// later on.
    pub fn add_to_index(&mut self, write_index: bool) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_submodule_add_to_index(self.raw, write_index));
        }
        Ok(())
    }

    /// Resolve the setup of a new git submodule.
    ///
    /// This should be called on a submodule once you have called add setup and
    /// done the clone of the submodule. This adds the .gitmodules file and the
    /// newly cloned submodule to the index to be ready to be committed (but
    /// doesn't actually do the commit).
    pub fn add_finalize(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_submodule_add_finalize(self.raw)); }
        Ok(())
    }
}

impl<'repo> Binding for Submodule<'repo> {
    type Raw = *mut raw::git_submodule;
    unsafe fn from_raw(raw: *mut raw::git_submodule) -> Submodule<'repo> {
        Submodule { raw: raw, _marker: marker::PhantomData }
    }
    fn raw(&self) -> *mut raw::git_submodule { self.raw }
}

impl<'repo> Drop for Submodule<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_submodule_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::fs;
    use tempdir::TempDir;
    use url::Url;

    use Repository;

    #[test]
    fn smoke() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let mut s1 = repo.submodule("/path/to/nowhere",
                                    Path::new("foo"), true).unwrap();
        s1.init(false).unwrap();
        s1.sync().unwrap();

        let s2 = repo.submodule("/path/to/nowhere",
                                Path::new("bar"), true).unwrap();
        drop((s1, s2));

        let mut submodules = repo.submodules().unwrap();
        assert_eq!(submodules.len(), 2);
        let mut s = submodules.remove(0);
        assert_eq!(s.name(), Some("bar"));
        assert_eq!(s.url(), Some("/path/to/nowhere"));
        assert_eq!(s.branch(), None);
        assert!(s.head_id().is_none());
        assert!(s.index_id().is_none());
        assert!(s.workdir_id().is_none());

        repo.find_submodule("bar").unwrap();
        s.open().unwrap();
        assert!(s.path() == Path::new("bar"));
        s.reload(true).unwrap();
    }

    #[test]
    fn add_a_submodule() {
        let (_td, repo1) = ::test::repo_init();
        let (td, repo2) = ::test::repo_init();

        let url = Url::from_file_path(&repo1.workdir().unwrap()).unwrap();
        let mut s = repo2.submodule(&url.to_string(), Path::new("bar"),
                                    true).unwrap();
        t!(fs::remove_dir_all(td.path().join("bar")));
        t!(Repository::clone(&url.to_string(),
                             td.path().join("bar")));
        t!(s.add_to_index(false));
        t!(s.add_finalize());
    }
}
