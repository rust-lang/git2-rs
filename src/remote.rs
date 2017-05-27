use std::ffi::CString;
use std::ops::Range;
use std::marker;
use std::mem;
use std::ptr;
use std::slice;
use std::str;
use libc;

use {raw, Direction, Error, Refspec, Oid, FetchPrune, ProxyOptions};
use {RemoteCallbacks, Progress, Repository, AutotagOption};
use string_array::StringArray;
use util::Binding;

/// A structure representing a [remote][1] of a git repository.
///
/// [1]: http://git-scm.com/book/en/Git-Basics-Working-with-Remotes
///
/// The lifetime is the lifetime of the repository that it is attached to. The
/// remote is used to manage fetches and pushes as well as refspecs.
pub struct Remote<'repo> {
    raw: *mut raw::git_remote,
    _marker: marker::PhantomData<&'repo Repository>,
}

/// An iterator over the refspecs that a remote contains.
pub struct Refspecs<'remote> {
    range: Range<usize>,
    remote: &'remote Remote<'remote>,
}

/// Description of a reference advertised bya remote server, given out on calls
/// to `list`.
pub struct RemoteHead<'remote> {
    raw: *const raw::git_remote_head,
    _marker: marker::PhantomData<&'remote str>,
}

/// Options which can be specified to various fetch operations.
pub struct FetchOptions<'cb> {
    callbacks: Option<RemoteCallbacks<'cb>>,
    proxy: Option<ProxyOptions<'cb>>,
    prune: FetchPrune,
    update_fetchhead: bool,
    download_tags: AutotagOption,
}

/// Options to control the behavior of a git push.
pub struct PushOptions<'cb> {
    callbacks: Option<RemoteCallbacks<'cb>>,
    proxy: Option<ProxyOptions<'cb>>,
    pb_parallelism: u32,
}

/// Holds callbacks for a connection to a `Remote`. Disconnects when dropped
pub struct RemoteConnection<'repo, 'connection, 'cb> where 'repo: 'connection {
    _callbacks: Box<RemoteCallbacks<'cb>>,
    _proxy: ProxyOptions<'cb>,
    remote: &'connection mut Remote<'repo>,
}

