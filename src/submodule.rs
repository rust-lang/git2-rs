use std::marker;
use std::mem;
use std::os::raw::c_int;
use std::path::Path;
use std::ptr;
use std::str;

use crate::util::{self, Binding};
use crate::{build::CheckoutBuilder, SubmoduleIgnore, SubmoduleUpdate};
use crate::{raw, Error, FetchOptions, Oid, Repository};

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
        unsafe { crate::opt_bytes(self, raw::git_submodule_branch(self.raw)) }
    }

    /// Perform the clone step for a newly created submodule.
    ///
    /// This performs the necessary `git_clone` to setup a newly-created submodule.
    pub fn clone(
        &mut self,
        opts: Option<&mut SubmoduleUpdateOptions<'_>>,
    ) -> Result<Repository, Error> {
        unsafe {
            let raw_opts = opts.map(|o| o.raw());
            let mut raw_repo = ptr::null_mut();
            try_call!(raw::git_submodule_clone(
                &mut raw_repo,
                self.raw,
                raw_opts.as_ref()
            ));
            Ok(Binding::from_raw(raw_repo))
        }
    }

    /// Get the submodule's URL.
    ///
    /// Returns `None` if the URL is not valid utf-8 or if the URL isn't present
    pub fn url(&self) -> Option<&str> {
        self.opt_url_bytes().and_then(|b| str::from_utf8(b).ok())
    }

    /// Get the URL for the submodule.
    #[doc(hidden)]
    #[deprecated(note = "renamed to `opt_url_bytes`")]
    pub fn url_bytes(&self) -> &[u8] {
        self.opt_url_bytes().unwrap()
    }

    /// Get the URL for the submodule.
    ///
    /// Returns `None` if the URL isn't present
    // TODO: delete this method and fix the signature of `url_bytes` on next
    // major version bump
    pub fn opt_url_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, raw::git_submodule_url(self.raw)) }
    }

    /// Get the submodule's name.
    ///
    /// Returns `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes()).ok()
    }

    /// Get the name for the submodule.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_submodule_name(self.raw)).unwrap() }
    }

    /// Get the path for the submodule.
    pub fn path(&self) -> &Path {
        util::bytes2path(unsafe {
            crate::opt_bytes(self, raw::git_submodule_path(self.raw)).unwrap()
        })
    }

    /// Get the OID for the submodule in the current HEAD tree.
    pub fn head_id(&self) -> Option<Oid> {
        unsafe { Binding::from_raw_opt(raw::git_submodule_head_id(self.raw)) }
    }

    /// Get the OID for the submodule in the index.
    pub fn index_id(&self) -> Option<Oid> {
        unsafe { Binding::from_raw_opt(raw::git_submodule_index_id(self.raw)) }
    }

    /// Get the OID for the submodule in the current working directory.
    ///
    /// This returns the OID that corresponds to looking up 'HEAD' in the
    /// checked out submodule. If there are pending changes in the index or
    /// anything else, this won't notice that.
    pub fn workdir_id(&self) -> Option<Oid> {
        unsafe { Binding::from_raw_opt(raw::git_submodule_wd_id(self.raw)) }
    }

    /// Get the ignore rule that will be used for the submodule.
    pub fn ignore_rule(&self) -> SubmoduleIgnore {
        SubmoduleIgnore::from_raw(unsafe { raw::git_submodule_ignore(self.raw) })
    }

    /// Get the update rule that will be used for the submodule.
    pub fn update_strategy(&self) -> SubmoduleUpdate {
        SubmoduleUpdate::from_raw(unsafe { raw::git_submodule_update_strategy(self.raw) })
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

    /// Set up the subrepository for a submodule in preparation for clone.
    ///
    /// This function can be called to init and set up a submodule repository
    /// from a submodule in preparation to clone it from its remote.

    /// use_gitlink: Should the workdir contain a gitlink to the repo in
    /// .git/modules vs. repo directly in workdir.
    pub fn repo_init(&mut self, use_gitlink: bool) -> Result<Repository, Error> {
        unsafe {
            let mut raw_repo = ptr::null_mut();
            try_call!(raw::git_submodule_repo_init(
                &mut raw_repo,
                self.raw,
                use_gitlink
            ));
            Ok(Binding::from_raw(raw_repo))
        }
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
        unsafe {
            try_call!(raw::git_submodule_sync(self.raw));
        }
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
        unsafe {
            try_call!(raw::git_submodule_add_finalize(self.raw));
        }
        Ok(())
    }

    /// Update submodule.
    ///
    /// This will clone a missing submodule and check out the subrepository to
    /// the commit specified in the index of the containing repository. If
    /// the submodule repository doesn't contain the target commit, then the
    /// submodule is fetched using the fetch options supplied in `opts`.
    ///
    /// `init` indicates if the submodule should be initialized first if it has
    /// not been initialized yet.
    pub fn update(
        &mut self,
        init: bool,
        opts: Option<&mut SubmoduleUpdateOptions<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let mut raw_opts = opts.map(|o| o.raw());
            try_call!(raw::git_submodule_update(
                self.raw,
                init as c_int,
                raw_opts.as_mut().map_or(ptr::null_mut(), |o| o)
            ));
        }
        Ok(())
    }
}

impl<'repo> Binding for Submodule<'repo> {
    type Raw = *mut raw::git_submodule;
    unsafe fn from_raw(raw: *mut raw::git_submodule) -> Submodule<'repo> {
        Submodule {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_submodule {
        self.raw
    }
}

impl<'repo> Drop for Submodule<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_submodule_free(self.raw) }
    }
}

/// Options to update a submodule.
pub struct SubmoduleUpdateOptions<'cb> {
    checkout_builder: CheckoutBuilder<'cb>,
    fetch_opts: FetchOptions<'cb>,
    allow_fetch: bool,
}

