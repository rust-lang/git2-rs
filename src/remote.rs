use std::c_str::{CString, ToCStr};
use std::kinds::marker;
use std::mem;
use std::slice;
use std::str;
use libc;

use {raw, Direction, Error, Refspec, Oid};
use {Signature, Push, RemoteCallbacks, Progress};

/// A structure representing a [remote][1] of a git repository.
///
/// [1]: http://git-scm.com/book/en/Git-Basics-Working-with-Remotes
///
/// The lifetime is the lifetime of the repository that it is attached to. The
/// remote is used to manage fetches and pushes as well as refspecs.
pub struct Remote<'repo, 'cb> {
    raw: *mut raw::git_remote,
    marker: marker::ContravariantLifetime<'repo>,
    callbacks: Option<&'cb mut RemoteCallbacks<'cb>>,
}

/// An iterator over the refspecs that a remote contains.
pub struct Refspecs<'remote, 'cb: 'remote> {
    cur: uint,
    cnt: uint,
    remote: &'remote Remote<'remote, 'cb>,
}

/// Description of a reference advertised bya remote server, given out on calls
/// to `list`.
pub struct RemoteHead<'remote> {
    raw: *const raw::git_remote_head,
    marker: marker::ContravariantLifetime<'remote>,
}

impl<'repo, 'cb> Remote<'repo, 'cb> {
    /// Creates a new remote from its raw pointer.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is valid or
    /// that no other remote is using it.
    pub unsafe fn from_raw(raw: *mut raw::git_remote) -> Remote<'repo, 'cb> {
        Remote {
            raw: raw,
            marker: marker::ContravariantLifetime,
            callbacks: None,
        }
    }

    /// Ensure the remote name is well-formed.
    pub fn is_valid_name(remote_name: &str) -> bool {
        ::init();
        let remote_name = remote_name.to_c_str();
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
        unsafe {
            try_call!(raw::git_remote_add_fetch(self.raw, spec.to_c_str()));
        }
        Ok(())
    }

    /// Add a push refspec to the remote
    pub fn add_push(&mut self, spec: &str) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_remote_add_push(self.raw, spec.to_c_str()));
        }
        Ok(())
    }

    /// Set the remote's url
    ///
    /// Existing connections will not be updated.
    pub fn set_url(&mut self, url: &str) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_remote_set_url(self.raw, url.to_c_str()));
        }
        Ok(())
    }

    /// Set the remote's pushurl.
    ///
    /// `None` indicates that it should be cleared.
    ///
    /// Existing connections will not be updated.
    pub fn set_pushurl(&mut self, pushurl: Option<&str>) -> Result<(), Error> {
        let pushurl = pushurl.map(|s| s.to_c_str());
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
    pub fn set_fetch_refspecs<T: ToCStr, I: Iterator<T>>(&mut self, i: I)
                                                         -> Result<(), Error> {
        let v = i.map(|t| t.to_c_str()).collect::<Vec<CString>>();
        let v2 = v.iter().map(|v| v.as_ptr()).collect::<Vec<*const libc::c_char>>();
        let mut arr = raw::git_strarray {
            strings: v2.as_ptr() as *mut _,
            count: v2.len() as libc::size_t,
        };

        unsafe {
            try_call!(raw::git_remote_set_fetch_refspecs(self.raw, &mut arr));
        }
        Ok(())
    }

    /// Set the remote's list of push refspecs
    pub fn set_push_refspecs<T: ToCStr, I: Iterator<T>>(&mut self, i: I)
                                                         -> Result<(), Error> {
        let v = i.map(|t| t.to_c_str()).collect::<Vec<CString>>();
        let v2 = v.iter().map(|v| v.as_ptr()).collect::<Vec<*const libc::c_char>>();
        let mut arr = raw::git_strarray {
            strings: v2.as_ptr() as *mut _,
            count: v2.len() as libc::size_t,
        };

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
    pub fn download(&mut self) -> Result<(), Error> {
        unsafe {
            try!(self.set_raw_callbacks());
            // FIXME expose refspec array at the API level
            try_call!(raw::git_remote_download(self.raw, 0 as *const _));
        }
        Ok(())
    }

    /// Get the number of refspecs for a remote
    pub fn refspecs<'a>(&'a self) -> Refspecs<'a, 'cb> {
        let cnt = unsafe { raw::git_remote_refspec_count(&*self.raw) as uint };
        Refspecs { cur: 0, cnt: cnt, remote: self }
    }

    /// Download new data and update tips
    ///
    /// Convenience function to connect to a remote, download the data,
    /// disconnect and update the remote-tracking branches.
    pub fn fetch(&mut self,
                 refspecs: &[&str],
                 signature: Option<&Signature>,
                 msg: Option<&str>) -> Result<(), Error> {
        let refspecs = refspecs.iter().map(|s| s.to_c_str()).collect::<Vec<_>>();
        let ptrs = refspecs.iter().map(|s| s.as_ptr()).collect::<Vec<_>>();
        let arr = raw::git_strarray {
            strings: ptrs.as_ptr() as *mut _,
            count: ptrs.len() as libc::size_t,
        };
        unsafe {
            try!(self.set_raw_callbacks());
            try_call!(raw::git_remote_fetch(self.raw,
                                            &arr,
                                            &*signature.map(|s| s.raw())
                                                       .unwrap_or(0 as *mut _),
                                            msg.map(|s| s.to_c_str())));
        }
        Ok(())
    }

    /// Update the tips to the new state
    pub fn update_tips(&mut self, signature: Option<&Signature>,
                       msg: Option<&str>) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_remote_update_tips(self.raw,
                                                  &*signature.map(|s| s.raw())
                                                             .unwrap_or(0 as *mut _),
                                                  msg.map(|s| s.to_c_str())));
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
            Ok(Push::from_raw(ret))
        }
    }

    /// Set the callbacks to be invoked when the transfer is in-progress.
    ///
    /// This will overwrite the previously set callbacks.
    pub fn set_callbacks(&mut self, callbacks: &'cb mut RemoteCallbacks<'cb>) {
        self.callbacks = Some(callbacks);
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
            Progress::from_raw(raw::git_remote_stats(self.raw))
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
            let base = base as *const _;
            let slice = slice::from_raw_buf(&base, size as uint);
            Ok(mem::transmute::<&[*const raw::git_remote_head],
                                &[RemoteHead]>(slice))
        }
    }
}

