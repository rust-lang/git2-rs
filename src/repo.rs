use std::c_str::CString;
use std::kinds::marker;
use std::str;
use libc::{c_int, c_uint, c_char};

use {raw, Revspec, Error, doit, init, Object, RepositoryState, Remote};
use StringArray;

pub struct Repository {
    raw: *mut raw::git_repository,
    marker1: marker::NoShare,
    marker2: marker::NoSend,
}

impl Repository {
    /// Attempt to open an already-existing repository at `path`.
    ///
    /// The path can point to either a normal or bare repository.
    pub fn open(path: &Path) -> Result<Repository, Error> {
        init();
        let s = path.to_c_str();
        let mut ret = 0 as *mut raw::git_repository;
        try!(doit(|| unsafe {
            raw::git_repository_open(&mut ret, s.as_ptr())
        }));
        Ok(Repository {
            raw: ret,
            marker1: marker::NoShare,
            marker2: marker::NoSend,
        })
    }

    /// Creates a new repository in the specified folder.
    ///
    /// The folder must exist prior to invoking this function.
    pub fn init(path: &Path, bare: bool) -> Result<Repository, Error> {
        init();
        let s = path.to_c_str();
        let mut ret = 0 as *mut raw::git_repository;
        try!(doit(|| unsafe {
            raw::git_repository_init(&mut ret, s.as_ptr(), bare as c_uint)
        }));
        Ok(Repository {
            raw: ret,
            marker1: marker::NoShare,
            marker2: marker::NoSend,
        })
    }

    /// Execute a rev-parse operation against the `spec` listed.
    ///
    /// The resulting revision specification is returned, or an error is
    /// returned if one occurs.
    pub fn revparse(&self, spec: &str) -> Result<Revspec, Error> {
        let s = spec.to_c_str();
        let mut spec = raw::git_revspec {
            from: 0 as *mut _,
            to: 0 as *mut _,
            flags: raw::git_revparse_mode_t::empty(),
        };
        try!(doit(|| unsafe {
            raw::git_revparse(&mut spec, self.raw, s.as_ptr())
        }));

        if spec.flags.contains(raw::GIT_REVPARSE_SINGLE) {
            assert!(spec.to.is_null());
            let obj = unsafe { Object::from_raw(self, spec.from) };
            Ok(Revspec::from_objects(Some(obj), None))
        } else {
            fail!()
        }
    }

    /// Find a single object, as specified by a revision string.
    pub fn revparse_single(&self, spec: &str) -> Result<Object, Error> {
        let s = spec.to_c_str();
        let mut obj = 0 as *mut raw::git_object;
        try!(doit(|| unsafe {
            raw::git_revparse_single(&mut obj, self.raw, s.as_ptr())
        }));
        assert!(!obj.is_null());
        Ok(unsafe { Object::from_raw(self, obj) })
    }

    /// Tests whether this repository is a bare repository or not.
    pub fn is_bare(&self) -> bool {
        unsafe { raw::git_repository_is_bare(self.raw) == 1 }
    }

    /// Tests whether this repository is a shallow clone.
    pub fn is_shallow(&self) -> bool {
        unsafe { raw::git_repository_is_shallow(self.raw) == 1 }
    }

    /// Tests whether this repository is empty.
    pub fn is_empty(&self) -> Result<bool, Error> {
        let empty = try!(doit(|| unsafe {
            raw::git_repository_is_empty(self.raw)
        }));
        Ok(empty == 1)
    }

    /// Returns the path to the `.git` folder for normal repositories or the
    /// repository itself for bare repositories.
    pub fn path(&self) -> Path {
        unsafe {
            let ptr = raw::git_repository_path(self.raw);
            assert!(!ptr.is_null());
            Path::new(CString::new(ptr, false).as_bytes_no_nul())
        }
    }

    /// Returns the current state of this repository
    pub fn state(&self) -> RepositoryState {
        let state = unsafe { raw::git_repository_state(self.raw) };
        macro_rules! check( ($($raw:ident => $real:ident),*) => (
            $(if state == raw::$raw as c_int { super::$real }) else *
            else {
                fail!("unknown repository state: {}", state)
            }
        ) )

        check!(
            GIT_REPOSITORY_STATE_NONE => Clean,
            GIT_REPOSITORY_STATE_MERGE => Merge,
            GIT_REPOSITORY_STATE_REVERT => Revert,
            GIT_REPOSITORY_STATE_CHERRYPICK => CherryPick,
            GIT_REPOSITORY_STATE_BISECT => Bisect,
            GIT_REPOSITORY_STATE_REBASE => Rebase,
            GIT_REPOSITORY_STATE_REBASE_INTERACTIVE => RebaseInteractive,
            GIT_REPOSITORY_STATE_REBASE_MERGE => RebaseMerge,
            GIT_REPOSITORY_STATE_APPLY_MAILBOX => ApplyMailbox,
            GIT_REPOSITORY_STATE_APPLY_MAILBOX_OR_REBASE => ApplyMailboxOrRebase
        )
    }

