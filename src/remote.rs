use std::ffi::CString;
use std::ops::Range;
use std::marker;
use std::mem;
use std::slice;
use std::str;
use libc;

use {raw, Direction, Error, Refspec, Oid, IntoCString};
use {Push, RemoteCallbacks, Progress, Repository};
use util::Binding;

/// A structure representing a [remote][1] of a git repository.
///
/// [1]: http://git-scm.com/book/en/Git-Basics-Working-with-Remotes
///
/// The lifetime is the lifetime of the repository that it is attached to. The
/// remote is used to manage fetches and pushes as well as refspecs.
pub struct Remote<'repo, 'cb> {
    raw: *mut raw::git_remote,
    _marker: marker::PhantomData<&'repo Repository>,
    callbacks: Option<Box<RemoteCallbacks<'cb>>>,
}

/// An iterator over the refspecs that a remote contains.
pub struct Refspecs<'remote, 'cb: 'remote> {
    range: Range<usize>,
    remote: &'remote Remote<'remote, 'cb>,
}

/// Description of a reference advertised bya remote server, given out on calls
/// to `list`.
pub struct RemoteHead<'remote> {
    raw: *const raw::git_remote_head,
    _marker: marker::PhantomData<&'remote str>,
}

impl<'repo, 'cb> Remote<'repo, 'cb> {
    /// Ensure the remote name is well-formed.
    pub fn is_valid_name(remote_name: &str) -> bool {
        ::init();
        let remote_name = CString::new(remote_name).unwrap();
        unsafe { raw::git_remote_is_valid_name(remote_name.as_ptr()) == 1 }
    }

