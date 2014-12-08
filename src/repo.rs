use std::c_str::CString;
use std::kinds::marker;
use std::mem;
use std::str;
use libc::{c_int, c_char, size_t, c_void};

use {raw, Revspec, Error, init, Object, RepositoryState, Remote, Buf};
use {StringArray, ResetType, Signature, Reference, References, Submodule};
use {Branches, BranchType, Index, Config, Oid, Blob, Branch, Commit, Tree};
use {ObjectType, Tag};
use build::{RepoBuilder, CheckoutBuilder};

/// An owned git repository, representing all state associated with the
/// underlying filesystem.
///
/// This structure corresponds to a `git_repository` in libgit2. Many other
/// types in git2-rs are derivative from this structure and are attached to its
/// lifetime.
///
/// When a repository goes out of scope it is freed in memory but not deleted
/// from the filesystem.
pub struct Repository {
    raw: *mut raw::git_repository,
    marker: marker::NoSync,
}

/// Options which can be used to configure how a repository is initialized
pub struct RepositoryInitOptions {
    flags: u32,
    mode: u32,
    workdir_path: Option<CString>,
    description: Option<CString>,
    template_path: Option<CString>,
    initial_head: Option<CString>,
    origin_url: Option<CString>,
}

impl Repository {
    /// Attempt to open an already-existing repository at `path`.
    ///
    /// The path can point to either a normal or bare repository.
    pub fn open(path: &Path) -> Result<Repository, Error> {
        init();
        let mut ret = 0 as *mut raw::git_repository;
        unsafe {
            try_call!(raw::git_repository_open(&mut ret, path.to_c_str()));
        }
        Ok(unsafe { Repository::from_raw(ret) })
    }

    /// Attempt to open an already-existing repository at or above `path`
    ///
    /// This starts at `path` and looks up the filesystem hierarchy
    /// until it finds a repository.
    pub fn discover(path: &Path) -> Result<Repository, Error> {
        init();
        unsafe {
            let mut raw: raw::git_buf = mem::zeroed();
            try_call!(raw::git_repository_discover(&mut raw,
                                                   path.to_c_str(),
                                                   1i32,
                                                   0 as *const c_char));
            let buf = Buf::from_raw(raw);
            Repository::open(&Path::new(buf.get()))
        }
    }

    /// Creates a new repository in the specified folder.
    ///
    /// This by default will create any necessary directories to create the
    /// repository, and it will read any user-specified templates when creating
    /// the repository. This behavior can be configured through `init_opts`.
    pub fn init(path: &Path) -> Result<Repository, Error> {
        Repository::init_opts(path, &RepositoryInitOptions::new())
    }

    /// Creates a new `--bare` repository in the specified folder.
    ///
    /// The folder must exist prior to invoking this function.
    pub fn init_bare(path: &Path) -> Result<Repository, Error> {
        Repository::init_opts(path, &RepositoryInitOptions::new().bare(true))
    }

    /// Creates a new `--bare` repository in the specified folder.
    ///
    /// The folder must exist prior to invoking this function.
    pub fn init_opts(path: &Path, opts: &RepositoryInitOptions)
                     -> Result<Repository, Error> {
        init();
        let mut ret = 0 as *mut raw::git_repository;
        unsafe {
            let mut opts = opts.raw();
            try_call!(raw::git_repository_init_ext(&mut ret,
                                                   path.to_c_str(),
                                                   &mut opts));
        }
        Ok(unsafe { Repository::from_raw(ret) })
    }

    /// Clone a remote repository.
    ///
    /// See the `RepoBuilder` struct for more information. This function will
    /// delegate to a fresh `RepoBuilder`
    pub fn clone(url: &str, into: &Path) -> Result<Repository, Error> {
        ::init();
        RepoBuilder::new().clone(url, into)
    }

    /// Create a repository from the raw underlying pointer.
    ///
    /// This function will take ownership of the pointer specified.
    pub unsafe fn from_raw(ptr: *mut raw::git_repository) -> Repository {
        Repository {
            raw: ptr,
            marker: marker::NoSync,
        }
    }

