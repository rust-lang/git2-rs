use std::str;
use std::kinds::marker;

use {raw, Oid, Repository, Error};

/// A structure to represent a git [submodule][1]
///
/// [1]: http://git-scm.com/book/en/Git-Tools-Submodules
pub struct Submodule<'a> {
    raw: *mut raw::git_submodule,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'a> Submodule<'a> {
    /// Create a new object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_submodule) -> Submodule {
        Submodule {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Get the submodule's branch.
    ///
    /// Returns `None` if the branch is not valid utf-8 or if the branch is not
    /// yet available.
    pub fn branch(&self) -> Option<&str> {
        self.branch_bytes().and_then(str::from_utf8)
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
    pub fn url(&self) -> Option<&str> { str::from_utf8(self.url_bytes()) }

    /// Get the url for the submodule.
    pub fn url_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_submodule_url(self.raw)).unwrap()
        }
    }

    /// Get the submodule's name.
    ///
    /// Returns `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> { str::from_utf8(self.name_bytes()) }

    /// Get the name for the submodule.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_submodule_name(self.raw)).unwrap()
        }
    }

    /// Get the path for the submodule.
    pub fn path(&self) -> Path {
        let bytes = unsafe {
            ::opt_bytes(self, raw::git_submodule_path(self.raw)).unwrap()
        };
        Path::new(bytes)
    }

    /// Get the OID for the submodule in the current HEAD tree.
    pub fn head_id(&self) -> Option<Oid> {
        unsafe {
            let ptr = raw::git_submodule_head_id(self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(Oid::from_raw(ptr))
            }
        }
    }

    /// Get the OID for the submodule in the index.
    pub fn index_id(&self) -> Option<Oid> {
        unsafe {
            let ptr = raw::git_submodule_index_id(self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(Oid::from_raw(ptr))
            }
        }
    }

    /// Get the OID for the submodule in the current working directory.
    ///
    /// This returns the OID that corresponds to looking up 'HEAD' in the
    /// checked out submodule. If there are pending changes in the index or
    /// anything else, this won't notice that.
    pub fn workdir_id(&self) -> Option<Oid> {
        unsafe {
            let ptr = raw::git_submodule_wd_id(self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(Oid::from_raw(ptr))
            }
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
        let mut raw = 0 as *mut raw::git_repository;
        unsafe {
            try_call!(raw::git_submodule_open(&mut raw, self.raw));
        }
        Ok(unsafe { Repository::from_raw(raw) })
    }

    /// Access the underlying raw git submodule pointer.
    pub fn raw(&self) -> *mut raw::git_submodule { self.raw }

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

    /// Write submodule settings to .gitmodules file.
    ///
    /// This commits any in-memory changes to the submodule to the gitmodules
    /// file on disk. You may also be interested in `init()` which
    /// writes submodule info to ".git/config" (which is better for local
    /// changes to submodule settings) and/or `sync()` which writes
    /// settings about remotes to the actual submodule repository.
    pub fn save(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_submodule_save(self.raw)); }
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
}

#[unsafe_destructor]
impl<'a> Drop for Submodule<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_submodule_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::TempDir;
    use Repository;

    #[test]
    fn smoke() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let mut s1 = repo.submodule("/path/to/nowhere",
                                    &Path::new("foo"), true).unwrap();
        s1.init(false).unwrap();
        let s2 = repo.submodule("/path/to/nowhere",
                                &Path::new("bar"), true).unwrap();
        drop((s1, s2));

        let mut submodules = repo.submodules().unwrap();
        assert_eq!(submodules.len(), 2);
        let mut s = submodules.remove(0).unwrap();
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
}