impl<'repo> Remote<'repo> {
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
        // TODO: can callbacks be exposed safely?
        unsafe {
            try_call!(raw::git_remote_connect(self.raw, dir,
                                              ptr::null(),
                                              ptr::null(),
                                              ptr::null()));
        }
        Ok(())
    }

    /// Open a connection to a remote with callbacks and proxy settings
    ///
    /// Returns a `RemoteConnection` that will disconnect once dropped
    pub fn connect_auth<'connection, 'cb>(&'connection mut self,
                                          dir: Direction,
                                          cb: Option<RemoteCallbacks<'cb>>,
                                          proxy_options: Option<ProxyOptions<'cb>>)
                    -> Result<RemoteConnection<'repo, 'connection, 'cb>, Error> {

        let cb = Box::new(cb.unwrap_or_else(|| RemoteCallbacks::new()));
        let proxy_options = proxy_options.unwrap_or_else(|| ProxyOptions::new());
        unsafe {
            try_call!(raw::git_remote_connect(self.raw, dir,
                                              &cb.raw(),
                                              &proxy_options.raw(),
                                              ptr::null()));
        }

        Ok(RemoteConnection {
            _callbacks: cb,
            _proxy: proxy_options,
            remote: self,
        })
    }

    /// Check whether the remote is connected
    pub fn connected(&mut self) -> bool {
        unsafe { raw::git_remote_connected(self.raw) == 1 }
    }

    /// Disconnect from the remote
    pub fn disconnect(&mut self) {
        unsafe { raw::git_remote_disconnect(self.raw) }
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
    pub fn download(&mut self, specs: &[&str], opts: Option<&mut FetchOptions>)
                    -> Result<(), Error> {
        let (_a, _b, arr) = try!(::util::iter2cstrs(specs.iter()));
        let raw = opts.map(|o| o.raw());
        unsafe {
            try_call!(raw::git_remote_download(self.raw, &arr, raw.as_ref()));
        }
        Ok(())
    }

    /// Get the number of refspecs for a remote
    pub fn refspecs<'a>(&'a self) -> Refspecs<'a> {
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
    ///
    /// # Examples
    ///
    /// Example of functionality similar to `git fetch origin/master`:
    ///
    /// ```no_run
    /// fn fetch_origin_master(repo: git2::Repository) -> Result<(), git2::Error> {
    ///     repo.find_remote("origin")?.fetch(&["master"], None, None)
    /// }
    ///
    /// let repo = git2::Repository::discover("rust").unwrap();
    /// fetch_origin_master(repo).unwrap();
    /// ```
    pub fn fetch(&mut self,
                 refspecs: &[&str],
                 opts: Option<&mut FetchOptions>,
                 reflog_msg: Option<&str>) -> Result<(), Error> {
        let (_a, _b, arr) = try!(::util::iter2cstrs(refspecs.iter()));
        let msg = try!(::opt_cstr(reflog_msg));
        let raw = opts.map(|o| o.raw());
        unsafe {
            try_call!(raw::git_remote_fetch(self.raw, &arr, raw.as_ref(), msg));
        }
        Ok(())
    }

    /// Update the tips to the new state
    pub fn update_tips(&mut self,
                       callbacks: Option<&mut RemoteCallbacks>,
                       update_fetchhead: bool,
                       download_tags: AutotagOption,
                       msg: Option<&str>) -> Result<(), Error> {
        let msg = try!(::opt_cstr(msg));
        let cbs = callbacks.map(|cb| cb.raw());
        unsafe {
            try_call!(raw::git_remote_update_tips(self.raw, cbs.as_ref(),
                                                  update_fetchhead,
                                                  download_tags, msg));
        }
        Ok(())
    }

    /// Perform a push
    ///
    /// Perform all the steps for a push. If no refspecs are passed then the
    /// configured refspecs will be used.
    ///
    /// Note that you'll likely want to use `RemoteCallbacks` and set
    /// `push_update_reference` to test whether all the references were pushed
    /// successfully.
    pub fn push(&mut self,
                refspecs: &[&str],
                opts: Option<&mut PushOptions>) -> Result<(), Error> {
        let (_a, _b, arr) = try!(::util::iter2cstrs(refspecs.iter()));
        let raw = opts.map(|o| o.raw());
        unsafe {
            try_call!(raw::git_remote_push(self.raw, &arr, raw.as_ref()));
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
        let mut base = ptr::null_mut();
        unsafe {
            try_call!(raw::git_remote_ls(&mut base, &mut size, self.raw));
            assert_eq!(mem::size_of::<RemoteHead>(),
                       mem::size_of::<*const raw::git_remote_head>());
            let slice = slice::from_raw_parts(base as *const _, size as usize);
            Ok(mem::transmute::<&[*const raw::git_remote_head],
                                &[RemoteHead]>(slice))
        }
    }
    
    /// Get the remote's list of fetch refspecs
    pub fn fetch_refspecs(&self) -> Result<StringArray, Error> {
        unsafe {
            let mut raw: raw::git_strarray = mem::zeroed();
            try_call!(raw::git_remote_get_fetch_refspecs(&mut raw, self.raw));
            Ok(StringArray::from_raw(raw))
        }
    }

    /// Get the remote's list of push refspecs
    pub fn push_refspecs(&self) -> Result<StringArray, Error> {
        unsafe {
            let mut raw: raw::git_strarray = mem::zeroed();
            try_call!(raw::git_remote_get_push_refspecs(&mut raw, self.raw));
            Ok(StringArray::from_raw(raw))
        }
    }
}

impl<'repo> Clone for Remote<'repo> {
    fn clone(&self) -> Remote<'repo> {
        let mut ret = ptr::null_mut();
        let rc = unsafe { call!(raw::git_remote_dup(&mut ret, self.raw)) };
        assert_eq!(rc, 0);
        Remote {
            raw: ret,
            _marker: marker::PhantomData,
        }
    }
}

impl<'repo> Binding for Remote<'repo> {
    type Raw = *mut raw::git_remote;

    unsafe fn from_raw(raw: *mut raw::git_remote) -> Remote<'repo> {
        Remote {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_remote { self.raw }
}

impl<'repo> Drop for Remote<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_remote_free(self.raw) }
    }
}

