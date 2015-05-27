use std::ffi::{CStr, CString};
use std::iter::IntoIterator;
use std::mem;
use std::path::Path;
use std::str;
use libc::{c_int, c_char, size_t, c_void, c_uint};

use {raw, Revspec, Error, init, Object, RepositoryState, Remote, Buf};
use {ResetType, Signature, Reference, References, Submodule, Blame, BlameOptions};
use {Branches, BranchType, Index, Config, Oid, Blob, Branch, Commit, Tree};
use {ObjectType, Tag, Note, Notes, StatusOptions, Statuses, Status, Revwalk};
use {RevparseMode, RepositoryInitMode, Reflog, IntoCString};
use build::{RepoBuilder, CheckoutBuilder};
use string_array::StringArray;
use util::{self, Binding};

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
}

// It is the current belief that a `Repository` can be sent among threads, or
// even shared among threads in a mutex.
unsafe impl Send for Repository {}

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
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Repository, Error> {
        init();
        let path = try!(path.as_ref().into_c_string());
        let mut ret = 0 as *mut raw::git_repository;
        unsafe {
            try_call!(raw::git_repository_open(&mut ret, path));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Attempt to open an already-existing repository at or above `path`
    ///
    /// This starts at `path` and looks up the filesystem hierarchy
    /// until it finds a repository.
    pub fn discover<P: AsRef<Path>>(path: P) -> Result<Repository, Error> {
        // TODO: this diverges significantly from the libgit2 API
        init();
        let buf = Buf::new();
        let path = try!(path.as_ref().into_c_string());
        unsafe {
            try_call!(raw::git_repository_discover(buf.raw(), path, 1,
                                                   0 as *const _));
        }
        Repository::open(util::bytes2path(&*buf))
    }

    /// Creates a new repository in the specified folder.
    ///
    /// This by default will create any necessary directories to create the
    /// repository, and it will read any user-specified templates when creating
    /// the repository. This behavior can be configured through `init_opts`.
    pub fn init<P: AsRef<Path>>(path: P) -> Result<Repository, Error> {
        Repository::init_opts(path, &RepositoryInitOptions::new())
    }

    /// Creates a new `--bare` repository in the specified folder.
    ///
    /// The folder must exist prior to invoking this function.
    pub fn init_bare<P: AsRef<Path>>(path: P) -> Result<Repository, Error> {
        Repository::init_opts(path, RepositoryInitOptions::new().bare(true))
    }

    /// Creates a new `--bare` repository in the specified folder.
    ///
    /// The folder must exist prior to invoking this function.
    pub fn init_opts<P: AsRef<Path>>(path: P, opts: &RepositoryInitOptions)
                     -> Result<Repository, Error> {
        init();
        let path = try!(path.as_ref().into_c_string());
        let mut ret = 0 as *mut raw::git_repository;
        unsafe {
            let mut opts = opts.raw();
            try_call!(raw::git_repository_init_ext(&mut ret, path, &mut opts));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Clone a remote repository.
    ///
    /// See the `RepoBuilder` struct for more information. This function will
    /// delegate to a fresh `RepoBuilder`
    pub fn clone<P: AsRef<Path>>(url: &str, into: P)
                                 -> Result<Repository, Error> {
        ::init();
        RepoBuilder::new().clone(url, into.as_ref())
    }

    /// Execute a rev-parse operation against the `spec` listed.
    ///
    /// The resulting revision specification is returned, or an error is
    /// returned if one occurs.
    pub fn revparse(&self, spec: &str) -> Result<Revspec, Error> {
        let mut raw = raw::git_revspec {
            from: 0 as *mut _,
            to: 0 as *mut _,
            flags: 0,
        };
        let spec = try!(CString::new(spec));
        unsafe {
            try_call!(raw::git_revparse(&mut raw, self.raw, spec));
            let to = Binding::from_raw_opt(raw.to);
            let from = Binding::from_raw_opt(raw.from);
            let mode = RevparseMode::from_bits_truncate(raw.flags as u32);
            Ok(Revspec::from_objects(from, to, mode))
        }
    }

    /// Find a single object, as specified by a revision string.
    pub fn revparse_single(&self, spec: &str) -> Result<Object, Error> {
        let spec = try!(CString::new(spec));
        let mut obj = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_revparse_single(&mut obj, self.raw, spec));
            assert!(!obj.is_null());
            Ok(Binding::from_raw(obj))
        }
    }

    /// Find a single object and intermediate reference by a revision string.
    ///
    /// See `man gitrevisions`, or
    /// http://git-scm.com/docs/git-rev-parse.html#_specifying_revisions for
    /// information on the syntax accepted.
    ///
    /// In some cases (`@{<-n>}` or `<branchname>@{upstream}`), the expression
    /// may point to an intermediate reference. When such expressions are being
    /// passed in, this intermediate reference is returned.
    pub fn revparse_ext(&self, spec: &str)
                        -> Result<(Object, Option<Reference>), Error> {
        let spec = try!(CString::new(spec));
        let mut git_obj = 0 as *mut raw::git_object;
        let mut git_ref = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_revparse_ext(&mut git_obj, &mut git_ref,
                                            self.raw, spec));
            assert!(!git_obj.is_null());
            Ok((Binding::from_raw(git_obj), Binding::from_raw_opt(git_ref)))
        }
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
    pub fn path(&self) -> &Path {
        unsafe {
            let ptr = raw::git_repository_path(self.raw);
            util::bytes2path(::opt_bytes(self, ptr).unwrap())
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
        ) );

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
    pub fn workdir(&self) -> Option<&Path> {
        unsafe {
            let ptr = raw::git_repository_workdir(self.raw);
            if ptr.is_null() {
                None
            } else {
                Some(util::bytes2path(CStr::from_ptr(ptr).to_bytes()))
            }
        }
    }

    /// Get the currently active namespace for this repository.
    ///
    /// If there is no namespace, or the namespace is not a valid utf8 string,
    /// `None` is returned.
    pub fn namespace(&self) -> Option<&str> {
        self.namespace_bytes().and_then(|s| str::from_utf8(s).ok())
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
            Ok(Binding::from_raw(arr))
        }
    }

    /// Get the information for a particular remote
    pub fn find_remote(&self, name: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_remote_lookup(&mut ret, self.raw, name));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Add a remote with the default fetch refspec to the repository's
    /// configuration.
    pub fn remote(&self, name: &str, url: &str) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        let name = try!(CString::new(name));
        let url = try!(CString::new(url));
        unsafe {
            try_call!(raw::git_remote_create(&mut ret, self.raw, name, url));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create an anonymous remote
    ///
    /// Create a remote with the given url and refspec in memory. You can use
    /// this when you have a URL instead of a remote's name. Note that anonymous
    /// remotes cannot be converted to persisted remotes.
    pub fn remote_anonymous(&self,
                            url: &str,
                            fetch: Option<&str>) -> Result<Remote, Error> {
        let mut ret = 0 as *mut raw::git_remote;
        let url = try!(CString::new(url));
        let fetch = match fetch {
            Some(t) => Some(try!(CString::new(t))),
            None => None,
        };
        unsafe {
            try_call!(raw::git_remote_create_anonymous(&mut ret, self.raw, url,
                                                       fetch));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Give a remote a new name
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// are updated.
    ///
    /// A temporary in-memory remote cannot be given a name with this method.
    ///
    /// No loaded instances of the remote with the old name will change their
    /// name or their list of refspecs.
    ///
    /// The returned array of strings is a list of the non-default refspecs
    /// which cannot be renamed and are returned for further processing by the
    /// caller.
    pub fn remote_rename(&self, name: &str,
                         new_name: &str) -> Result<StringArray, Error> {
        let name = try!(CString::new(name));
        let new_name = try!(CString::new(new_name));
        let mut problems = raw::git_strarray {
            count: 0,
            strings: 0 as *mut *mut c_char,
        };
        unsafe {
            try_call!(raw::git_remote_rename(&mut problems, self.raw, name,
                                             new_name));
            Ok(Binding::from_raw(problems))
        }
    }

    /// Delete an existing persisted remote.
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// will be removed.
    pub fn remote_delete(&self, name: &str) -> Result<(), Error> {
        let name = try!(CString::new(name));
        unsafe { try_call!(raw::git_remote_delete(self.raw, name)); }
        Ok(())
    }

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
    ///
    /// The `target` is a commit-ish to which the head should be moved to. The
    /// object can either be a commit or a tag, but tags must be derefernceable
    /// to a commit.
    ///
    /// The `checkout` options will only be used for a hard reset.
    pub fn reset(&self,
                 target: &Object,
                 kind: ResetType,
                 checkout: Option<&mut CheckoutBuilder>)
                 -> Result<(), Error> {
        unsafe {
            let mut opts: raw::git_checkout_options = mem::zeroed();
            let opts = checkout.map(|c| {
                c.configure(&mut opts); &mut opts
            });
            try_call!(raw::git_reset(self.raw, target.raw(), kind, opts));
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
    pub fn reset_default<T, I>(&self,
                               target: Option<&Object>,
                               paths: I) -> Result<(), Error>
        where T: IntoCString, I: IntoIterator<Item=T>,
    {
        let (_a, _b, mut arr) = try!(::util::iter2cstrs(paths));
        let target = target.map(|t| t.raw());
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
            Ok(Binding::from_raw(ret))
        }
    }

    /// Make the repository HEAD point to the specified reference.
    ///
    /// If the provided reference points to a tree or a blob, the HEAD is
    /// unaltered and an error is returned.
    ///
    /// If the provided reference points to a branch, the HEAD will point to
    /// that branch, staying attached, or become attached if it isn't yet. If
    /// the branch doesn't exist yet, no error will be returned. The HEAD will
    /// then be attached to an unborn branch.
    ///
    /// Otherwise, the HEAD will be detached and will directly point to the
    /// commit.
    pub fn set_head(&self, refname: &str) -> Result<(), Error> {
        let refname = try!(CString::new(refname));
        unsafe {
            try_call!(raw::git_repository_set_head(self.raw, refname));
        }
        Ok(())
    }

    /// Make the repository HEAD directly point to the commit.
    ///
    /// If the provided committish cannot be found in the repository, the HEAD
    /// is unaltered and an error is returned.
    ///
    /// If the provided commitish cannot be peeled into a commit, the HEAD is
    /// unaltered and an error is returned.
    ///
    /// Otherwise, the HEAD will eventually be detached and will directly point
    /// to the peeled commit.
    pub fn set_head_detached(&self, commitish: Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_set_head_detached(self.raw,
                                                            commitish.raw()));
        }
        Ok(())
    }

    /// Create an iterator for the repo's references
    pub fn references(&self) -> Result<References, Error> {
        let mut ret = 0 as *mut raw::git_reference_iterator;
        unsafe {
            try_call!(raw::git_reference_iterator_new(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create an iterator for the repo's references that match the specified
    /// glob
    pub fn references_glob(&self, glob: &str) -> Result<References, Error> {
        let mut ret = 0 as *mut raw::git_reference_iterator;
        let glob = try!(CString::new(glob));
        unsafe {
            try_call!(raw::git_reference_iterator_glob_new(&mut ret, self.raw,
                                                           glob));

            Ok(Binding::from_raw(ret))
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
                data.ret.push(Binding::from_raw(raw));
            }
            0
        }
    }

    /// Gather file status information and populate the returned structure.
    ///
    /// Note that if a pathspec is given in the options to filter the
    /// status, then the results from rename detection (if you enable it) may
    /// not be accurate. To do rename detection properly, this must be called
    /// with no pathspec so that all files can be considered.
    pub fn statuses(&self, options: Option<&mut StatusOptions>)
                    -> Result<Statuses, Error> {
        let mut ret = 0 as *mut raw::git_status_list;
        unsafe {
            try_call!(raw::git_status_list_new(&mut ret, self.raw,
                                               options.map(|s| s.raw())
                                                      .unwrap_or(0 as *const _)));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Test if the ignore rules apply to a given file.
    ///
    /// This function checks the ignore rules to see if they would apply to the
    /// given file. This indicates if the file would be ignored regardless of
    /// whether the file is already in the index or committed to the repository.
    ///
    /// One way to think of this is if you were to do "git add ." on the
    /// directory containing the file, would it be added or not?
    pub fn status_should_ignore(&self, path: &Path) -> Result<bool, Error> {
        let mut ret = 0 as c_int;
        let path = try!(path.into_c_string());
        unsafe {
            try_call!(raw::git_status_should_ignore(&mut ret, self.raw,
                                                    path));
        }
        Ok(ret != 0)
    }

    /// Get file status for a single file.
    ///
    /// This tries to get status for the filename that you give. If no files
    /// match that name (in either the HEAD, index, or working directory), this
    /// returns NotFound.
    ///
    /// If the name matches multiple files (for example, if the path names a
    /// directory or if running on a case- insensitive filesystem and yet the
    /// HEAD has two entries that both match the path), then this returns
    /// Ambiguous because it cannot give correct results.
    ///
    /// This does not do any sort of rename detection. Renames require a set of
    /// targets and because of the path filtering, there is not enough
    /// information to check renames correctly. To check file status with rename
    /// detection, there is no choice but to do a full `statuses` and scan
    /// through looking for the path that you are interested in.
    pub fn status_file(&self, path: &Path) -> Result<Status, Error> {
        let mut ret = 0 as c_uint;
        let path = try!(path.into_c_string());
        unsafe {
            try_call!(raw::git_status_file(&mut ret, self.raw,
                                           path));
        }
        Ok(Status::from_bits_truncate(ret as u32))
    }

    /// Create an iterator which loops over the requested branches.
    pub fn branches(&self, filter: Option<BranchType>)
                    -> Result<Branches, Error> {
        let mut raw = 0 as *mut raw::git_branch_iterator;
        unsafe {
            try_call!(raw::git_branch_iterator_new(&mut raw, self.raw(), filter));
            Ok(Branches::from_raw(raw))
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
            Ok(Binding::from_raw(raw))
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
            Ok(Binding::from_raw(raw))
        }
    }

    /// Write an in-memory buffer to the ODB as a blob.
    ///
    /// The Oid returned can in turn be passed to `find_blob` to get a handle to
    /// the blob.
    pub fn blob(&self, data: &[u8]) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            let ptr = data.as_ptr() as *const c_void;
            let len = data.len() as size_t;
            try_call!(raw::git_blob_create_frombuffer(&mut raw, self.raw(),
                                                      ptr, len));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Read a file from the filesystem and write its content to the Object
    /// Database as a loose blob
    ///
    /// The Oid returned can in turn be passed to `find_blob` to get a handle to
    /// the blob.
    pub fn blob_path(&self, path: &Path) -> Result<Oid, Error> {
        let path = try!(path.into_c_string());
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_blob_create_fromdisk(&mut raw, self.raw(),
                                                    path));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_blob(&self, oid: Oid) -> Result<Blob, Error> {
        let mut raw = 0 as *mut raw::git_blob;
        unsafe {
            try_call!(raw::git_blob_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new branch pointing at a target commit
    ///
    /// A new direct reference will be created pointing to this target commit.
    /// If `force` is true and a reference already exists with the given name,
    /// it'll be replaced.
    pub fn branch(&self,
                  branch_name: &str,
                  target: &Commit,
                  force: bool) -> Result<Branch, Error> {
        let branch_name = try!(CString::new(branch_name));
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_branch_create(&mut raw,
                                             self.raw(),
                                             branch_name,
                                             target.raw(),
                                             force));
            Ok(Branch::wrap(Binding::from_raw(raw)))
        }
    }

    /// Lookup a branch by its name in a repository.
    pub fn find_branch(&self, name: &str, branch_type: BranchType)
                       -> Result<Branch, Error> {
        let name = try!(CString::new(name));
        let mut ret = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_branch_lookup(&mut ret, self.raw(), name,
                                             branch_type));
            Ok(Branch::wrap(Binding::from_raw(ret)))
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
    pub fn commit(&self,
                  update_ref: Option<&str>,
                  author: &Signature,
                  committer: &Signature,
                  message: &str,
                  tree: &Tree,
                  parents: &[&Commit]) -> Result<Oid, Error> {
        let update_ref = try!(::opt_cstr(update_ref));
        let parent_ptrs: Vec<*const raw::git_commit> =  parents.iter().map(|p| {
            p.raw() as *const raw::git_commit
        }).collect();
        let message = try!(CString::new(message));
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_commit_create(&mut raw,
                                             self.raw(),
                                             update_ref,
                                             author.raw(),
                                             committer.raw(),
                                             0 as *const c_char,
                                             message,
                                             tree.raw(),
                                             parents.len() as size_t,
                                             parent_ptrs.as_ptr()));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }


    /// Lookup a reference to one of the commits in a repository.
    pub fn find_commit(&self, oid: Oid) -> Result<Commit, Error> {
        let mut raw = 0 as *mut raw::git_commit;
        unsafe {
            try_call!(raw::git_commit_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_object(&self, oid: Oid,
                       kind: Option<ObjectType>) -> Result<Object, Error> {
        let mut raw = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_object_lookup(&mut raw, self.raw(), oid.raw(),
                                             kind));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new direct reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn reference(&self, name: &str, id: Oid, force: bool,
                     log_message: &str) -> Result<Reference, Error> {
        let name = try!(CString::new(name));
        let log_message = try!(CString::new(log_message));
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_reference_create(&mut raw, self.raw(), name,
                                                id.raw(), force,
                                                log_message));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new symbolic reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn reference_symbolic(&self, name: &str, target: &str,
                              force: bool,
                              log_message: &str)
                              -> Result<Reference, Error> {
        let name = try!(CString::new(name));
        let target = try!(CString::new(target));
        let log_message = try!(CString::new(log_message));
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_reference_symbolic_create(&mut raw, self.raw(),
                                                         name, target, force,
                                                         log_message));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_reference(&self, name: &str) -> Result<Reference, Error> {
        let name = try!(CString::new(name));
        let mut raw = 0 as *mut raw::git_reference;
        unsafe {
            try_call!(raw::git_reference_lookup(&mut raw, self.raw(), name));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference by name and resolve immediately to OID.
    ///
    /// This function provides a quick way to resolve a reference name straight
    /// through to the object id that it refers to. This avoids having to
    /// allocate or free any `Reference` objects for simple situations.
    pub fn refname_to_id(&self, name: &str) -> Result<Oid, Error> {
        let name = try!(CString::new(name));
        let mut ret = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_reference_name_to_id(&mut ret, self.raw(), name));
            Ok(Binding::from_raw(&ret as *const _))
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
            Ok(Binding::from_raw(ret))
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
        let url = try!(CString::new(url));
        let path = try!(path.into_c_string());
        let mut raw = 0 as *mut raw::git_submodule;
        unsafe {
            try_call!(raw::git_submodule_add_setup(&mut raw, self.raw(),
                                                   url, path, use_gitlink));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup submodule information by name or path.
    ///
    /// Given either the submodule name or path (they are usually the same),
    /// this returns a structure describing the submodule.
    pub fn find_submodule(&self, name: &str) -> Result<Submodule, Error> {
        let name = try!(CString::new(name));
        let mut raw = 0 as *mut raw::git_submodule;
        unsafe {
            try_call!(raw::git_submodule_lookup(&mut raw, self.raw(), name));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_tree(&self, oid: Oid) -> Result<Tree, Error> {
        let mut raw = 0 as *mut raw::git_tree;
        unsafe {
            try_call!(raw::git_tree_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new tag in the repository from an object
    ///
    /// A new reference will also be created pointing to this tag object. If
    /// `force` is true and a reference already exists with the given name,
    /// it'll be replaced.
    ///
    /// The message will not be cleaned up.
    ///
    /// The tag name will be checked for validity. You must avoid the characters
    /// '~', '^', ':', ' \ ', '?', '[', and '*', and the sequences ".." and " @
    /// {" which have special meaning to revparse.
    pub fn tag(&self, name: &str, target: &Object,
               tagger: &Signature, message: &str,
               force: bool) -> Result<Oid, Error> {
        let name = try!(CString::new(name));
        let message = try!(CString::new(message));
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_tag_create(&mut raw, self.raw, name,
                                          target.raw(), tagger.raw(),
                                          message, force));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Create a new lightweight tag pointing at a target object
    ///
    /// A new direct reference will be created pointing to this target object.
    /// If force is true and a reference already exists with the given name,
    /// it'll be replaced.
    pub fn tag_lightweight(&self,
                           name: &str,
                           target: &Object,
                           force: bool) -> Result<Oid, Error> {
        let name = try!(CString::new(name));
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_tag_create_lightweight(&mut raw, self.raw, name,
                                                      target.raw(), force));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Lookup a tag object from the repository.
    pub fn find_tag(&self, id: Oid) -> Result<Tag, Error> {
        let mut raw = 0 as *mut raw::git_tag;
        unsafe {
            try_call!(raw::git_tag_lookup(&mut raw, self.raw, id.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Delete an existing tag reference.
    ///
    /// The tag name will be checked for validity, see `tag` for some rules
    /// about valid names.
    pub fn tag_delete(&self, name: &str) -> Result<(), Error> {
        let name = try!(CString::new(name));
        unsafe {
            try_call!(raw::git_tag_delete(self.raw, name));
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
                    let s = try!(CString::new(s));
                    try_call!(raw::git_tag_list_match(&mut arr, s, self.raw));
                }
                None => { try_call!(raw::git_tag_list(&mut arr, self.raw)); }
            }
            Ok(Binding::from_raw(arr))
        }
    }

    /// Updates files in the index and the working tree to match the content of
    /// the commit pointed at by HEAD.
    pub fn checkout_head(&self, opts: Option<&mut CheckoutBuilder>)
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
                          opts: Option<&mut CheckoutBuilder>) -> Result<(), Error> {
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
                         opts: Option<&mut CheckoutBuilder>) -> Result<(), Error> {
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

    /// Add a note for an object
    ///
    /// The `notes_ref` argument is the canonical name of the reference to use,
    /// defaulting to "refs/notes/commits". If `force` is specified then
    /// previous notes are overwritten.
    pub fn note(&self,
                author: &Signature,
                committer: &Signature,
                notes_ref: Option<&str>,
                oid: Oid,
                note: &str,
                force: bool) -> Result<Oid, Error> {
        let notes_ref = try!(::opt_cstr(notes_ref));
        let note = try!(CString::new(note));
        let mut ret = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_note_create(&mut ret,
                                           self.raw,
                                           notes_ref,
                                           author.raw(),
                                           committer.raw(),
                                           oid.raw(),
                                           note,
                                           force));
            Ok(Binding::from_raw(&ret as *const _))
        }
    }

    /// Get the default notes reference for this repository
    pub fn note_default_ref(&self) -> Result<String, Error> {
        let ret = Buf::new();
        unsafe {
            try_call!(raw::git_note_default_ref(ret.raw(), self.raw));
        }
        Ok(str::from_utf8(&ret).unwrap().to_string())
    }

    /// Creates a new iterator for notes in this repository.
    ///
    /// The `notes_ref` argument is the canonical name of the reference to use,
    /// defaulting to "refs/notes/commits".
    ///
    /// The iterator returned yields pairs of (Oid, Oid) where the first element
    /// is the id of the note and the second id is the id the note is
    /// annotating.
    pub fn notes(&self, notes_ref: Option<&str>) -> Result<Notes, Error> {
        let notes_ref = try!(::opt_cstr(notes_ref));
        let mut ret = 0 as *mut raw::git_note_iterator;
        unsafe {
            try_call!(raw::git_note_iterator_new(&mut ret, self.raw, notes_ref));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Read the note for an object.
    ///
    /// The `notes_ref` argument is the canonical name of the reference to use,
    /// defaulting to "refs/notes/commits".
    ///
    /// The id specified is the Oid of the git object to read the note from.
    pub fn find_note(&self, notes_ref: Option<&str>, id: Oid)
                     -> Result<Note, Error> {
        let notes_ref = try!(::opt_cstr(notes_ref));
        let mut ret = 0 as *mut raw::git_note;
        unsafe {
            try_call!(raw::git_note_read(&mut ret, self.raw, notes_ref,
                                         id.raw()));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Remove the note for an object.
    ///
    /// The `notes_ref` argument is the canonical name of the reference to use,
    /// defaulting to "refs/notes/commits".
    ///
    /// The id specified is the Oid of the git object to remove the note from.
    pub fn note_delete(&self,
                       id: Oid,
                       notes_ref: Option<&str>,
                       author: &Signature,
                       committer: &Signature) -> Result<(), Error> {
        let notes_ref = try!(::opt_cstr(notes_ref));
        unsafe {
            try_call!(raw::git_note_remove(self.raw, notes_ref, author.raw(),
                                           committer.raw(), id.raw()));
            Ok(())
        }
    }

    /// Create a revwalk that can be used to traverse the commit graph.
    pub fn revwalk(&self) -> Result<Revwalk, Error> {
        let mut raw = 0 as *mut raw::git_revwalk;
        unsafe {
            try_call!(raw::git_revwalk_new(&mut raw, self.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the blame for a single file.
    pub fn blame_file(&self, path: &Path, opts: Option<&mut BlameOptions>)
                      -> Result<Blame, Error> {
        let path = try!(path.into_c_string());
        let mut raw = 0 as *mut raw::git_blame;

        unsafe {
            try_call!(raw::git_blame_file(&mut raw,
                                          self.raw(),
                                          path,
                                          opts.map(|s| s.raw())));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Find a merge base between two commits
    pub fn merge_base(&self, one: Oid, two: Oid) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_merge_base(&mut raw, self.raw,
                                          one.raw(), two.raw()));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Count the number of unique commits between two commit objects
    ///
    /// There is no need for branches containing the commits to have any
    /// upstream relationship, but it helps to think of one as a branch and the
    /// other as its upstream, the ahead and behind values will be what git
    /// would report for the branches.
    pub fn graph_ahead_behind(&self, local: Oid, upstream: Oid)
                              -> Result<(usize, usize), Error> {
        unsafe {
            let mut ahead: size_t = 0;
            let mut behind: size_t = 0;
            try_call!(raw::git_graph_ahead_behind(&mut ahead, &mut behind,
                                                  self.raw(), local.raw(),
                                                  upstream.raw()));
            Ok((ahead as usize, behind as usize))
        }
    }

    /// Determine if a commit is the descendant of another commit
    pub fn graph_descendant_of(&self, commit: Oid, ancestor: Oid)
                               -> Result<bool, Error> {
        unsafe {
            let rv = try_call!(raw::git_graph_descendant_of(self.raw(),
                                                            commit.raw(),
                                                            ancestor.raw()));
            Ok(rv != 0)
        }
    }

    /// Read the reflog for the given reference
    ///
    /// If there is no reflog file for the given reference yet, an empty reflog
    /// object will be returned.
    pub fn reflog(&self, name: &str) -> Result<Reflog, Error> {
        let name = try!(CString::new(name));
        let mut ret = 0 as *mut raw::git_reflog;
        unsafe {
            try_call!(raw::git_reflog_read(&mut ret, self.raw, name));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Delete the reflog for the given reference
    pub fn reflog_delete(&self, name: &str) -> Result<(), Error> {
        let name = try!(CString::new(name));
        unsafe { try_call!(raw::git_reflog_delete(self.raw, name)); }
        Ok(())
    }

    /// Rename a reflog
    ///
    /// The reflog to be renamed is expected to already exist.
    pub fn reflog_rename(&self, old_name: &str, new_name: &str)
                         -> Result<(), Error> {
        let old_name = try!(CString::new(old_name));
        let new_name = try!(CString::new(new_name));
        unsafe {
            try_call!(raw::git_reflog_rename(self.raw, old_name, new_name));
        }
        Ok(())
    }
}

impl Binding for Repository {
    type Raw = *mut raw::git_repository;
    unsafe fn from_raw(ptr: *mut raw::git_repository) -> Repository {
        Repository { raw: ptr }
    }
    fn raw(&self) -> *mut raw::git_repository { self.raw }
}

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
    pub fn bare(&mut self, bare: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_BARE, bare)
    }

    /// Return an error if the repository path appears to already be a git
    /// repository.
    ///
    /// Defaults to false.
    pub fn no_reinit(&mut self, enabled: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_NO_REINIT, enabled)
    }

    /// Normally a '/.git/' will be appended to the repo apth for non-bare repos
    /// (if it is not already there), but passing this flag prevents that
    /// behavior.
    ///
    /// Defaults to false.
    pub fn no_dotgit_dir(&mut self, enabled: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_NO_DOTGIT_DIR, enabled)
    }

    /// Make the repo path (and workdir path) as needed. The ".git" directory
    /// will always be created regardless of this flag.
    ///
    /// Defaults to true.
    pub fn mkdir(&mut self, enabled: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_MKDIR, enabled)
    }

    /// Recursively make all components of the repo and workdir path sas
    /// necessary.
    ///
    /// Defaults to true.
    pub fn mkpath(&mut self, enabled: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_MKPATH, enabled)
    }

    /// Set to one of the `RepositoryInit` constants, or a custom value.
    pub fn mode(&mut self, mode: RepositoryInitMode)
                -> &mut RepositoryInitOptions {
        self.mode = mode.bits();
        self
    }

    /// Enable or disable using external templates.
    ///
    /// If enabled, then the `template_path` option will be queried first, then
    /// `init.templatedir` from the global config, and finally
    /// `/usr/share/git-core-templates` will be used (if it exists).
    ///
    /// Defaults to true.
    pub fn external_template(&mut self, enabled: bool)
                             -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE, enabled)
    }

    fn flag(&mut self, flag: raw::git_repository_init_flag_t, on: bool)
            -> &mut RepositoryInitOptions {
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
    pub fn workdir_path(&mut self, path: &Path) -> &mut RepositoryInitOptions {
        self.workdir_path = Some(path.into_c_string().unwrap());
        self
    }

    /// If set, this will be used to initialize the "description" file in the
    /// repository instead of using the template content.
    pub fn description(&mut self, desc: &str) -> &mut RepositoryInitOptions {
        self.description = Some(CString::new(desc).unwrap());
        self
    }

    /// When the `external_template` option is set, this is the first location
    /// to check for the template directory.
    ///
    /// If this is not configured, then the default locations will be searched
    /// instead.
    pub fn template_path(&mut self, path: &Path) -> &mut RepositoryInitOptions {
        self.template_path = Some(path.into_c_string().unwrap());
        self
    }

    /// The name of the head to point HEAD at.
    ///
    /// If not configured, this will be treated as `master` and the HEAD ref
    /// will be set to `refs/heads/master`. If this begins with `refs/` it will
    /// be used verbatim; otherwise `refs/heads/` will be prefixed
    pub fn initial_head(&mut self, head: &str) -> &mut RepositoryInitOptions {
        self.initial_head = Some(CString::new(head).unwrap());
        self
    }

    /// If set, then after the rest of the repository initialization is
    /// completed an `origin` remote will be added pointing to this URL.
    pub fn origin_url(&mut self, url: &str) -> &mut RepositoryInitOptions {
        self.origin_url = Some(CString::new(url).unwrap());
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
        opts.workdir_path = ::call::convert(&self.workdir_path);
        opts.description = ::call::convert(&self.description);
        opts.template_path = ::call::convert(&self.template_path);
        opts.initial_head = ::call::convert(&self.initial_head);
        opts.origin_url = ::call::convert(&self.origin_url);
        return opts;
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use tempdir::TempDir;
    use {Repository, Oid, ObjectType, ResetType};
    use build::CheckoutBuilder;

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
        assert_eq!(::test::realpath(&repo.path()).unwrap(),
                   ::test::realpath(&td.path().join(".git/")).unwrap());
        assert_eq!(repo.state(), ::RepositoryState::Clean);
    }

    #[test]
    fn smoke_open_bare() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();
        Repository::init_bare(td.path()).unwrap();

        let repo = Repository::open(path).unwrap();
        assert!(repo.is_bare());
        assert_eq!(::test::realpath(&repo.path()).unwrap(),
                   ::test::realpath(&td.path().join("")).unwrap());
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
        repo.reset(&obj, ResetType::Hard, None).unwrap();
        let mut opts = CheckoutBuilder::new();
        t!(repo.reset(&obj, ResetType::Soft, Some(&mut opts)));
    }

    #[test]
    fn makes_dirs() {
        let td = TempDir::new("foo").unwrap();
        Repository::init(&td.path().join("a/b/c/d")).unwrap();
    }

    #[test]
    fn smoke_discover() {
        let td = TempDir::new("test").unwrap();
        let subdir = td.path().join("subdi");
        fs::create_dir(&subdir).unwrap();
        Repository::init_bare(td.path()).unwrap();
        let repo = Repository::discover(&subdir).unwrap();
        assert_eq!(::test::realpath(&repo.path()).unwrap(),
                   ::test::realpath(&td.path().join("")).unwrap());
    }

    fn graph_repo_init() -> (TempDir, Repository) {
        let (_td, repo) = ::test::repo_init();
        {
            let head = repo.head().unwrap().target().unwrap();
            let head = repo.find_commit(head).unwrap();

            let mut index = repo.index().unwrap();
            let id = index.write_tree().unwrap();

            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second",
                        &tree, &[&head]).unwrap();
        }
        (_td, repo)
    }

    #[test]
    fn smoke_graph_ahead_behind() {
        let (_td, repo) = graph_repo_init();
        let head = repo.head().unwrap().target().unwrap();
        let head = repo.find_commit(head).unwrap();
        let head_id = head.id();
        let head_parent_id = head.parent(0).unwrap().id();
        let (ahead, behind) = repo.graph_ahead_behind(head_id,
                                                      head_parent_id).unwrap();
        assert_eq!(ahead, 1);
        assert_eq!(behind, 0);
        let (ahead, behind) = repo.graph_ahead_behind(head_parent_id,
                                                      head_id).unwrap();
        assert_eq!(ahead, 0);
        assert_eq!(behind, 1);
    }

    #[test]
    fn smoke_graph_descendant_of() {
        let (_td, repo) = graph_repo_init();
        let head = repo.head().unwrap().target().unwrap();
        let head = repo.find_commit(head).unwrap();
        let head_id = head.id();
        let head_parent_id = head.parent(0).unwrap().id();
        assert!(repo.graph_descendant_of(head_id, head_parent_id).unwrap());
        assert!(!repo.graph_descendant_of(head_parent_id, head_id).unwrap());
    }

    #[test]
    fn smoke_set_head() {
        let (_td, repo) = ::test::repo_init();

        assert!(repo.set_head("refs/heads/does-not-exist").is_ok());
        assert!(repo.head().is_err());

        assert!(repo.set_head("refs/heads/master").is_ok());
        assert!(repo.head().is_ok());

        assert!(repo.set_head("*").is_err());
    }

    #[test]
    fn smoke_set_head_detached() {
        let (_td, repo) = ::test::repo_init();

        let void_oid = Oid::from_bytes(b"00000000000000000000").unwrap();
        assert!(repo.set_head_detached(void_oid).is_err());

        let master_oid = repo.revparse_single("master").unwrap().id();
        assert!(repo.set_head_detached(master_oid).is_ok());
        assert_eq!(repo.head().unwrap().target().unwrap(), master_oid);
    }

    #[test]
    fn smoke_revparse_ext() {
        let (_td, repo) = graph_repo_init();

        {
            let short_refname = "master";
            let expected_refname = "refs/heads/master";
            let (obj, reference) = repo.revparse_ext(short_refname).unwrap();
            let expected_obj = repo.revparse_single(expected_refname).unwrap();
            assert_eq!(obj.id(), expected_obj.id());
            assert_eq!(reference.unwrap().name().unwrap(), expected_refname);
        }
        {
            let missing_refname = "refs/heads/does-not-exist";
            assert!(repo.revparse_ext(missing_refname).is_err());
        }
        {
            let (_obj, reference) = repo.revparse_ext("HEAD^").unwrap();
            assert!(reference.is_none());
        }
    }
}