    /// Execute a rev-parse operation against the `spec` listed.
    ///
    /// The resulting revision specification is returned, or an error is
    /// returned if one occurs.
    pub fn revparse(&self, spec: &str) -> Result<Revspec, Error> {
        let mut raw = raw::git_revspec {
            from: 0 as *mut _,
            to: 0 as *mut _,
            flags: raw::git_revparse_mode_t::empty(),
        };
        unsafe {
            try_call!(raw::git_revparse(&mut raw, self.raw, spec.to_c_str()));
        }

        if raw.flags.contains(raw::GIT_REVPARSE_SINGLE) {
            assert!(raw.to.is_null());
            let obj = unsafe { Object::from_raw(self, raw.from) };
            Ok(Revspec::from_objects(Some(obj), None))
        } else {
            panic!()
        }
    }

    /// Find a single object, as specified by a revision string.
    pub fn revparse_single(&self, spec: &str) -> Result<Object, Error> {
        let mut obj = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_revparse_single(&mut obj, self.raw,
                                               spec.to_c_str()));
        }
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
        let empty = unsafe {
            try_call!(raw::git_repository_is_empty(self.raw))
        };
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
            $(if state == raw::$raw as c_int {
                super::RepositoryState::$real
            }) else *
            else {
                panic!("unknown repository state: {}", state)
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
    pub fn remotes(&self) -> Result<StringArray, Error> {
        let mut arr = raw::git_strarray {
            strings: 0 as *mut *mut c_char,
            count: 0,
        };
        unsafe {
            try_call!(raw::git_remote_list(&mut arr, self.raw));
        }
        Ok(unsafe { StringArray::from_raw(arr) })
    }

    /// Get the information for a particular remote
    pub fn find_remote(&self, name: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        unsafe {
            try_call!(raw::git_remote_lookup(&mut ret, self.raw,
                                             name.to_c_str()));
            Ok(Remote::from_raw(self, ret))
        }
    }

    /// Add a remote with the default fetch refspec to the repository's
    /// configuration.
    pub fn remote(&self, name: &str, url: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        unsafe {
            try_call!(raw::git_remote_create(&mut ret, self.raw,
                                             name.to_c_str(), url.to_c_str()));
            Ok(Remote::from_raw(self, ret))
        }
    }

    /// Create an anonymous remote
    ///
    /// Create a remote with the given url and refspec in memory. You can use
    /// this when you have a URL instead of a remote's name. Note that anonymous
    /// remotes cannot be converted to persisted remotes.
    pub fn remote_anonymous(&self, url: &str,
                            fetch: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        unsafe {
            try_call!(raw::git_remote_create_anonymous(&mut ret, self.raw,
                                                       url.to_c_str(),
                                                       fetch.to_c_str()));
            Ok(Remote::from_raw(self, ret))
        }
    }

    /// Give a remote a new name
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// are updated.
    ///
    /// A temporary in-memory remote cannot be given a name with this method.
    pub fn remote_rename(&self, name: &str,
                         new_name: &str) -> Result<(), Error> {
        let mut problems = raw::git_strarray {
            count: 0,
            strings: 0 as *mut *mut c_char,
        };
        unsafe {
            try_call!(raw::git_remote_rename(&mut problems,
                                             self.raw,
                                             name.to_c_str(),
                                             new_name.to_c_str()));
            let _s = StringArray::from_raw(problems);
        }
        Ok(())
    }

    /// Delete an existing persisted remote.
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// will be removed.
    pub fn remote_delete(&self, name: &str) -> Result<(), Error> {
        unsafe { try_call!(raw::git_remote_delete(self.raw, name.to_c_str())); }
        Ok(())
    }

    /// Get the underlying raw repository
    pub fn raw(&self) -> *mut raw::git_repository { self.raw }

    /// Sets the current head to the specified object and optionally resets
    /// the index and working tree to match.
    ///
    /// A soft reset means the head will be moved to the commit.
    ///
    /// A mixed reset will trigger a soft reset, plus the index will be
    /// replaced with the content of the commit tree.
    ///
    /// A hard reset will trigger a mixed reset and the working directory will
    /// be replaced with the content of the index. (Untracked and ignored files
    /// will be left alone, however.)
    pub fn reset<'a>(&'a self, target: &Object<'a>, kind: ResetType,
                     sig: Option<&Signature>, msg: Option<&str>)
                     -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_reset(self.raw, target.raw(), kind,
                                     // FIXME: expose git_checkout_options_t
                                     0 as *mut _,
                                     sig.map(|s| s.raw()).unwrap_or(0 as *mut _),
                                     msg.map(|s| s.to_c_str())));
        }
        Ok(())
    }

    /// Updates some entries in the index from the target commit tree.
    ///
    /// The scope of the updated entries is determined by the paths being
    /// in the iterator provided.
    ///
    /// Passing a `None` target will result in removing entries in the index
    /// matching the provided pathspecs.
    pub fn reset_default<'a,
                         T: ToCStr,
                         I: Iterator<T>>(&'a self,
                                         target: Option<&Object<'a>>,
                                         paths: I) -> Result<(), Error> {
        let v = paths.map(|t| t.to_c_str()).collect::<Vec<CString>>();
        let v2 = v.iter().map(|v| v.as_ptr()).collect::<Vec<*const c_char>>();
        let mut arr = raw::git_strarray {
            strings: v2.as_ptr() as *mut _,
            count: v2.len() as size_t,
        };
        let target = target.map(|t| t.raw()).unwrap_or(0 as *mut _);

        unsafe {
            try_call!(raw::git_reset_default(self.raw, target, &mut arr));
        }
        Ok(())
    }

    /// Retrieve and resolve the reference pointed at by HEAD.
    pub fn head(&self) -> Result<Reference, Error> {
        let mut ret = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_repository_head(&mut ret, self.raw));
            Ok(Reference::from_raw(self, ret))
        }
    }

    /// Create an iterator for the repo's references
    pub fn references(&self) -> Result<References, Error> {
        let mut ret = 0 as *mut raw::git_reference_iterator;
        unsafe {
            try_call!(raw::git_reference_iterator_new(&mut ret, self.raw));
            Ok(References::from_raw(self, ret))
        }
    }

    /// Create an iterator for the repo's references that match the specified
    /// glob
    pub fn references_glob(&self, glob: &str) -> Result<References, Error> {
        let mut ret = 0 as *mut raw::git_reference_iterator;
        unsafe {
            try_call!(raw::git_reference_iterator_glob_new(&mut ret, self.raw,
                                                           glob.to_c_str()));
            Ok(References::from_raw(self, ret))
        }
    }

    /// Load all submodules for this repository and return them.
    pub fn submodules(&self) -> Result<Vec<Submodule>, Error> {
        struct Data<'a, 'b:'a> {
            repo: &'b Repository,
            ret: &'a mut Vec<Submodule<'b>>,
        }
        let mut ret = Vec::new();

        unsafe {
            let mut data = Data {
                repo: self,
                ret: &mut ret,
            };
            try_call!(raw::git_submodule_foreach(self.raw, append,
                                                 &mut data as *mut _
                                                           as *mut c_void));
        }

        return Ok(ret);

        extern fn append(_repo: *mut raw::git_submodule,
                         name: *const c_char,
                         data: *mut c_void) -> c_int {
            unsafe {
                let data = &mut *(data as *mut Data);
                let mut raw = 0 as *mut raw::git_submodule;
                let rc = raw::git_submodule_lookup(&mut raw, data.repo.raw(),
                                                   name);
                assert_eq!(rc, 0);
                data.ret.push(Submodule::from_raw(data.repo, raw));
            }
            0
        }
    }

    /// Create an iterator which loops over the requested branches.
    pub fn branches(&self, filter: Option<BranchType>) -> Result<Branches, Error> {
        let mut raw = 0 as *mut raw::git_branch_iterator;
        unsafe {
            try_call!(raw::git_branch_iterator_new(&mut raw, self.raw(), filter));
            Ok(Branches::from_raw(self, raw))
        }
    }

    /// Get the Index file for this repository.
    ///
    /// If a custom index has not been set, the default index for the repository
    /// will be returned (the one located in .git/index).
    pub fn index(&self) -> Result<Index, Error> {
        let mut raw = 0 as *mut raw::git_index;
        unsafe {
            try_call!(raw::git_repository_index(&mut raw, self.raw()));
            Ok(Index::from_raw(raw))
        }
    }

    /// Get the configuration file for this repository.
    ///
    /// If a configuration file has not been set, the default config set for the
    /// repository will be returned, including global and system configurations
    /// (if they are available).
    pub fn config(&self) -> Result<Config, Error> {
        let mut raw = 0 as *mut raw::git_config;
        unsafe {
            try_call!(raw::git_repository_config(&mut raw, self.raw()));
            Ok(Config::from_raw(raw))
        }
    }

    /// Write an in-memory buffer to the ODB as a blob.
    ///
    /// The Oid returned can in turn be passed to `find_blob` to get a handle to
    /// the blob.
    pub fn blob(&self, data: &[u8]) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            let ptr = data.as_ptr() as *const c_void;
            let len = data.len() as size_t;
            try_call!(raw::git_blob_create_frombuffer(&mut raw, self.raw(),
                                                      ptr, len));
            Ok(Oid::from_raw(&raw))
        }
    }

    /// Read a file from the filesystem and write its content to the Object
    /// Database as a loose blob
    ///
    /// The Oid returned can in turn be passed to `find_blob` to get a handle to
    /// the blob.
    pub fn blob_path(&self, path: &Path) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_blob_create_fromdisk(&mut raw, self.raw(),
                                                    path.to_c_str()));
            Ok(Oid::from_raw(&raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_blob(&self, oid: Oid) -> Result<Blob, Error> {
        let mut raw = 0 as *mut raw::git_blob;
        unsafe {
            try_call!(raw::git_blob_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Blob::from_raw(self, raw))
        }
    }

    /// Create a new branch pointing at a target commit
    ///
    /// A new direct reference will be created pointing to this target commit.
    /// If `force` is true and a reference already exists with the given name,
    /// it'll be replaced.
    pub fn branch<'a>(&'a self,
                      branch_name: &str,
                      target: &Commit<'a>,
                      force: bool,
                      signature: Option<&Signature>,
                      log_message: Option<&str>) -> Result<Branch<'a>, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_branch_create(&mut raw,
                                             self.raw(),
                                             branch_name.to_c_str(),
                                             &*target.raw(),
                                             force,
                                             &*signature.map(|s| s.raw())
                                                        .unwrap_or(0 as *mut _),
                                             log_message.map(|s| s.to_c_str())));
            Ok(Branch::wrap(Reference::from_raw(self, raw)))
        }
    }

    /// Lookup a branch by its name in a repository.
    pub fn find_branch(&self, name: &str, branch_type: BranchType)
                       -> Result<Branch, Error> {
        let mut ret = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_branch_lookup(&mut ret, self.raw(),
                                             name.to_c_str(), branch_type));
            Ok(Branch::wrap(Reference::from_raw(self, ret)))
        }
    }

    /// Create new commit in the repository
    ///
    /// If the `update_ref` is not `None`, name of the reference that will be
    /// updated to point to this commit. If the reference is not direct, it will
    /// be resolved to a direct reference. Use "HEAD" to update the HEAD of the
    /// current branch and make it point to this commit. If the reference
    /// doesn't exist yet, it will be created. If it does exist, the first
    /// parent must be the tip of this branch.
    pub fn commit<'a>(&'a self,
                      update_ref: Option<&str>,
                      author: &Signature,
                      committer: &Signature,
                      message: &str,
                      tree: &Tree<'a>,
                      parents: &[&Commit<'a>]) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        let parent_ptrs: Vec<*const raw::git_commit> =  parents.iter().map(|p| {
            p.raw() as *const raw::git_commit
        }).collect();
        unsafe {
            try_call!(raw::git_commit_create(&mut raw,
                                             self.raw(),
                                             update_ref.map(|s| s.to_c_str()),
                                             &*author.raw(),
                                             &*committer.raw(),
                                             0 as *const c_char,
                                             message.to_c_str(),
                                             &*tree.raw(),
                                             parents.len() as size_t,
                                             parent_ptrs.as_ptr()));
            Ok(Oid::from_raw(&raw))
        }
    }


    /// Lookup a reference to one of the commits in a repository.
    pub fn find_commit(&self, oid: Oid) -> Result<Commit, Error> {
        let mut raw = 0 as *mut raw::git_commit;
        unsafe {
            try_call!(raw::git_commit_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Commit::from_raw(self, raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_object(&self, oid: Oid,
                       kind: Option<ObjectType>) -> Result<Object, Error> {
        let mut raw = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_object_lookup(&mut raw, self.raw(), oid.raw(),
                                             kind));
            Ok(Object::from_raw(self, raw))
        }
    }

    /// Create a new direct reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn reference(&self, name: &str, id: Oid, force: bool,
                     sig: Option<&Signature>,
                     log_message: &str) -> Result<Reference, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_reference_create(&mut raw, self.raw(),
                                                name.to_c_str(),
                                                &*id.raw(), force,
                                                &*sig.map(|s| s.raw())
                                                     .unwrap_or(0 as *mut _),
                                                log_message.to_c_str()));
            Ok(Reference::from_raw(self, raw))
        }
    }

    /// Create a new symbolic reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn reference_symbolic(&self, name: &str, target: &str,
                              force: bool, sig: Option<&Signature>,
                              log_message: &str)
                              -> Result<Reference, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_reference_symbolic_create(&mut raw, self.raw(),
                                                         name.to_c_str(),
                                                         target.to_c_str(),
                                                         force,
                                                         &*sig.map(|s| s.raw())
                                                              .unwrap_or(0 as *mut _),
                                                         log_message.to_c_str()));
            Ok(Reference::from_raw(self, raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_reference(&self, name: &str) -> Result<Reference, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_reference_lookup(&mut raw, self.raw(),
                                                name.to_c_str()));
            Ok(Reference::from_raw(self, raw))
        }
    }

    /// Lookup a reference by name and resolve immediately to OID.
    ///
    /// This function provides a quick way to resolve a reference name straight
    /// through to the object id that it refers to. This avoids having to
    /// allocate or free any `Reference` objects for simple situations.
    pub fn refname_to_id(&self, name: &str) -> Result<Oid, Error> {
        let mut ret: raw::git_oid = unsafe { mem::zeroed() };
        unsafe {
            try_call!(raw::git_reference_name_to_id(&mut ret, self.raw(),
                                                    name.to_c_str()));
            Ok(Oid::from_raw(&ret))
        }
    }

    /// Create a new action signature with default user and now timestamp.
    ///
    /// This looks up the user.name and user.email from the configuration and
    /// uses the current time as the timestamp, and creates a new signature
    /// based on that information. It will return `NotFound` if either the
    /// user.name or user.email are not set.
    pub fn signature(&self) -> Result<Signature<'static>, Error> {
        let mut ret = 0 as *mut raw::git_signature;
        unsafe {
            try_call!(raw::git_signature_default(&mut ret, self.raw()));
            Ok(Signature::from_raw(ret))
        }
    }

    /// Set up a new git submodule for checkout.
    ///
    /// This does "git submodule add" up to the fetch and checkout of the
    /// submodule contents. It preps a new submodule, creates an entry in
    /// `.gitmodules` and creates an empty initialized repository either at the
    /// given path in the working directory or in `.git/modules` with a gitlink
    /// from the working directory to the new repo.
    ///
    /// To fully emulate "git submodule add" call this function, then `open()`
    /// the submodule repo and perform the clone step as needed. Lastly, call
    /// `finalize()` to wrap up adding the new submodule and `.gitmodules` to
    /// the index to be ready to commit.
    pub fn submodule(&self, url: &str, path: &Path,
                     use_gitlink: bool) -> Result<Submodule, Error> {
        let mut raw = 0 as *mut raw::git_submodule;
        unsafe {
            try_call!(raw::git_submodule_add_setup(&mut raw, self.raw(),
                                                   url.to_c_str(),
                                                   path.to_c_str(),
                                                   use_gitlink));
            Ok(Submodule::from_raw(self, raw))
        }
    }

    /// Lookup submodule information by name or path.
    ///
    /// Given either the submodule name or path (they are usually the same),
    /// this returns a structure describing the submodule.
    pub fn find_submodule(&self, name: &str) -> Result<Submodule, Error> {
        let mut raw = 0 as *mut raw::git_submodule;
        unsafe {
            try_call!(raw::git_submodule_lookup(&mut raw, self.raw(),
                                                name.to_c_str()));
            Ok(Submodule::from_raw(self, raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_tree(&self, oid: Oid) -> Result<Tree, Error> {
        let mut raw = 0 as *mut raw::git_tree;
        unsafe {
            try_call!(raw::git_tree_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Tree::from_raw(self, raw))
        }
    }

    /// Create a new tag in the repository from an object
    ///
    /// A new reference will also be created pointing to this tag object. If
    /// `force` is true and a reference already exists with the given name, it'll
    /// be replaced.
    ///
    /// The message will not be cleaned up.
    ///
    /// The tag name will be checked for validity. You must avoid the characters
    /// '~', '^', ':', ' \ ', '?', '[', and '*', and the sequences ".." and " @
    /// {" which have special meaning to revparse.
    pub fn tag<'a>(&'a self, name: &str, target: &Object<'a>,
                   tagger: &Signature, message: &str,
                   force: bool) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_tag_create(&mut raw, self.raw, name.to_c_str(),
                                          &*target.raw(), &*tagger.raw(),
                                          message.to_c_str(), force));
            Ok(Oid::from_raw(&raw))
        }
    }

    /// Lookup a tag object from the repository.
    pub fn find_tag(&self, id: Oid) -> Result<Tag, Error> {
        let mut raw = 0 as *mut raw::git_tag;
        unsafe {
            try_call!(raw::git_tag_lookup(&mut raw, self.raw, id.raw()));
            Ok(Tag::from_raw(self, raw))
        }
    }

    /// Delete an existing tag reference.
    ///
    /// The tag name will be checked for validity, see `tag` for some rules
    /// about valid names.
    pub fn tag_delete(&self, name: &str) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_tag_delete(self.raw, name.to_c_str()));
            Ok(())
        }
    }

    /// Get a list with all the tags in the repository.
    ///
    /// An optional fnmatch pattern can also be specified.
    pub fn tag_names(&self, pattern: Option<&str>) -> Result<StringArray, Error> {
        let mut arr = raw::git_strarray {
            strings: 0 as *mut *mut c_char,
            count: 0,
        };
        unsafe {
            match pattern {
                Some(s) => {
                    try_call!(raw::git_tag_list_match(&mut arr, s.to_c_str(),
                                                      self.raw));
                }
                None => { try_call!(raw::git_tag_list(&mut arr, self.raw)); }
            }
        }
        Ok(unsafe { StringArray::from_raw(arr) })
    }

    /// Updates files in the index and the working tree to match the content of
    /// the commit pointed at by HEAD.
    pub fn checkout_head(&self, opts: Option<&CheckoutBuilder>)
                         -> Result<(), Error> {
        unsafe {
            let mut raw_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(&mut raw_opts,
                                raw::GIT_CHECKOUT_OPTIONS_VERSION));
            match opts {
                Some(c) => c.configure(&mut raw_opts),
                None => {}
            }

            try_call!(raw::git_checkout_head(self.raw, &raw_opts));
        }
        Ok(())
    }

    /// Updates files in the working tree to match the content of the index.
    ///
    /// If the index is `None`, the repository's index will be used.
    pub fn checkout_index(&self,
                          index: Option<&mut Index>,
                          opts: Option<&CheckoutBuilder>) -> Result<(), Error> {
        unsafe {
            let mut raw_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(&mut raw_opts,
                                raw::GIT_CHECKOUT_OPTIONS_VERSION));
            match opts {
                Some(c) => c.configure(&mut raw_opts),
                None => {}
            }

            try_call!(raw::git_checkout_index(self.raw,
                                              index.map(|i| &mut *i.raw()),
                                              &raw_opts));
        }
        Ok(())
    }

    /// Updates files in the index and working tree to match the content of the
    /// tree pointed at by the treeish.
    pub fn checkout_tree(&self,
                         treeish: &Object,
                         opts: Option<&CheckoutBuilder>) -> Result<(), Error> {
        unsafe {
            let mut raw_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(&mut raw_opts,
                                raw::GIT_CHECKOUT_OPTIONS_VERSION));
            match opts {
                Some(c) => c.configure(&mut raw_opts),
                None => {}
            }

            try_call!(raw::git_checkout_tree(self.raw, &*treeish.raw(),
            &raw_opts));
        }
        Ok(())
    }
}