impl<'repo> Iterator for Refspecs<'repo> {
    type Item = Refspec<'repo>;
    fn next(&mut self) -> Option<Refspec<'repo>> {
        self.range.next().and_then(|i| self.remote.get_refspec(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}
impl<'repo> DoubleEndedIterator for Refspecs<'repo> {
    fn next_back(&mut self) -> Option<Refspec<'repo>> {
        self.range.next_back().and_then(|i| self.remote.get_refspec(i))
    }
}
impl<'repo> ExactSizeIterator for Refspecs<'repo> {}

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

impl<'cb> Default for FetchOptions<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'cb> FetchOptions<'cb> {
    /// Creates a new blank set of fetch options
    pub fn new() -> FetchOptions<'cb> {
        FetchOptions {
            callbacks: None,
            proxy: None,
            prune: FetchPrune::Unspecified,
            update_fetchhead: true,
            download_tags: AutotagOption::Unspecified,
        }
    }

    /// Set the callbacks to use for the fetch operation.
    pub fn remote_callbacks(&mut self, cbs: RemoteCallbacks<'cb>) -> &mut Self {
        self.callbacks = Some(cbs);
        self
    }

    /// Set the proxy options to use for the fetch operation.
    pub fn proxy_options(&mut self, opts: ProxyOptions<'cb>) -> &mut Self {
        self.proxy = Some(opts);
        self
    }

    /// Set whether to perform a prune after the fetch.
    pub fn prune(&mut self, prune: FetchPrune) -> &mut Self {
        self.prune = prune;
        self
    }

    /// Set whether to write the results to FETCH_HEAD.
    ///
    /// Defaults to `true`.
    pub fn update_fetchhead(&mut self, update: bool) -> &mut Self {
        self.update_fetchhead = update;
        self
    }

    /// Set how to behave regarding tags on the remote, such as auto-downloading
    /// tags for objects we're downloading or downloading all of them.
    ///
    /// The default is to auto-follow tags.
    pub fn download_tags(&mut self, opt: AutotagOption) -> &mut Self {
        self.download_tags = opt;
        self
    }
}

impl<'cb> Binding for FetchOptions<'cb> {
    type Raw = raw::git_fetch_options;

    unsafe fn from_raw(_raw: raw::git_fetch_options) -> FetchOptions<'cb> {
        panic!("unimplemented");
    }
    fn raw(&self) -> raw::git_fetch_options {
        raw::git_fetch_options {
            version: 1,
            callbacks: self.callbacks.as_ref().map(|m| m.raw())
                           .unwrap_or_else(|| RemoteCallbacks::new().raw()),
            proxy_opts: self.proxy.as_ref().map(|m| m.raw())
                            .unwrap_or_else(|| ProxyOptions::new().raw()),
            prune: ::call::convert(&self.prune),
            update_fetchhead: ::call::convert(&self.update_fetchhead),
            download_tags: ::call::convert(&self.download_tags),
            // TODO: expose this as a builder option
            custom_headers: raw::git_strarray {
                count: 0,
                strings: ptr::null_mut(),
            },
        }
    }
}


impl<'cb> Default for PushOptions<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'cb> PushOptions<'cb> {
    /// Creates a new blank set of push options
    pub fn new() -> PushOptions<'cb> {
        PushOptions {
            callbacks: None,
            proxy: None,
            pb_parallelism: 1,
        }
    }

    /// Set the callbacks to use for the fetch operation.
    pub fn remote_callbacks(&mut self, cbs: RemoteCallbacks<'cb>) -> &mut Self {
        self.callbacks = Some(cbs);
        self
    }

    /// Set the proxy options to use for the fetch operation.
    pub fn proxy_options(&mut self, opts: ProxyOptions<'cb>) -> &mut Self {
        self.proxy = Some(opts);
        self
    }

    /// If the transport being used to push to the remote requires the creation
    /// of a pack file, this controls the number of worker threads used by the
    /// packbuilder when creating that pack file to be sent to the remote.
    ///
    /// if set to 0 the packbuilder will auto-detect the number of threads to
    /// create, and the default value is 1.
    pub fn packbuilder_parallelism(&mut self, parallel: u32) -> &mut Self {
        self.pb_parallelism = parallel;
        self
    }
}

impl<'cb> Binding for PushOptions<'cb> {
    type Raw = raw::git_push_options;

    unsafe fn from_raw(_raw: raw::git_push_options) -> PushOptions<'cb> {
        panic!("unimplemented");
    }
    fn raw(&self) -> raw::git_push_options {
        raw::git_push_options {
            version: 1,
            callbacks: self.callbacks.as_ref()
                           .map(|m| m.raw())
                           .unwrap_or_else(|| RemoteCallbacks::new().raw()),
            proxy_opts: self.proxy.as_ref().map(|m| m.raw())
                            .unwrap_or_else(|| ProxyOptions::new().raw()),
            pb_parallelism: self.pb_parallelism as libc::c_uint,
            // TODO: expose this as a builder option
            custom_headers: raw::git_strarray {
                count: 0,
                strings: ptr::null_mut(),
            },
        }
    }
}

impl<'repo, 'connection, 'cb> RemoteConnection<'repo, 'connection, 'cb> {
    /// Check whether the remote is (still) connected
    pub fn connected(&mut self) -> bool {
        self.remote.connected()
    }

