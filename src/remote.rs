use std::c_str::CString;
use std::kinds::marker;
use std::str;
use libc;

use {raw, Repository, Direction, Fetch, Push, Error, Refspec, StringArray};
use Signature;

pub struct Remote<'a> {
    raw: *mut raw::git_remote,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoShare,
}

pub struct Refspecs<'a> {
    cur: uint,
    cnt: uint,
    remote: &'a Remote<'a>,
}

impl<'a> Remote<'a> {
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_remote) -> Remote {
        Remote {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }

    /// Ensure the remote name is well-formed.
    pub fn is_valid_name(remote_name: &str) -> bool {
        let remote_name = remote_name.to_c_str();
        unsafe { raw::git_remote_is_valid_name(remote_name.as_ptr()) == 1 }
    }

    /// Return whether a string is a valid remote URL
    pub fn is_valid_url(url: &str) -> bool {
        let url = url.to_c_str();
        unsafe { raw::git_remote_valid_url(url.as_ptr()) == 1 }
    }

    /// Return whether the passed URL is supported by this version of the
    /// library.
    pub fn is_supported_url(url: &str) -> bool {
        let url = url.to_c_str();
        unsafe { raw::git_remote_supported_url(url.as_ptr()) == 1 }
    }

    /// Get the remote's name.
    ///
    /// Returns `None` if this remote has not yet been named or if the name is
    /// not valid utf-8
    pub fn name(&self) -> Option<&str> {
        self.name_bytes().and_then(str::from_utf8)
    }

    /// Get the remote's name, in bytes.
    ///
    /// Returns `None` if this remote has not yet been named
    pub fn name_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_remote_name(&*self.raw)) }
    }

    /// Get the remote's owner.
    ///
    /// Returns `None` if the owner is not valid utf-8
    pub fn owner(&self) -> Option<&str> {
        str::from_utf8(self.owner_bytes())
    }

    /// Get the remote's owner as a byte array.
    pub fn owner_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_remote_owner(&*self.raw)).unwrap() }
    }

    /// Get the remote's url.
    ///
    /// Returns `None` if the owner is not valid utf-8
    pub fn url(&self) -> Option<&str> {
        str::from_utf8(self.url_bytes())
    }

    /// Get the remote's url as a byte array.
    pub fn url_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_remote_url(&*self.raw)).unwrap() }
    }

    /// Get the remote's pushurl.
    ///
    /// Returns `None` if the owner is not valid utf-8
    pub fn pushurl(&self) -> Option<&str> {
        self.pushurl_bytes().and_then(str::from_utf8)
    }

    /// Get the remote's pushurl as a byte array.
    pub fn pushurl_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_remote_pushurl(&*self.raw)) }
    }

    /// Open a connection to a remote.
    pub fn connect(&mut self, dir: Direction) -> Result<(), Error> {
        try!(::doit(|| unsafe {
            raw::git_remote_connect(self.raw, match dir {
                Fetch => raw::GIT_DIRECTION_FETCH,
                Push => raw::GIT_DIRECTION_PUSH,
            })
        }));
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
        try!(::doit(|| unsafe { raw::git_remote_save(&*self.raw) }));
        Ok(())
    }

    /// Add a fetch refspec to the remote
    pub fn add_fetch(&mut self, spec: &str) -> Result<(), Error> {
        let spec = spec.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_remote_add_fetch(self.raw, spec.as_ptr())
        }));
        Ok(())
    }

    /// Add a push refspec to the remote
    pub fn add_push(&mut self, spec: &str) -> Result<(), Error> {
        let spec = spec.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_remote_add_push(self.raw, spec.as_ptr())
        }));
        Ok(())
    }

    /// Choose whether to check the server's certificate (applies to HTTPS only)
    ///
    /// The default is yes.
    pub fn set_check_cert(&mut self, check: bool) {
        unsafe { raw::git_remote_check_cert(self.raw, check as libc::c_int) }
    }

    /// Set the remote's url
    ///
    /// Existing connections will not be updated.
    pub fn set_url(&mut self, url: &str) -> Result<(), Error> {
        let url = url.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_remote_set_url(self.raw, url.as_ptr())
        }));
        Ok(())
    }

    /// Set the remote's pushurl.
    ///
    /// `None` indicates that it should be cleared.
    ///
    /// Existing connections will not be updated.
    pub fn set_pushurl(&mut self, pushurl: Option<&str>) -> Result<(), Error> {
        let pushurl = pushurl.map(|s| s.to_c_str());
        let pushurl = pushurl.as_ref().map(|s| s.as_ptr()).unwrap_or(0 as *const _);
        try!(::doit(|| unsafe {
            raw::git_remote_set_pushurl(self.raw, pushurl)
        }));
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

        try!(::doit(|| unsafe {
            raw::git_remote_set_fetch_refspecs(self.raw, &mut arr)
        }));
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

        try!(::doit(|| unsafe {
            raw::git_remote_set_push_refspecs(self.raw, &mut arr)
        }));
        Ok(())
    }

    /// Clear the refspecs
    ///
    /// Remove all configured fetch and push refspecs from the remote.
    pub fn clear_refspecs(&mut self) {
        unsafe { raw::git_remote_clear_refspecs(self.raw) }
    }

    /// Delete an existing persisted remote.
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// will be removed.
    pub fn delete(&mut self) -> Result<(), Error> {
        try!(::doit(|| unsafe { raw::git_remote_delete(self.raw) }));
        Ok(())
    }

    /// Download and index the packfile
    ///
    /// Connect to the remote if it hasn't been done yet, negotiate with the
    /// remote git which objects are missing, download and index the packfile.
    ///
    /// The .idx file will be created and both it and the packfile with be
    /// renamed to their final name.
    pub fn download(&mut self) -> Result<(), Error> {
        try!(::doit(|| unsafe { raw::git_remote_download(self.raw) }));
        Ok(())
    }

    /// Get the number of refspecs for a remote
    pub fn refspecs(&self) -> Refspecs {
        let cnt = unsafe { raw::git_remote_refspec_count(&*self.raw) as uint };
        Refspecs { cur: 0, cnt: cnt, remote: self }
    }

    /// Give the remote a new name
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// are updated.
    ///
    /// A temporary in-memory remote cannot be given a name with this method.
    pub fn rename(&mut self, new_name: &str) -> Result<(), Error> {
        let mut problems = raw::git_strarray {
            count: 0,
            strings: 0 as *mut *mut libc::c_char,
        };
        let new_name = new_name.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_remote_rename(&mut problems, self.raw, new_name.as_ptr())
        }));
        let _s = unsafe { StringArray::from_raw(problems) };
        Ok(())
    }

    /// Download new data and update tips
    ///
    /// Convenience function to connect to a remote, download the data,
    /// disconnect and update the remote-tracking branches.
    pub fn fetch(&mut self, signature: &Signature,
                 msg: Option<&str>) -> Result<(), Error> {
        let msg = msg.map(|s| s.to_c_str());
        let msg = msg.as_ref().map(|s| s.as_ptr()).unwrap_or(0 as *const _);

        try!(::doit(|| unsafe {
            raw::git_remote_fetch(self.raw, signature.raw(), msg)
        }));
        Ok(())
    }

    /// Update the tips to the new state
    pub fn update_tips(&mut self, signature: &Signature,
                       msg: Option<&str>) -> Result<(), Error> {
        let msg = msg.map(|s| s.to_c_str());
        let msg = msg.as_ref().map(|s| s.as_ptr()).unwrap_or(0 as *const _);

        try!(::doit(|| unsafe {
            raw::git_remote_update_tips(self.raw, signature.raw(), msg)
        }));
        Ok(())
    }

    /// Retrieve the update FETCH_HEAD setting.
    pub fn update_fetchhead(&mut self) -> Result<(), Error> {
        try!(::doit(|| unsafe { raw::git_remote_update_fetchhead(self.raw) }));
        Ok(())
    }
}