#[unsafe_destructor]
impl Drop for Repository {
    fn drop(&mut self) {
        unsafe { raw::git_repository_free(self.raw) }
    }
}

impl RepositoryInitOptions {
    /// Creates a default set of initialization options.
    ///
    /// By default this will set flags for creating all necessary directories
    /// and initializing a directory from the user-configured templates path.
    pub fn new() -> RepositoryInitOptions {
        RepositoryInitOptions {
            flags: raw::GIT_REPOSITORY_INIT_MKDIR as u32 |
                   raw::GIT_REPOSITORY_INIT_MKPATH as u32 |
                   raw::GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE as u32,
            mode: 0,
            workdir_path: None,
            description: None,
            template_path: None,
            initial_head: None,
            origin_url: None,
        }
    }

    /// Create a bare repository with no working directory.
    ///
    /// Defaults to false.
    pub fn bare(self, bare: bool) -> RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_BARE, bare)
    }

    /// Return an error if the repository path appears to already be a git
    /// repository.
    ///
    /// Defaults to false.
    pub fn no_reinit(self, enabled: bool) -> RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_NO_REINIT, enabled)
    }

    /// Normally a '/.git/' will be appended to the repo apth for non-bare repos
    /// (if it is not already there), but passing this flag prevents that
    /// behavior.
    ///
    /// Defaults to false.
    pub fn no_dotgit_dir(self, enabled: bool) -> RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_NO_DOTGIT_DIR, enabled)
    }

    /// Make the repo path (and workdir path) as needed. The ".git" directory
    /// will always be created regardless of this flag.
    ///
    /// Defaults to true.
    pub fn mkdir(self, enabled: bool) -> RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_MKDIR, enabled)
    }

    /// Recursively make all components of the repo and workdir path sas
    /// necessary.
    ///
    /// Defaults to true.
    pub fn mkpath(self, enabled: bool) -> RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_MKPATH, enabled)
    }

    /// Enable or disable using external templates.
    ///
    /// If enabled, then the `template_path` option will be queried first, then
    /// `init.templatedir` from the global config, and finally
    /// `/usr/share/git-core-templates` will be used (if it exists).
    ///
    /// Defaults to true.
    pub fn external_template(self, enabled: bool) -> RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE, enabled)
    }

    fn flag(mut self, flag: raw::git_repository_init_flag_t, on: bool)
            -> RepositoryInitOptions {
        if on {
            self.flags |= flag as u32;
        } else {
            self.flags &= !(flag as u32);
        }
        self
    }

    /// The path do the working directory.
    ///
    /// If this is a relative path it will be evaulated relative to the repo
    /// path. If this is not the "natural" working directory, a .git gitlink
    /// file will be created here linking to the repo path.
    pub fn workdir_path(mut self, path: &Path) -> RepositoryInitOptions {
        self.workdir_path = Some(path.to_c_str());
        self
    }

    /// If set, this will be used to initialize the "description" file in the
    /// repository instead of using the template content.
    pub fn description(mut self, desc: &str) -> RepositoryInitOptions {
        self.description = Some(desc.to_c_str());
        self
    }

    /// When the `external_template` option is set, this is the first location
    /// to check for the template directory.
    ///
    /// If this is not configured, then the default locations will be searched
    /// instead.
    pub fn template_path(mut self, path: &Path) -> RepositoryInitOptions {
        self.template_path = Some(path.to_c_str());
        self
    }

    /// The name of the head to point HEAD at.
    ///
    /// If not configured, this will be treated as `master` and the HEAD ref
    /// will be set to `refs/heads/master`. If this begins with `refs/` it will
    /// be used verbatim; otherwise `refs/heads/` will be prefixed
    pub fn initial_head(mut self, head: &str) -> RepositoryInitOptions {
        self.initial_head = Some(head.to_c_str());
        self
    }

    /// If set, then after the rest of the repository initialization is
    /// completed an `origin` remote will be added pointing to this URL.
    pub fn origin_url(mut self, url: &str) -> RepositoryInitOptions {
        self.origin_url = Some(url.to_c_str());
        self
    }

    /// Creates a set of raw init options to be used with
    /// `git_repository_init_ext`.
    ///
    /// This method is unsafe as the returned value may have pointers to the
    /// interior of this structure.
    pub unsafe fn raw(&self) -> raw::git_repository_init_options {
        let mut opts = mem::zeroed();
        assert_eq!(raw::git_repository_init_init_options(&mut opts,
                                raw::GIT_REPOSITORY_INIT_OPTIONS_VERSION), 0);
        opts.flags = self.flags;
        opts.mode = self.mode;
        let cstr = |a: &Option<CString>| {
            a.as_ref().map(|s| s.as_ptr()).unwrap_or(0 as *const _)
        };
        opts.workdir_path = cstr(&self.workdir_path);
        opts.description = cstr(&self.description);
        opts.template_path = cstr(&self.template_path);
        opts.initial_head = cstr(&self.initial_head);
        opts.origin_url = cstr(&self.origin_url);
        return opts;
    }
}