    /// Get the path of the working directory for this repository.
    ///
    /// If this repository is bare, then `None` is returned.
    pub fn workdir(&self) -> Option<Path> {
        unsafe {
            let ptr = raw::git_repository_workdir(self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(Path::new(CString::new(ptr, false).as_bytes_no_nul()))
            }
        }
    }

    /// Get the currently active namespace for this repository.
    ///
    /// If there is no namespace, or the namespace is not a valid utf8 string,
    /// `None` is returned.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace_bytes().and_then(str::from_utf8)
    }

    /// Get the currently active namespace for this repository as a byte array.
    ///
    /// If there is no namespace, `None` is returned.
    pub fn namespace_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_repository_get_namespace(self.raw)) }
    }

    /// List all remotes for a given repository
    pub fn remote_list(&self) -> Result<StringArray, Error> {
        let mut arr = raw::git_strarray {
            strings: 0 as *mut *mut c_char,
            count: 0,
        };
        try!(::doit(|| unsafe {
            raw::git_remote_list(&mut arr, self.raw)
        }));
        Ok(unsafe { StringArray::from_raw(arr) })
    }

    /// Get the information for a particular remote
    pub fn remote_load(&self, name: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        let name = name.to_c_str();
        try!(doit(|| unsafe {
            raw::git_remote_load(&mut ret, self.raw, name.as_ptr())
        }));
        Ok(unsafe { Remote::from_raw(self, ret) })
    }

    /// Add a remote with the default fetch refspec to the repository's
    /// configuration.
    pub fn remote_create(&self, name: &str, url: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        let name = name.to_c_str();
        let url = url.to_c_str();
        try!(doit(|| unsafe {
            raw::git_remote_create(&mut ret, self.raw, name.as_ptr(),
                                   url.as_ptr())
        }));
        Ok(unsafe { Remote::from_raw(self, ret) })
    }

    /// Create an anonymous remote
    ///
    /// Create a remote with the given url and refspec in memory. You can use
    /// this when you have a URL instead of a remote's name. Note that anonymous
    /// remotes cannot be converted to persisted remotes.
    pub fn remote_create_anonymous(&self, url: &str,
                                   fetch: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        let url = url.to_c_str();
        let fetch = fetch.to_c_str();
        try!(doit(|| unsafe {
            raw::git_remote_create_anonymous(&mut ret, self.raw, url.as_ptr(),
                                             fetch.as_ptr())
        }));
        Ok(unsafe { Remote::from_raw(self, ret) })
    }

    /// Get the underlying raw repository
    pub fn raw(&self) -> *mut raw::git_repository { self.raw }
}

#[unsafe_destructor]
impl Drop for Repository {
    fn drop(&mut self) {
        unsafe { raw::git_repository_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{TempDir, File};
    use super::Repository;

    #[test]
    fn smoke_init() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path, false).unwrap();
        assert!(!repo.is_bare());
    }

    #[test]
    fn smoke_init_bare() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path, true).unwrap();
        assert!(repo.is_bare());
        assert!(repo.namespace().is_none());
    }

    #[test]
    fn smoke_open() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();
        git!(td.path(), "init");

        let repo = Repository::open(path).unwrap();
        assert!(!repo.is_bare());
        assert!(!repo.is_shallow());
        assert!(repo.is_empty().unwrap());
        assert!(repo.path() == td.path().join(".git"));
        assert_eq!(repo.state(), ::Clean);
    }

    #[test]
    fn smoke_open_bare() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();
        git!(td.path(), "init", "--bare");

        let repo = Repository::open(path).unwrap();
        assert!(repo.is_bare());
        assert!(repo.path() == *td.path());
    }

    #[test]
    fn smoke_revparse() {
        let td = TempDir::new("test").unwrap();
        git!(td.path(), "init");
        File::create(&td.path().join("foo")).write_str("foobar").unwrap();
        git!(td.path(), "add", ".");
        git!(td.path(), "commit", "-m", "foo");
        let expected_rev = git!(td.path(), "rev-parse", "HEAD");

        let repo = Repository::open(td.path()).unwrap();
        let actual_rev = repo.revparse("HEAD").unwrap();
        let from = actual_rev.from().unwrap();
        assert!(actual_rev.to().is_none());
        assert_eq!(expected_rev, from.id().to_string());

        assert_eq!(repo.revparse_single("HEAD").unwrap().id().to_string(),
                   expected_rev);
    }
}