impl<'cb> SubmoduleUpdateOptions<'cb> {
    /// Return default options.
    pub fn new() -> Self {
        SubmoduleUpdateOptions {
            checkout_builder: CheckoutBuilder::new(),
            fetch_opts: FetchOptions::new(),
            allow_fetch: true,
        }
    }

    unsafe fn raw(&mut self) -> raw::git_submodule_update_options {
        let mut checkout_opts: raw::git_checkout_options = mem::zeroed();
        let init_res =
            raw::git_checkout_init_options(&mut checkout_opts, raw::GIT_CHECKOUT_OPTIONS_VERSION);
        assert_eq!(0, init_res);
        self.checkout_builder.configure(&mut checkout_opts);
        let opts = raw::git_submodule_update_options {
            version: raw::GIT_SUBMODULE_UPDATE_OPTIONS_VERSION,
            checkout_opts,
            fetch_opts: self.fetch_opts.raw(),
            allow_fetch: self.allow_fetch as c_int,
        };
        opts
    }

    /// Set checkout options.
    pub fn checkout(&mut self, opts: CheckoutBuilder<'cb>) -> &mut Self {
        self.checkout_builder = opts;
        self
    }

    /// Set fetch options and allow fetching.
    pub fn fetch(&mut self, opts: FetchOptions<'cb>) -> &mut Self {
        self.fetch_opts = opts;
        self.allow_fetch = true;
        self
    }

    /// Allow or disallow fetching.
    pub fn allow_fetch(&mut self, b: bool) -> &mut Self {
        self.allow_fetch = b;
        self
    }
}

impl<'cb> Default for SubmoduleUpdateOptions<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;
    use url::Url;

    use crate::Repository;
    use crate::SubmoduleUpdateOptions;

    #[test]
    fn smoke() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let mut s1 = repo
            .submodule("/path/to/nowhere", Path::new("foo"), true)
            .unwrap();
        s1.init(false).unwrap();
        s1.sync().unwrap();

        let s2 = repo
            .submodule("/path/to/nowhere", Path::new("bar"), true)
            .unwrap();
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
        let (_td, repo1) = crate::test::repo_init();
        let (td, repo2) = crate::test::repo_init();

        let url = Url::from_file_path(&repo1.workdir().unwrap()).unwrap();
        let mut s = repo2
            .submodule(&url.to_string(), Path::new("bar"), true)
            .unwrap();
        t!(fs::remove_dir_all(td.path().join("bar")));
        t!(Repository::clone(&url.to_string(), td.path().join("bar")));
        t!(s.add_to_index(false));
        t!(s.add_finalize());
    }

    #[test]
    fn update_submodule() {
        // -----------------------------------
        // Same as `add_a_submodule()`
        let (_td, repo1) = crate::test::repo_init();
        let (td, repo2) = crate::test::repo_init();

        let url = Url::from_file_path(&repo1.workdir().unwrap()).unwrap();
        let mut s = repo2
            .submodule(&url.to_string(), Path::new("bar"), true)
            .unwrap();
        t!(fs::remove_dir_all(td.path().join("bar")));
        t!(Repository::clone(&url.to_string(), td.path().join("bar")));
        t!(s.add_to_index(false));
        t!(s.add_finalize());
        // -----------------------------------

        // Attempt to update submodule
        let submodules = t!(repo1.submodules());
        for mut submodule in submodules {
            let mut submodule_options = SubmoduleUpdateOptions::new();
            let init = true;
            let opts = Some(&mut submodule_options);

            t!(submodule.update(init, opts));
        }
    }

    #[test]
    fn clone_submodule() {
        // -----------------------------------
        // Same as `add_a_submodule()`
        let (_td, repo1) = crate::test::repo_init();
        let (_td, repo2) = crate::test::repo_init();
        let (_td, parent) = crate::test::repo_init();

        let url1 = Url::from_file_path(&repo1.workdir().unwrap()).unwrap();
        let url2 = Url::from_file_path(&repo2.workdir().unwrap()).unwrap();
        let mut s1 = parent
            .submodule(&url1.to_string(), Path::new("bar"), true)
            .unwrap();
        let mut s2 = parent
            .submodule(&url2.to_string(), Path::new("bar2"), true)
            .unwrap();
        // -----------------------------------

        t!(s1.clone(Some(&mut SubmoduleUpdateOptions::default())));
        t!(s2.clone(None));
    }

    #[test]
    fn repo_init_submodule() {
        // -----------------------------------
        // Same as `clone_submodule()`
        let (_td, child) = crate::test::repo_init();
        let (_td, parent) = crate::test::repo_init();

        let url_child = Url::from_file_path(&child.workdir().unwrap()).unwrap();
        let url_parent = Url::from_file_path(&parent.workdir().unwrap()).unwrap();
        let mut sub = parent
            .submodule(&url_child.to_string(), Path::new("bar"), true)
            .unwrap();

        // -----------------------------------
        // Let's commit the submodule for later clone
        t!(sub.clone(None));
        t!(sub.add_to_index(true));
        t!(sub.add_finalize());

        crate::test::commit(&parent);

        // Clone the parent to init its submodules
        let td = TempDir::new().unwrap();
        let new_parent = Repository::clone(&url_parent.to_string(), &td).unwrap();

        let mut submodules = new_parent.submodules().unwrap();
        let child = submodules.first_mut().unwrap();

        // First init child
        t!(child.init(false));
        assert_eq!(child.url().unwrap(), url_child.as_str());

        // open() is not possible before initializing the repo
        assert!(child.open().is_err());
        t!(child.repo_init(true));
        assert!(child.open().is_ok());
    }
}