#[cfg(test)]
mod tests {
    use std::io::TempDir;
    use {Repository, ObjectType, ResetType};

    #[test]
    fn smoke_init() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();
        assert!(!repo.is_bare());
    }

    #[test]
    fn smoke_init_bare() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();

        let repo = Repository::init_bare(path).unwrap();
        assert!(repo.is_bare());
        assert!(repo.namespace().is_none());
    }

    #[test]
    fn smoke_open() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();
        Repository::init(td.path()).unwrap();
        let repo = Repository::open(path).unwrap();
        assert!(!repo.is_bare());
        assert!(!repo.is_shallow());
        assert!(repo.is_empty().unwrap());
        assert!(repo.path() == td.path().join(".git"));
        assert_eq!(repo.state(), ::RepositoryState::Clean);
    }

    #[test]
    fn smoke_open_bare() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();
        Repository::init_bare(td.path()).unwrap();

        let repo = Repository::open(path).unwrap();
        assert!(repo.is_bare());
        assert!(repo.path() == *td.path());
    }

    #[test]
    fn smoke_checkout() {
        let (_td, repo) = ::test::repo_init();
        repo.checkout_head(None).unwrap();
    }

    #[test]
    fn smoke_revparse() {
        let (_td, repo) = ::test::repo_init();
        let rev = repo.revparse("HEAD").unwrap();
        assert!(rev.to().is_none());
        let from = rev.from().unwrap();
        assert!(rev.from().is_some());

        assert_eq!(repo.revparse_single("HEAD").unwrap().id(), from.id());
        let obj = repo.find_object(from.id(), None).unwrap().clone();
        obj.peel(ObjectType::Any).unwrap();
        obj.short_id().unwrap();
        let sig = repo.signature().unwrap();
        repo.reset(&obj, ResetType::Hard, None, None).unwrap();
        repo.reset(&obj, ResetType::Soft, Some(&sig), Some("foo")).unwrap();
    }

    #[test]
    fn makes_dirs() {
        let td = TempDir::new("foo").unwrap();
        Repository::init(&td.path().join("a/b/c/d")).unwrap();
    }

    #[test]
    fn smoke_discover() {
        let td = TempDir::new("test").unwrap();
        let subdir = TempDir::new_in(td.path(), "subdir").unwrap();
        Repository::init_bare(td.path()).unwrap();
        let repo = Repository::discover(subdir.path()).unwrap();
        assert!(repo.path() == *td.path());
    }
}