impl<'a> Iterator<Refspec<'a>> for Refspecs<'a> {
    fn next(&mut self) -> Option<Refspec<'a>> {
        if self.cur == self.cnt { return None }
        let ret = unsafe {
            let ptr = raw::git_remote_get_refspec(&*self.remote.raw,
                                                  self.cur as libc::size_t);
            assert!(!ptr.is_null());
            Refspec::from_raw(self.remote, ptr)
        };
        self.cur += 1;
        Some(ret)
    }
}

impl<'a> Clone for Remote<'a> {
    fn clone(&self) -> Remote<'a> {
        let mut ret = 0 as *mut raw::git_remote;
        ::doit(|| unsafe {
            raw::git_remote_dup(&mut ret, self.raw)
        }).unwrap();
        Remote {
            raw: ret,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Remote<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_remote_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::TempDir;
    use {Repository, Remote, Signature};

    #[test]
    fn smoke() {
        let td = TempDir::new("test").unwrap();
        git!(td.path(), "init");
        git!(td.path(), "remote", "add", "origin", "/path/to/nowhere");

        let repo = Repository::init(td.path(), false).unwrap();
        let origin = repo.remote_load("origin").unwrap();
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.owner(), Some(""));
        assert_eq!(origin.url(), Some("/path/to/nowhere"));
    }