    /// Get the remote repository's reference advertisement list.
    ///
    /// This list is available as soon as the connection to
    /// the remote is initiated and it remains available after disconnecting.
    pub fn list(&self) -> Result<&[RemoteHead], Error> {
        self.remote.list()
    }
}

impl<'repo, 'connection, 'cb> Drop for RemoteConnection<'repo, 'connection, 'cb> {
    fn drop(&mut self) {
        self.remote.disconnect()
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use tempdir::TempDir;
    use {Repository, Remote, RemoteCallbacks, Direction, FetchOptions};
    use {AutotagOption, PushOptions};

    #[test]
    fn smoke() {
        let (td, repo) = ::test::repo_init();
        t!(repo.remote("origin", "/path/to/nowhere"));
        drop(repo);

        let repo = t!(Repository::init(td.path()));
        let origin = t!(repo.find_remote("origin"));
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.url(), Some("/path/to/nowhere"));
        assert_eq!(origin.pushurl(), None);

        t!(repo.remote_set_url("origin", "/path/to/elsewhere"));
        t!(repo.remote_set_pushurl("origin", Some("/path/to/elsewhere")));

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
        origin.download(&[], None).unwrap();
        origin.disconnect();

        {
            let mut connection = origin.connect_auth(Direction::Push, None, None).unwrap();
            assert!(connection.connected());
        }
        assert!(!origin.connected());

        {
            let mut connection = origin.connect_auth(Direction::Fetch, None, None).unwrap();
            assert!(connection.connected());
        }
        assert!(!origin.connected());

        origin.fetch(&[], None, None).unwrap();
        origin.fetch(&[], None, Some("foo")).unwrap();
        origin.update_tips(None, true, AutotagOption::Unspecified, None).unwrap();
        origin.update_tips(None, true, AutotagOption::All, Some("foo")).unwrap();

        t!(repo.remote_add_fetch("origin", "foo"));
        t!(repo.remote_add_fetch("origin", "bar"));
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

        let origin = repo.remote_anonymous("/path/to/nowhere").unwrap();
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
            origin.fetch(&[],
                         Some(FetchOptions::new().remote_callbacks(callbacks)),
                         None).unwrap();

            let list = t!(origin.list());
            assert_eq!(list.len(), 2);
            assert_eq!(list[0].name(), "HEAD");
            assert!(!list[0].is_local());
            assert_eq!(list[1].name(), "refs/heads/master");
            assert!(!list[1].is_local());
        }
        assert!(progress_hit.get());
    }

    /// This test is meant to assure that the callbacks provided to connect will not cause
    /// segfaults
    #[test]
    fn connect_list() {
        let (td, _repo) = ::test::repo_init();
        let td2 = TempDir::new("git").unwrap();
        let url = ::test::path2url(&td.path());

        let repo = Repository::init(td2.path()).unwrap();
        let mut callbacks = RemoteCallbacks::new();
        callbacks.sideband_progress(|_progress| {
            // no-op
            true
        });

        let mut origin = repo.remote("origin", &url).unwrap();

        {
            let mut connection = origin.connect_auth(Direction::Fetch, Some(callbacks), None).unwrap();
            assert!(connection.connected());

            let list = t!(connection.list());
            assert_eq!(list.len(), 2);
            assert_eq!(list[0].name(), "HEAD");
            assert!(!list[0].is_local());
            assert_eq!(list[1].name(), "refs/heads/master");
            assert!(!list[1].is_local());
        }
        assert!(!origin.connected());
    }

    #[test]
    fn push() {
        let (_td, repo) = ::test::repo_init();
        let td2 = TempDir::new("git1").unwrap();
        let td3 = TempDir::new("git2").unwrap();
        let url = ::test::path2url(&td2.path());

        Repository::init_bare(td2.path()).unwrap();
        // git push
        let mut remote = repo.remote("origin", &url).unwrap();
        let mut updated = false;
        {
            let mut callbacks = RemoteCallbacks::new();
            callbacks.push_update_reference(|refname, status| {
                updated = true;
                assert_eq!(refname, "refs/heads/master");
                assert_eq!(status, None);
                Ok(())
            });
            let mut options = PushOptions::new();
            options.remote_callbacks(callbacks);
            remote.push(&["refs/heads/master"], Some(&mut options)).unwrap();
        }
        assert!(updated);

        let repo = Repository::clone(&url, td3.path()).unwrap();
        let commit = repo.head().unwrap().target().unwrap();
        let commit = repo.find_commit(commit).unwrap();
        assert_eq!(commit.message(), Some("initial"));
    }
}