impl<'a, 'b> Iterator<Refspec<'a>> for Refspecs<'a, 'b> {
    fn next(&mut self) -> Option<Refspec<'a>> {
        if self.cur == self.cnt { return None }
        let ret = unsafe {
            let ptr = raw::git_remote_get_refspec(&*self.remote.raw,
                                                  self.cur as libc::size_t);
            assert!(!ptr.is_null());
            Refspec::from_raw(ptr)
        };
        self.cur += 1;
        Some(ret)
    }
}

impl<'a, 'b> Clone for Remote<'a, 'b> {
    fn clone(&self) -> Remote<'a, 'b> {
        let mut ret = 0 as *mut raw::git_remote;
        let rc = unsafe { call!(raw::git_remote_dup(&mut ret, self.raw)) };
        assert_eq!(rc, 0);
        Remote {
            raw: ret,
            marker: marker::ContravariantLifetime,
            callbacks: None,
        }
    }
}

#[unsafe_destructor]
impl<'a, 'b> Drop for Remote<'a, 'b> {
    fn drop(&mut self) {
        unsafe { raw::git_remote_free(self.raw) }
    }
}

#[allow(missing_docs)] // not documented in libgit2 :(
impl<'remote> RemoteHead<'remote> {
    /// Flag if this is available locally.
    pub fn is_local(&self) -> bool {
        unsafe { (*self.raw).local != 0 }
    }


    pub fn oid(&self) -> Oid { unsafe { Oid::from_raw(&(*self.raw).oid) } }
    pub fn loid(&self) -> Oid { unsafe { Oid::from_raw(&(*self.raw).loid) } }

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
    use std::io::TempDir;
    use std::cell::Cell;
    use url::Url;
    use {Repository, Remote, RemoteCallbacks, Direction};

    #[test]
    fn smoke() {
        let (td, repo) = ::test::repo_init();
        repo.remote("origin", "/path/to/nowhere").unwrap();
        drop(repo);

        let repo = Repository::init(td.path()).unwrap();
        let origin = repo.find_remote("origin").unwrap();
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.url(), Some("/path/to/nowhere"));
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
                                        .as_slice().replace("\\", "/"))
        };
        let mut origin = repo.remote("origin", url.as_slice()).unwrap();
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.url(), Some(url.as_slice()));
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
        origin.download().unwrap();
        origin.disconnect();

        origin.save().unwrap();

        origin.add_fetch("foo").unwrap();
        origin.add_fetch("bar").unwrap();
        origin.clear_refspecs();

        origin.set_fetch_refspecs(["foo"].iter().map(|a| *a)).unwrap();
        origin.set_push_refspecs(["foo"].iter().map(|a| *a)).unwrap();

        let sig = repo.signature().unwrap();
        origin.fetch(&[], Some(&sig), None).unwrap();
        origin.fetch(&[], None, Some("foo")).unwrap();
        origin.update_tips(Some(&sig), None).unwrap();
        origin.update_tips(None, Some("foo")).unwrap();
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
        let url = Url::from_file_path(td.path()).unwrap();
        let url = url.to_string();

        let repo = Repository::init(td2.path()).unwrap();
        let mut origin = repo.remote("origin", url.as_slice()).unwrap();

        let progress_hit = Cell::new(false);
        let mut callbacks = RemoteCallbacks::new();
        callbacks.transfer_progress(|_progress| {
            progress_hit.set(true);
            true
        });
        origin.set_callbacks(&mut callbacks);
        origin.fetch(&[], None, None).unwrap();
        assert!(progress_hit.get());
    }
}