    #[test]
    fn create_remote() {
        let td = TempDir::new("test").unwrap();
        let repo = td.path().join("repo");
        let remote = td.path().join("remote");
        let repo = Repository::init(&repo, false).unwrap();
        Repository::init(&remote, true).unwrap();

        let url = format!("file://{}", remote.display());
        let mut origin = repo.remote_create("origin", url.as_slice()).unwrap();
        assert_eq!(origin.name(), Some("origin"));
        assert_eq!(origin.owner(), Some(""));
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
            let remotes = repo.remote_list().unwrap();
            assert_eq!(remotes.len(), 1);
            assert_eq!(remotes.get(0), Some("origin"));
            assert_eq!(remotes.iter().count(), 1);
            assert_eq!(remotes.iter().next().unwrap(), Some("origin"));
        }

        origin.connect(::Push).unwrap();
        assert!(origin.connected());
        origin.disconnect();

        origin.connect(::Fetch).unwrap();
        assert!(origin.connected());
        origin.download().unwrap();
        origin.disconnect();

        origin.save().unwrap();

        origin.add_fetch("foo").unwrap();
        origin.add_fetch("bar").unwrap();
        origin.set_check_cert(true);
        origin.clear_refspecs();

        origin.set_fetch_refspecs(["foo"].iter().map(|a| *a)).unwrap();
        origin.set_push_refspecs(["foo"].iter().map(|a| *a)).unwrap();

        origin.rename("origin2").unwrap();
        let sig = Signature::default(&repo).unwrap();
        origin.fetch(&sig, None).unwrap();
        origin.fetch(&sig, Some("foo")).unwrap();
        origin.update_tips(&sig, None).unwrap();
        origin.update_tips(&sig, Some("foo")).unwrap();
        origin.delete().unwrap();
    }

    #[test]
    fn create_remote_anonymous() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path(), false).unwrap();

        let origin = repo.remote_create_anonymous("/path/to/nowhere",
                                                  "master").unwrap();
        assert_eq!(origin.name(), None);
        drop(origin.clone());
    }

    #[test]
    fn is_valid() {
        assert!(Remote::is_valid_name("foobar"));
        assert!(!Remote::is_valid_name("\x01"));
        assert!(Remote::is_valid_url("http://example.com/foo/bar"));
        assert!(!Remote::is_valid_url("test"));
    }
}
