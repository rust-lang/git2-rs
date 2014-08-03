use std::c_str::CString;
use std::mem;
use libc;

use {raw, Signature, Error, Repository};

pub struct RepoBuilder {
    bare: bool,
    branch: Option<CString>,
    sig: Option<Signature>,
    local: bool,
    hardlinks: bool,
}

impl RepoBuilder {
    pub fn new() -> RepoBuilder {
        RepoBuilder {
            bare: false,
            branch: None,
            sig: None,
            local: true,
            hardlinks: true,
        }
    }

    /// Indicate whether the repository will be cloned as a bare repository or
    /// not.
    pub fn bare(&mut self, bare: bool) -> &mut RepoBuilder {
        self.bare = bare;
        self
    }

    /// Specify the name of the branch to check out after the clone.
    ///
    /// If not specified, the remote's default branch will be used.
    pub fn branch(&mut self, branch: &str) -> &mut RepoBuilder {
        self.branch = Some(branch.to_c_str());
        self
    }

    /// Specify the identity that will be used when updating the reflog.
    ///
    /// If not specified, the default signature will be used.
    pub fn signature(&mut self, sig: Signature) -> &mut RepoBuilder {
        self.sig = Some(sig);
        self
    }

    /// Set the flag for bypassing the git aware transport mechanism for local
    /// paths.
    ///
    /// If `true`, the git-aware transport will be bypassed for local paths. If
    /// `false`, the git-aware transport will not be bypassed.
    pub fn local(&mut self, local: bool) -> &mut RepoBuilder {
        self.local = local;
        self
    }

    /// Set the flag for whether hardlinks are used when using a local git-aware
    /// transport mechanism.
    pub fn hardlinks(&mut self, links: bool) -> &mut RepoBuilder {
        self.hardlinks = links;
        self
    }

    /// Clone a remote repository.
    ///
    /// This will use the options configured so far to clone the specified url
    /// into the specified local path.
    pub fn clone(&self, url: &str, into: &Path) -> Result<Repository, Error> {
        let mut opts: raw::git_clone_options = unsafe { mem::zeroed() };
        try!(::doit(|| unsafe {
            raw::git_clone_init_options(&mut opts, raw::GIT_CLONE_OPTIONS_VERSION)
        }));
        opts.bare = self.bare as libc::c_int;
        opts.checkout_branch = self.branch.as_ref().map(|s| {
            s.as_ptr()
        }).unwrap_or(0 as *const _);
        opts.signature = self.sig.as_ref().map(|s| {
            s.raw()
        }).unwrap_or(0 as *const _) as *mut _;

        opts.local = match (self.local, self.hardlinks) {
            (true, false) => raw::GIT_CLONE_LOCAL_NO_LINKS,
            (false, _) => raw::GIT_CLONE_NO_LOCAL,
            (true, _) => raw::GIT_CLONE_LOCAL_AUTO,
        };

        let url = url.to_c_str();
        let into = into.to_c_str();
        let mut raw = 0 as *mut raw::git_repository;
        try!(::doit(|| unsafe {
            raw::git_clone(&mut raw, url.as_ptr(), into.as_ptr(), &opts)
        }));
        Ok(unsafe { Repository::from_raw(raw) })
    }
}

#[cfg(test)]
mod tests {
    use std::io::{fs, TempDir};
    use super::RepoBuilder;
    use Repository;

    #[test]
    fn smoke() {
        let r = RepoBuilder::new().clone("/path/to/nowhere", &Path::new("foo"));
        assert!(r.is_err());
    }

    #[test]
    fn smoke2() {
        let td = TempDir::new("test").unwrap();
        Repository::init(&td.path().join("bare"), true).unwrap();
        let url = format!("file://{}/bare", td.path().display());
        let dst = td.path().join("foo");
        RepoBuilder::new().clone(url.as_slice(), &dst).unwrap();
        fs::rmdir_recursive(&dst).unwrap();
        RepoBuilder::new().local(false).clone(url.as_slice(), &dst).unwrap();
        fs::rmdir_recursive(&dst).unwrap();
        RepoBuilder::new().local(false).hardlinks(false).bare(true)
                          .clone(url.as_slice(), &dst).unwrap();
        fs::rmdir_recursive(&dst).unwrap();
        assert!(RepoBuilder::new().branch("foo")
                                  .clone(url.as_slice(), &dst).is_err());
    }

}