    /// Get the remote's name.
    ///
    /// Returns `None` if this remote has not yet been named or if the name is
    /// not valid utf-8
    pub fn name(&self) -> Option<&str> {
        self.name_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the remote's name, in bytes.
    ///
    /// Returns `None` if this remote has not yet been named
    pub fn name_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_remote_name(&*self.raw)) }
    }

    /// Get the remote's url.
    ///
    /// Returns `None` if the url is not valid utf-8
    pub fn url(&self) -> Option<&str> {
        str::from_utf8(self.url_bytes()).ok()
    }

    /// Get the remote's url as a byte array.
    pub fn url_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_remote_url(&*self.raw)).unwrap() }
    }

    /// Get the remote's pushurl.
    ///
    /// Returns `None` if the pushurl is not valid utf-8
    pub fn pushurl(&self) -> Option<&str> {
        self.pushurl_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the remote's pushurl as a byte array.
    pub fn pushurl_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_remote_pushurl(&*self.raw)) }
    }

    /// Open a connection to a remote.
    pub fn connect(&mut self, dir: Direction) -> Result<(), Error> {
        unsafe {
            try!(self.set_raw_callbacks());
            try_call!(raw::git_remote_connect(self.raw, dir));
        }
        Ok(())
    }

    /// Check whether the remote is connected
    pub fn connected(&mut self) -> bool {
        unsafe { raw::git_remote_connected(self.raw) == 1 }
    }

    /// Disconnect from the remote
    pub fn disconnect(&mut self) {
        unsafe { raw::git_remote_disconnect(self.raw) }
    }

    /// Save a remote to its repository's configuration
    ///
    /// Anonymous remotes cannot be saved
    pub fn save(&self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_remote_save(&*self.raw)); }
        Ok(())
    }

    /// Add a fetch refspec to the remote
    pub fn add_fetch(&mut self, spec: &str) -> Result<(), Error> {
        let spec = try!(CString::new(spec));
        unsafe {
            try_call!(raw::git_remote_add_fetch(self.raw, spec));
        }
        Ok(())
    }

    /// Add a push refspec to the remote
    pub fn add_push(&mut self, spec: &str) -> Result<(), Error> {
        let spec = try!(CString::new(spec));
        unsafe {
            try_call!(raw::git_remote_add_push(self.raw, spec));
        }
        Ok(())
    }

    /// Set the remote's url
    ///
    /// Existing connections will not be updated.
    pub fn set_url(&mut self, url: &str) -> Result<(), Error> {
        let url = try!(CString::new(url));
        unsafe { try_call!(raw::git_remote_set_url(self.raw, url)); }
        Ok(())
    }

    /// Set the remote's pushurl.
    ///
    /// `None` indicates that it should be cleared.
    ///
    /// Existing connections will not be updated.
    pub fn set_pushurl(&mut self, pushurl: Option<&str>) -> Result<(), Error> {
        let pushurl = try!(::opt_cstr(pushurl));
        unsafe {
            try_call!(raw::git_remote_set_pushurl(self.raw, pushurl));
        }
        Ok(())
    }

    /// Sets the update FETCH_HEAD setting. By default, FETCH_HEAD will be
    /// updated on every fetch.
    pub fn set_update_fetchhead(&mut self, update: bool) {
        unsafe {
            raw::git_remote_set_update_fetchhead(self.raw, update as libc::c_int)
        }
    }

    /// Set the remote's list of fetch refspecs
    pub fn set_fetch_refspecs<T, I>(&mut self, i: I) -> Result<(), Error>
        where T: IntoCString, I: Iterator<Item=T>
    {
        let (_a, _b, mut arr) = try!(::util::iter2cstrs(i));
        unsafe {
            try_call!(raw::git_remote_set_fetch_refspecs(self.raw, &mut arr));
        }
        Ok(())
    }

    /// Set the remote's list of push refspecs
    pub fn set_push_refspecs<T, I>(&mut self, i: I) -> Result<(), Error>
        where T: IntoCString, I: Iterator<Item=T>
    {
        let (_a, _b, mut arr) = try!(::util::iter2cstrs(i));
        unsafe {
            try_call!(raw::git_remote_set_push_refspecs(self.raw, &mut arr));
        }
        Ok(())
    }

    /// Clear the refspecs
    ///
    /// Remove all configured fetch and push refspecs from the remote.
    pub fn clear_refspecs(&mut self) {
        unsafe { raw::git_remote_clear_refspecs(self.raw) }
    }

    /// Download and index the packfile
    ///
    /// Connect to the remote if it hasn't been done yet, negotiate with the
    /// remote git which objects are missing, download and index the packfile.
    ///
    /// The .idx file will be created and both it and the packfile with be
    /// renamed to their final name.
    ///
    /// The `specs` argument is a list of refspecs to use for this negotiation
    /// and download. Use an empty array to use the base refspecs.
    pub fn download(&mut self, specs: &[&str]) -> Result<(), Error> {
        let (_a, _b, arr) = try!(::util::iter2cstrs(specs.iter()));
        unsafe {
            try!(self.set_raw_callbacks());
            try_call!(raw::git_remote_download(self.raw, &arr));
        }
        Ok(())
    }

    /// Get the number of refspecs for a remote
    pub fn refspecs<'a>(&'a self) -> Refspecs<'a, 'cb> {
        let cnt = unsafe { raw::git_remote_refspec_count(&*self.raw) as usize };
        Refspecs { range: 0..cnt, remote: self }
    }

    /// Get the `nth` refspec from this remote.
    ///
    /// The `refspecs` iterator can be used to iterate over all refspecs.
    pub fn get_refspec(&self, i: usize) -> Option<Refspec<'repo>> {
        unsafe {
            let ptr = raw::git_remote_get_refspec(&*self.raw,
                                                  i as libc::size_t);
            Binding::from_raw_opt(ptr)
        }
    }

    /// Download new data and update tips
    ///
    /// Convenience function to connect to a remote, download the data,
    /// disconnect and update the remote-tracking branches.
    pub fn fetch(&mut self,
                 refspecs: &[&str],
                 msg: Option<&str>) -> Result<(), Error> {
        let (_a, _b, arr) = try!(::util::iter2cstrs(refspecs.iter()));
        let msg = try!(::opt_cstr(msg));
        unsafe {
            try!(self.set_raw_callbacks());
            try_call!(raw::git_remote_fetch(self.raw, &arr, msg));
        }
        Ok(())
    }

    /// Update the tips to the new state
    pub fn update_tips(&mut self, msg: Option<&str>) -> Result<(), Error> {
        let msg = try!(::opt_cstr(msg));
        unsafe {
            try_call!(raw::git_remote_update_tips(self.raw, msg));
        }
        Ok(())
    }

    /// Retrieve the update FETCH_HEAD setting.
    pub fn update_fetchhead(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_remote_update_fetchhead(self.raw)); }
        Ok(())
    }

    /// Create a new push object
    pub fn push(&mut self) -> Result<Push, Error> {
        let mut ret = 0 as *mut raw::git_push;
        try!(self.set_raw_callbacks());
        unsafe {
            try_call!(raw::git_push_new(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Set the callbacks to be invoked when the transfer is in-progress.
    ///
    /// This will overwrite the previously set callbacks.
    pub fn set_callbacks(&mut self, callbacks: RemoteCallbacks<'cb>) {
        self.callbacks = Some(Box::new(callbacks));
    }

    fn set_raw_callbacks(&mut self) -> Result<(), Error> {
        match self.callbacks {
            Some(ref mut cbs) => unsafe {
                let raw = cbs.raw();
                try_call!(raw::git_remote_set_callbacks(self.raw, &raw));
            },
            None => {}
        }
        Ok(())
    }

    /// Get the statistics structure that is filled in by the fetch operation.
    pub fn stats(&self) -> Progress {
        unsafe {
            Binding::from_raw(raw::git_remote_stats(self.raw))
        }
    }

    /// Get the remote repository's reference advertisement list.
    ///
    /// Get the list of references with which the server responds to a new
    /// connection.
    ///
    /// The remote (or more exactly its transport) must have connected to the
    /// remote repository. This list is available as soon as the connection to
    /// the remote is initiated and it remains available after disconnecting.
    pub fn list(&self) -> Result<&[RemoteHead], Error> {
        let mut size = 0;
        let mut base = 0 as *mut _;
        unsafe {
            try_call!(raw::git_remote_ls(&mut base, &mut size, self.raw));
            assert_eq!(mem::size_of::<RemoteHead>(),
                       mem::size_of::<*const raw::git_remote_head>());
            let slice = slice::from_raw_parts(base as *const _, size as usize);
            Ok(mem::transmute::<&[*const raw::git_remote_head],
                                &[RemoteHead]>(slice))
        }
    }
}

impl<'a, 'b> Clone for Remote<'a, 'b> {
    fn clone(&self) -> Remote<'a, 'b> {
        let mut ret = 0 as *mut raw::git_remote;
        let rc = unsafe { call!(raw::git_remote_dup(&mut ret, self.raw)) };
        assert_eq!(rc, 0);
        Remote {
            raw: ret,
            _marker: marker::PhantomData,
            callbacks: None,
        }
    }
}

impl<'repo, 'cb> Binding for Remote<'repo, 'cb> {
    type Raw = *mut raw::git_remote;

    unsafe fn from_raw(raw: *mut raw::git_remote) -> Remote<'repo, 'cb> {
        Remote {
            raw: raw,
            _marker: marker::PhantomData,
            callbacks: None,
        }
    }
    fn raw(&self) -> *mut raw::git_remote { self.raw }
}

impl<'a, 'b> Drop for Remote<'a, 'b> {
    fn drop(&mut self) {
        unsafe { raw::git_remote_free(self.raw) }
    }
}

impl<'repo, 'cb> Iterator for Refspecs<'repo, 'cb> {
    type Item = Refspec<'repo>;
    fn next(&mut self) -> Option<Refspec<'repo>> {
        self.range.next().and_then(|i| self.remote.get_refspec(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}
impl<'repo, 'cb> DoubleEndedIterator for Refspecs<'repo, 'cb> {
    fn next_back(&mut self) -> Option<Refspec<'repo>> {
        self.range.next_back().and_then(|i| self.remote.get_refspec(i))
    }
}
impl<'repo, 'cb> ExactSizeIterator for Refspecs<'repo, 'cb> {}

#[allow(missing_docs)] // not documented in libgit2 :(
impl<'remote> RemoteHead<'remote> {
    /// Flag if this is available locally.
    pub fn is_local(&self) -> bool {
        unsafe { (*self.raw).local != 0 }
    }

    pub fn oid(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).oid as *const _) }
    }
    pub fn loid(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).loid as *const _) }
    }

    pub fn name(&self) -> &str {
        let b = unsafe { ::opt_bytes(self, (*self.raw).name).unwrap() };
        str::from_utf8(b).unwrap()
    }

    pub fn symref_target(&self) -> Option<&str> {
        let b = unsafe { ::opt_bytes(self, (*self.raw).symref_target) };
        b.map(|b| str::from_utf8(b).unwrap())
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use tempdir::TempDir;
    use {Repository, Remote, RemoteCallbacks, Direction};

    #[test]
    fn smoke() {
        let (td, repo) = ::test::repo_init();
        t!(repo.remote("origin", "/path/to/nowhere"));
        drop(repo);

        let repo = t!(Repository::init(td.path()));
        let mut origin = t!(repo.find_remote("origin"));
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.url(), Some("/path/to/nowhere"));
        assert_eq!(origin.pushurl(), None);

        t!(origin.set_url("/path/to/elsewhere"));
        assert_eq!(origin.url(), Some("/path/to/elsewhere"));
        t!(origin.set_pushurl(Some("/path/to/elsewhere")));
        assert_eq!(origin.pushurl(), Some("/path/to/elsewhere"));

        origin.set_update_fetchhead(true);
        let stats = origin.stats();
        assert_eq!(stats.total_objects(), 0);
    }

    #[test]
    fn create_remote() {
        let td = TempDir::new("test").unwrap();
        let remote = td.path().join("remote");
        Repository::init_bare(&remote).unwrap();

        let (_td, repo) = ::test::repo_init();
        let url = if cfg!(unix) {
            format!("file://{}", remote.display())
        } else {
            format!("file:///{}", remote.display().to_string()
                                        .replace("\\", "/"))
        };
        let mut origin = repo.remote("origin", &url).unwrap();
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.url(), Some(&url[..]));
        assert_eq!(origin.pushurl(), None);

        {
            let mut specs = origin.refspecs();
            let spec = specs.next().unwrap();
            assert!(specs.next().is_none());
            assert_eq!(spec.str(), Some("+refs/heads/*:refs/remotes/origin/*"));
            assert_eq!(spec.dst(), Some("refs/remotes/origin/*"));
            assert_eq!(spec.src(), Some("refs/heads/*"));
            assert!(spec.is_force());
        }
        assert!(origin.refspecs().next_back().is_some());
        {
            let remotes = repo.remotes().unwrap();
            assert_eq!(remotes.len(), 1);
            assert_eq!(remotes.get(0), Some("origin"));
            assert_eq!(remotes.iter().count(), 1);
            assert_eq!(remotes.iter().next().unwrap(), Some("origin"));
        }

        origin.connect(Direction::Push).unwrap();
        assert!(origin.connected());
        origin.disconnect();

        origin.connect(Direction::Fetch).unwrap();
        assert!(origin.connected());
        origin.download(&[]).unwrap();
        origin.disconnect();

        t!(origin.save());

        t!(origin.add_fetch("foo"));
        t!(origin.add_fetch("bar"));
        origin.clear_refspecs();
        t!(origin.update_fetchhead());

        origin.set_fetch_refspecs(["foo"].iter().map(|a| *a)).unwrap();
        origin.set_push_refspecs(["foo"].iter().map(|a| *a)).unwrap();

        origin.fetch(&[], None).unwrap();
        origin.fetch(&[], Some("foo")).unwrap();
        origin.update_tips(None).unwrap();
        origin.update_tips(Some("foo")).unwrap();
    }

    #[test]
    fn rename_remote() {
        let (_td, repo) = ::test::repo_init();
        repo.remote("origin", "foo").unwrap();
        repo.remote_rename("origin", "foo").unwrap();
        repo.remote_delete("foo").unwrap();
    }

    #[test]
    fn create_remote_anonymous() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();

        let origin = repo.remote_anonymous("/path/to/nowhere",
                                           Some("master")).unwrap();
        assert_eq!(origin.name(), None);
        drop(origin.clone());
    }

    #[test]
    fn is_valid() {
        assert!(Remote::is_valid_name("foobar"));
        assert!(!Remote::is_valid_name("\x01"));
    }

    #[test]
    fn transfer_cb() {
        let (td, _repo) = ::test::repo_init();
        let td2 = TempDir::new("git").unwrap();
        let url = ::test::path2url(&td.path());

        let repo = Repository::init(td2.path()).unwrap();
        let progress_hit = Cell::new(false);
        {
            let mut callbacks = RemoteCallbacks::new();
            let mut origin = repo.remote("origin", &url).unwrap();

            callbacks.transfer_progress(|_progress| {
                progress_hit.set(true);
                true
            });
            origin.set_callbacks(callbacks);
            origin.fetch(&[], None).unwrap();

            let list = t!(origin.list());
            assert_eq!(list.len(), 2);
            assert_eq!(list[0].name(), "HEAD");
            assert!(!list[0].is_local());
            assert_eq!(list[1].name(), "refs/heads/master");
            assert!(!list[1].is_local());
        }
        assert!(progress_hit.get());
    }
}
