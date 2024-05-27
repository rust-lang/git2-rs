use libc::{c_char, c_int, c_uint, c_void, size_t};
use std::env;
use std::ffi::{CStr, CString, OsStr};
use std::mem;
use std::path::{Path, PathBuf};
use std::ptr;
use std::str;

use crate::build::{CheckoutBuilder, RepoBuilder};
use crate::diff::{
    binary_cb_c, file_cb_c, hunk_cb_c, line_cb_c, BinaryCb, DiffCallbacks, FileCb, HunkCb, LineCb,
};
use crate::oid_array::OidArray;
use crate::stash::{stash_cb, StashApplyOptions, StashCbData, StashSaveOptions};
use crate::string_array::StringArray;
use crate::tagforeach::{tag_foreach_cb, TagForeachCB, TagForeachData};
use crate::util::{self, path_to_repo_path, Binding};
use crate::worktree::{Worktree, WorktreeAddOptions};
use crate::CherrypickOptions;
use crate::RevertOptions;
use crate::{mailmap::Mailmap, panic};
use crate::{
    raw, AttrCheckFlags, Buf, Error, Object, Remote, RepositoryOpenFlags, RepositoryState, Revspec,
    StashFlags,
};
use crate::{
    AnnotatedCommit, MergeAnalysis, MergeOptions, MergePreference, SubmoduleIgnore,
    SubmoduleStatus, SubmoduleUpdate,
};
use crate::{ApplyLocation, ApplyOptions, Rebase, RebaseOptions};
use crate::{Blame, BlameOptions, Reference, References, ResetType, Signature, Submodule};
use crate::{Blob, BlobWriter, Branch, BranchType, Branches, Commit, Config, Index, Oid, Tree};
use crate::{Describe, IntoCString, Reflog, RepositoryInitMode, RevparseMode};
use crate::{DescribeOptions, Diff, DiffOptions, Odb, PackBuilder, TreeBuilder};
use crate::{Note, Notes, ObjectType, Revwalk, Status, StatusOptions, Statuses, Tag, Transaction};

type MergeheadForeachCb<'a> = dyn FnMut(&Oid) -> bool + 'a;
type FetchheadForeachCb<'a> = dyn FnMut(&str, &[u8], &Oid, bool) -> bool + 'a;

struct FetchheadForeachCbData<'a> {
    callback: &'a mut FetchheadForeachCb<'a>,
}

struct MergeheadForeachCbData<'a> {
    callback: &'a mut MergeheadForeachCb<'a>,
}

extern "C" fn mergehead_foreach_cb(oid: *const raw::git_oid, payload: *mut c_void) -> c_int {
    panic::wrap(|| unsafe {
        let data = &mut *(payload as *mut MergeheadForeachCbData<'_>);
        let res = {
            let callback = &mut data.callback;
            callback(&Binding::from_raw(oid))
        };

        if res {
            0
        } else {
            1
        }
    })
    .unwrap_or(1)
}

extern "C" fn fetchhead_foreach_cb(
    ref_name: *const c_char,
    remote_url: *const c_char,
    oid: *const raw::git_oid,
    is_merge: c_uint,
    payload: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let data = &mut *(payload as *mut FetchheadForeachCbData<'_>);
        let res = {
            let callback = &mut data.callback;

            assert!(!ref_name.is_null());
            assert!(!remote_url.is_null());
            assert!(!oid.is_null());

            let ref_name = str::from_utf8(CStr::from_ptr(ref_name).to_bytes()).unwrap();
            let remote_url = CStr::from_ptr(remote_url).to_bytes();
            let oid = Binding::from_raw(oid);
            let is_merge = is_merge == 1;

            callback(&ref_name, remote_url, &oid, is_merge)
        };

        if res {
            0
        } else {
            1
        }
    })
    .unwrap_or(1)
}

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
        crate::init();
        // Normal file path OK (does not need Windows conversion).
        let path = path.as_ref().into_c_string()?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_open(&mut ret, path));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Attempt to open an already-existing bare repository at `path`.
    ///
    /// The path can point to only a bare repository.
    pub fn open_bare<P: AsRef<Path>>(path: P) -> Result<Repository, Error> {
        crate::init();
        // Normal file path OK (does not need Windows conversion).
        let path = path.as_ref().into_c_string()?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_open_bare(&mut ret, path));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Find and open an existing repository, respecting git environment
    /// variables.  This acts like `open_ext` with the
    /// [FROM_ENV](RepositoryOpenFlags::FROM_ENV) flag, but additionally respects `$GIT_DIR`.
    /// With `$GIT_DIR` unset, this will search for a repository starting in
    /// the current directory.
    pub fn open_from_env() -> Result<Repository, Error> {
        crate::init();
        let mut ret = ptr::null_mut();
        let flags = raw::GIT_REPOSITORY_OPEN_FROM_ENV;
        unsafe {
            try_call!(raw::git_repository_open_ext(
                &mut ret,
                ptr::null(),
                flags as c_uint,
                ptr::null()
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Find and open an existing repository, with additional options.
    ///
    /// If flags contains [NO_SEARCH](RepositoryOpenFlags::NO_SEARCH), the path must point
    /// directly to a repository; otherwise, this may point to a subdirectory
    /// of a repository, and `open_ext` will search up through parent
    /// directories.
    ///
    /// If flags contains [CROSS_FS](RepositoryOpenFlags::CROSS_FS), the search through parent
    /// directories will not cross a filesystem boundary (detected when the
    /// stat st_dev field changes).
    ///
    /// If flags contains [BARE](RepositoryOpenFlags::BARE), force opening the repository as
    /// bare even if it isn't, ignoring any working directory, and defer
    /// loading the repository configuration for performance.
    ///
    /// If flags contains [NO_DOTGIT](RepositoryOpenFlags::NO_DOTGIT), don't try appending
    /// `/.git` to `path`.
    ///
    /// If flags contains [FROM_ENV](RepositoryOpenFlags::FROM_ENV), `open_ext` will ignore
    /// other flags and `ceiling_dirs`, and respect the same environment
    /// variables git does. Note, however, that `path` overrides `$GIT_DIR`; to
    /// respect `$GIT_DIR` as well, use `open_from_env`.
    ///
    /// ceiling_dirs specifies a list of paths that the search through parent
    /// directories will stop before entering.  Use the functions in std::env
    /// to construct or manipulate such a path list. (You can use `&[] as
    /// &[&std::ffi::OsStr]` as an argument if there are no ceiling
    /// directories.)
    pub fn open_ext<P, O, I>(
        path: P,
        flags: RepositoryOpenFlags,
        ceiling_dirs: I,
    ) -> Result<Repository, Error>
    where
        P: AsRef<Path>,
        O: AsRef<OsStr>,
        I: IntoIterator<Item = O>,
    {
        crate::init();
        // Normal file path OK (does not need Windows conversion).
        let path = path.as_ref().into_c_string()?;
        let ceiling_dirs_os = env::join_paths(ceiling_dirs)?;
        let ceiling_dirs = ceiling_dirs_os.into_c_string()?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_open_ext(
                &mut ret,
                path,
                flags.bits() as c_uint,
                ceiling_dirs
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Attempt to open an already-existing repository from a worktree.
    pub fn open_from_worktree(worktree: &Worktree) -> Result<Repository, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_open_from_worktree(
                &mut ret,
                worktree.raw()
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Attempt to open an already-existing repository at or above `path`
    ///
    /// This starts at `path` and looks up the filesystem hierarchy
    /// until it finds a repository.
    pub fn discover<P: AsRef<Path>>(path: P) -> Result<Repository, Error> {
        // TODO: this diverges significantly from the libgit2 API
        crate::init();
        let buf = Buf::new();
        // Normal file path OK (does not need Windows conversion).
        let path = path.as_ref().into_c_string()?;
        unsafe {
            try_call!(raw::git_repository_discover(
                buf.raw(),
                path,
                1,
                ptr::null()
            ));
        }
        Repository::open(util::bytes2path(&*buf))
    }

    /// Attempt to find the path to a git repo for a given path
    ///
    /// This starts at `path` and looks up the filesystem hierarchy
    /// until it finds a repository, stopping if it finds a member of ceiling_dirs
    pub fn discover_path<P: AsRef<Path>, I, O>(path: P, ceiling_dirs: I) -> Result<PathBuf, Error>
    where
        O: AsRef<OsStr>,
        I: IntoIterator<Item = O>,
    {
        crate::init();
        let buf = Buf::new();
        // Normal file path OK (does not need Windows conversion).
        let path = path.as_ref().into_c_string()?;
        let ceiling_dirs_os = env::join_paths(ceiling_dirs)?;
        let ceiling_dirs = ceiling_dirs_os.into_c_string()?;
        unsafe {
            try_call!(raw::git_repository_discover(
                buf.raw(),
                path,
                1,
                ceiling_dirs
            ));
        }

        Ok(util::bytes2path(&*buf).to_path_buf())
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

    /// Creates a new repository in the specified folder with the given options.
    ///
    /// See `RepositoryInitOptions` struct for more information.
    pub fn init_opts<P: AsRef<Path>>(
        path: P,
        opts: &RepositoryInitOptions,
    ) -> Result<Repository, Error> {
        crate::init();
        // Normal file path OK (does not need Windows conversion).
        let path = path.as_ref().into_c_string()?;
        let mut ret = ptr::null_mut();
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
    pub fn clone<P: AsRef<Path>>(url: &str, into: P) -> Result<Repository, Error> {
        crate::init();
        RepoBuilder::new().clone(url, into.as_ref())
    }

    /// Clone a remote repository, initialize and update its submodules
    /// recursively.
    ///
    /// This is similar to `git clone --recursive`.
    pub fn clone_recurse<P: AsRef<Path>>(url: &str, into: P) -> Result<Repository, Error> {
        let repo = Repository::clone(url, into)?;
        repo.update_submodules()?;
        Ok(repo)
    }

    /// Attempt to wrap an object database as a repository.
    pub fn from_odb(odb: Odb<'_>) -> Result<Repository, Error> {
        crate::init();
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_wrap_odb(&mut ret, odb.raw()));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Update submodules recursively.
    ///
    /// Uninitialized submodules will be initialized.
    fn update_submodules(&self) -> Result<(), Error> {
        fn add_subrepos(repo: &Repository, list: &mut Vec<Repository>) -> Result<(), Error> {
            for mut subm in repo.submodules()? {
                subm.update(true, None)?;
                list.push(subm.open()?);
            }
            Ok(())
        }

        let mut repos = Vec::new();
        add_subrepos(self, &mut repos)?;
        while let Some(repo) = repos.pop() {
            add_subrepos(&repo, &mut repos)?;
        }
        Ok(())
    }

    /// Execute a rev-parse operation against the `spec` listed.
    ///
    /// The resulting revision specification is returned, or an error is
    /// returned if one occurs.
    pub fn revparse(&self, spec: &str) -> Result<Revspec<'_>, Error> {
        let mut raw = raw::git_revspec {
            from: ptr::null_mut(),
            to: ptr::null_mut(),
            flags: 0,
        };
        let spec = CString::new(spec)?;
        unsafe {
            try_call!(raw::git_revparse(&mut raw, self.raw, spec));
            let to = Binding::from_raw_opt(raw.to);
            let from = Binding::from_raw_opt(raw.from);
            let mode = RevparseMode::from_bits_truncate(raw.flags as u32);
            Ok(Revspec::from_objects(from, to, mode))
        }
    }

    /// Find a single object, as specified by a revision string.
    pub fn revparse_single(&self, spec: &str) -> Result<Object<'_>, Error> {
        let spec = CString::new(spec)?;
        let mut obj = ptr::null_mut();
        unsafe {
            try_call!(raw::git_revparse_single(&mut obj, self.raw, spec));
            assert!(!obj.is_null());
            Ok(Binding::from_raw(obj))
        }
    }

    /// Find a single object and intermediate reference by a revision string.
    ///
    /// See `man gitrevisions`, or
    /// <http://git-scm.com/docs/git-rev-parse.html#_specifying_revisions> for
    /// information on the syntax accepted.
    ///
    /// In some cases (`@{<-n>}` or `<branchname>@{upstream}`), the expression
    /// may point to an intermediate reference. When such expressions are being
    /// passed in, this intermediate reference is returned.
    pub fn revparse_ext(&self, spec: &str) -> Result<(Object<'_>, Option<Reference<'_>>), Error> {
        let spec = CString::new(spec)?;
        let mut git_obj = ptr::null_mut();
        let mut git_ref = ptr::null_mut();
        unsafe {
            try_call!(raw::git_revparse_ext(
                &mut git_obj,
                &mut git_ref,
                self.raw,
                spec
            ));
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

    /// Tests whether this repository is a worktree.
    pub fn is_worktree(&self) -> bool {
        unsafe { raw::git_repository_is_worktree(self.raw) == 1 }
    }

    /// Tests whether this repository is empty.
    pub fn is_empty(&self) -> Result<bool, Error> {
        let empty = unsafe { try_call!(raw::git_repository_is_empty(self.raw)) };
        Ok(empty == 1)
    }

    /// Returns the path to the `.git` folder for normal repositories or the
    /// repository itself for bare repositories.
    pub fn path(&self) -> &Path {
        unsafe {
            let ptr = raw::git_repository_path(self.raw);
            util::bytes2path(crate::opt_bytes(self, ptr).unwrap())
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
            GIT_REPOSITORY_STATE_REVERT_SEQUENCE => RevertSequence,
            GIT_REPOSITORY_STATE_CHERRYPICK => CherryPick,
            GIT_REPOSITORY_STATE_CHERRYPICK_SEQUENCE => CherryPickSequence,
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

    /// Set the path to the working directory for this repository.
    ///
    /// If `update_link` is true, create/update the gitlink file in the workdir
    /// and set config "core.worktree" (if workdir is not the parent of the .git
    /// directory).
    pub fn set_workdir(&self, path: &Path, update_gitlink: bool) -> Result<(), Error> {
        // Normal file path OK (does not need Windows conversion).
        let path = path.into_c_string()?;
        unsafe {
            try_call!(raw::git_repository_set_workdir(
                self.raw(),
                path,
                update_gitlink
            ));
        }
        Ok(())
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
        unsafe { crate::opt_bytes(self, raw::git_repository_get_namespace(self.raw)) }
    }

    /// Set the active namespace for this repository.
    pub fn set_namespace(&self, namespace: &str) -> Result<(), Error> {
        self.set_namespace_bytes(namespace.as_bytes())
    }

    /// Set the active namespace for this repository as a byte array.
    pub fn set_namespace_bytes(&self, namespace: &[u8]) -> Result<(), Error> {
        unsafe {
            let namespace = CString::new(namespace)?;
            try_call!(raw::git_repository_set_namespace(self.raw, namespace));
            Ok(())
        }
    }

    /// Remove the active namespace for this repository.
    pub fn remove_namespace(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_set_namespace(self.raw, ptr::null()));
            Ok(())
        }
    }

    /// Retrieves the Git merge message.
    /// Remember to remove the message when finished.
    pub fn message(&self) -> Result<String, Error> {
        unsafe {
            let buf = Buf::new();
            try_call!(raw::git_repository_message(buf.raw(), self.raw));
            Ok(str::from_utf8(&buf).unwrap().to_string())
        }
    }

    /// Remove the Git merge message.
    pub fn remove_message(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_message_remove(self.raw));
            Ok(())
        }
    }

    /// List all remotes for a given repository
    pub fn remotes(&self) -> Result<StringArray, Error> {
        let mut arr = raw::git_strarray {
            strings: ptr::null_mut(),
            count: 0,
        };
        unsafe {
            try_call!(raw::git_remote_list(&mut arr, self.raw));
            Ok(Binding::from_raw(arr))
        }
    }

    /// Get the information for a particular remote
    pub fn find_remote(&self, name: &str) -> Result<Remote<'_>, Error> {
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_remote_lookup(&mut ret, self.raw, name));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Add a remote with the default fetch refspec to the repository's
    /// configuration.
    pub fn remote(&self, name: &str, url: &str) -> Result<Remote<'_>, Error> {
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        let url = CString::new(url)?;
        unsafe {
            try_call!(raw::git_remote_create(&mut ret, self.raw, name, url));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Add a remote with the provided fetch refspec to the repository's
    /// configuration.
    pub fn remote_with_fetch(
        &self,
        name: &str,
        url: &str,
        fetch: &str,
    ) -> Result<Remote<'_>, Error> {
        let mut ret = ptr::null_mut();
        let name = CString::new(name)?;
        let url = CString::new(url)?;
        let fetch = CString::new(fetch)?;
        unsafe {
            try_call!(raw::git_remote_create_with_fetchspec(
                &mut ret, self.raw, name, url, fetch
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create an anonymous remote
    ///
    /// Create a remote with the given URL and refspec in memory. You can use
    /// this when you have a URL instead of a remote's name. Note that anonymous
    /// remotes cannot be converted to persisted remotes.
    pub fn remote_anonymous(&self, url: &str) -> Result<Remote<'_>, Error> {
        let mut ret = ptr::null_mut();
        let url = CString::new(url)?;
        unsafe {
            try_call!(raw::git_remote_create_anonymous(&mut ret, self.raw, url));
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
    pub fn remote_rename(&self, name: &str, new_name: &str) -> Result<StringArray, Error> {
        let name = CString::new(name)?;
        let new_name = CString::new(new_name)?;
        let mut problems = raw::git_strarray {
            count: 0,
            strings: ptr::null_mut(),
        };
        unsafe {
            try_call!(raw::git_remote_rename(
                &mut problems,
                self.raw,
                name,
                new_name
            ));
            Ok(Binding::from_raw(problems))
        }
    }

    /// Delete an existing persisted remote.
    ///
    /// All remote-tracking branches and configuration settings for the remote
    /// will be removed.
    pub fn remote_delete(&self, name: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_remote_delete(self.raw, name));
        }
        Ok(())
    }

    /// Add a fetch refspec to the remote's configuration
    ///
    /// Add the given refspec to the fetch list in the configuration. No loaded
    /// remote instances will be affected.
    pub fn remote_add_fetch(&self, name: &str, spec: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let spec = CString::new(spec)?;
        unsafe {
            try_call!(raw::git_remote_add_fetch(self.raw, name, spec));
        }
        Ok(())
    }

    /// Add a push refspec to the remote's configuration.
    ///
    /// Add the given refspec to the push list in the configuration. No
    /// loaded remote instances will be affected.
    pub fn remote_add_push(&self, name: &str, spec: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let spec = CString::new(spec)?;
        unsafe {
            try_call!(raw::git_remote_add_push(self.raw, name, spec));
        }
        Ok(())
    }

    /// Set the remote's URL in the configuration
    ///
    /// Remote objects already in memory will not be affected. This assumes
    /// the common case of a single-url remote and will otherwise return an
    /// error.
    pub fn remote_set_url(&self, name: &str, url: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let url = CString::new(url)?;
        unsafe {
            try_call!(raw::git_remote_set_url(self.raw, name, url));
        }
        Ok(())
    }

    /// Set the remote's URL for pushing in the configuration.
    ///
    /// Remote objects already in memory will not be affected. This assumes
    /// the common case of a single-url remote and will otherwise return an
    /// error.
    ///
    /// `None` indicates that it should be cleared.
    pub fn remote_set_pushurl(&self, name: &str, pushurl: Option<&str>) -> Result<(), Error> {
        let name = CString::new(name)?;
        let pushurl = crate::opt_cstr(pushurl)?;
        unsafe {
            try_call!(raw::git_remote_set_pushurl(self.raw, name, pushurl));
        }
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
    /// object can either be a commit or a tag, but tags must be dereferenceable
    /// to a commit.
    ///
    /// The `checkout` options will only be used for a hard reset.
    pub fn reset(
        &self,
        target: &Object<'_>,
        kind: ResetType,
        checkout: Option<&mut CheckoutBuilder<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let mut opts: raw::git_checkout_options = mem::zeroed();
            try_call!(raw::git_checkout_init_options(
                &mut opts,
                raw::GIT_CHECKOUT_OPTIONS_VERSION
            ));
            let opts = checkout.map(|c| {
                c.configure(&mut opts);
                &mut opts
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
    pub fn reset_default<T, I>(&self, target: Option<&Object<'_>>, paths: I) -> Result<(), Error>
    where
        T: IntoCString,
        I: IntoIterator<Item = T>,
    {
        let (_a, _b, mut arr) = crate::util::iter2cstrs_paths(paths)?;
        let target = target.map(|t| t.raw());
        unsafe {
            try_call!(raw::git_reset_default(self.raw, target, &mut arr));
        }
        Ok(())
    }

    /// Retrieve and resolve the reference pointed at by HEAD.
    pub fn head(&self) -> Result<Reference<'_>, Error> {
        let mut ret = ptr::null_mut();
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
        self.set_head_bytes(refname.as_bytes())
    }

    /// Make the repository HEAD point to the specified reference as a byte array.
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
    pub fn set_head_bytes(&self, refname: &[u8]) -> Result<(), Error> {
        let refname = CString::new(refname)?;
        unsafe {
            try_call!(raw::git_repository_set_head(self.raw, refname));
        }
        Ok(())
    }

    /// Determines whether the repository HEAD is detached.
    pub fn head_detached(&self) -> Result<bool, Error> {
        unsafe {
            let value = raw::git_repository_head_detached(self.raw);
            match value {
                0 => Ok(false),
                1 => Ok(true),
                _ => Err(Error::last_error(value).unwrap()),
            }
        }
    }

    /// Make the repository HEAD directly point to the commit.
    ///
    /// If the provided commitish cannot be found in the repository, the HEAD
    /// is unaltered and an error is returned.
    ///
    /// If the provided commitish cannot be peeled into a commit, the HEAD is
    /// unaltered and an error is returned.
    ///
    /// Otherwise, the HEAD will eventually be detached and will directly point
    /// to the peeled commit.
    pub fn set_head_detached(&self, commitish: Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_set_head_detached(
                self.raw,
                commitish.raw()
            ));
        }
        Ok(())
    }

    /// Make the repository HEAD directly point to the commit.
    ///
    /// If the provided commitish cannot be found in the repository, the HEAD
    /// is unaltered and an error is returned.
    /// If the provided commitish cannot be peeled into a commit, the HEAD is
    /// unaltered and an error is returned.
    /// Otherwise, the HEAD will eventually be detached and will directly point
    /// to the peeled commit.
    pub fn set_head_detached_from_annotated(
        &self,
        commitish: AnnotatedCommit<'_>,
    ) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_set_head_detached_from_annotated(
                self.raw,
                commitish.raw()
            ));
        }
        Ok(())
    }

    /// Create an iterator for the repo's references
    pub fn references(&self) -> Result<References<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_iterator_new(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create an iterator for the repo's references that match the specified
    /// glob
    pub fn references_glob(&self, glob: &str) -> Result<References<'_>, Error> {
        let mut ret = ptr::null_mut();
        let glob = CString::new(glob)?;
        unsafe {
            try_call!(raw::git_reference_iterator_glob_new(
                &mut ret, self.raw, glob
            ));

            Ok(Binding::from_raw(ret))
        }
    }

    /// Load all submodules for this repository and return them.
    pub fn submodules(&self) -> Result<Vec<Submodule<'_>>, Error> {
        struct Data<'a, 'b> {
            repo: &'b Repository,
            ret: &'a mut Vec<Submodule<'b>>,
        }
        let mut ret = Vec::new();

        unsafe {
            let mut data = Data {
                repo: self,
                ret: &mut ret,
            };
            let cb: raw::git_submodule_cb = Some(append);
            try_call!(raw::git_submodule_foreach(
                self.raw,
                cb,
                &mut data as *mut _ as *mut c_void
            ));
        }

        return Ok(ret);

        extern "C" fn append(
            _repo: *mut raw::git_submodule,
            name: *const c_char,
            data: *mut c_void,
        ) -> c_int {
            unsafe {
                let data = &mut *(data as *mut Data<'_, '_>);
                let mut raw = ptr::null_mut();
                let rc = raw::git_submodule_lookup(&mut raw, data.repo.raw(), name);
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
    pub fn statuses(&self, options: Option<&mut StatusOptions>) -> Result<Statuses<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_status_list_new(
                &mut ret,
                self.raw,
                options.map(|s| s.raw()).unwrap_or(ptr::null())
            ));
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
        let path = util::cstring_to_repo_path(path)?;
        unsafe {
            try_call!(raw::git_status_should_ignore(&mut ret, self.raw, path));
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
        let path = path_to_repo_path(path)?;
        unsafe {
            try_call!(raw::git_status_file(&mut ret, self.raw, path));
        }
        Ok(Status::from_bits_truncate(ret as u32))
    }

    /// Create an iterator which loops over the requested branches.
    pub fn branches(&self, filter: Option<BranchType>) -> Result<Branches<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_branch_iterator_new(&mut raw, self.raw(), filter));
            Ok(Branches::from_raw(raw))
        }
    }

    /// Get the Index file for this repository.
    ///
    /// If a custom index has not been set, the default index for the repository
    /// will be returned (the one located in .git/index).
    ///
    /// **Caution**: If the [`Repository`] of this index is dropped, then this
    /// [`Index`] will become detached, and most methods on it will fail. See
    /// [`Index::open`]. Be sure the repository has a binding such as a local
    /// variable to keep it alive at least as long as the index.
    pub fn index(&self) -> Result<Index, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_index(&mut raw, self.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Set the Index file for this repository.
    pub fn set_index(&self, index: &mut Index) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_set_index(self.raw(), index.raw()));
        }
        Ok(())
    }

    /// Get the configuration file for this repository.
    ///
    /// If a configuration file has not been set, the default config set for the
    /// repository will be returned, including global and system configurations
    /// (if they are available).
    pub fn config(&self) -> Result<Config, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_config(&mut raw, self.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the value of a git attribute for a path as a string.
    ///
    /// This function will return a special string if the attribute is set to a special value.
    /// Interpreting the special string is discouraged. You should always use
    /// [`AttrValue::from_string`](crate::AttrValue::from_string) to interpret the return value
    /// and avoid the special string.
    ///
    /// As such, the return type of this function will probably be changed in the next major version
    /// to prevent interpreting the returned string without checking whether it's special.
    pub fn get_attr(
        &self,
        path: &Path,
        name: &str,
        flags: AttrCheckFlags,
    ) -> Result<Option<&str>, Error> {
        Ok(self
            .get_attr_bytes(path, name, flags)?
            .and_then(|a| str::from_utf8(a).ok()))
    }

    /// Get the value of a git attribute for a path as a byte slice.
    ///
    /// This function will return a special byte slice if the attribute is set to a special value.
    /// Interpreting the special byte slice is discouraged. You should always use
    /// [`AttrValue::from_bytes`](crate::AttrValue::from_bytes) to interpret the return value and
    /// avoid the special string.
    ///
    /// As such, the return type of this function will probably be changed in the next major version
    /// to prevent interpreting the returned byte slice without checking whether it's special.
    pub fn get_attr_bytes(
        &self,
        path: &Path,
        name: &str,
        flags: AttrCheckFlags,
    ) -> Result<Option<&[u8]>, Error> {
        let mut ret = ptr::null();
        let path = util::cstring_to_repo_path(path)?;
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_attr_get(
                &mut ret,
                self.raw(),
                flags.bits(),
                path,
                name
            ));
            Ok(crate::opt_bytes(self, ret))
        }
    }

    /// Write an in-memory buffer to the ODB as a blob.
    ///
    /// The Oid returned can in turn be passed to `find_blob` to get a handle to
    /// the blob.
    pub fn blob(&self, data: &[u8]) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            let ptr = data.as_ptr() as *const c_void;
            let len = data.len() as size_t;
            try_call!(raw::git_blob_create_frombuffer(
                &mut raw,
                self.raw(),
                ptr,
                len
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Read a file from the filesystem and write its content to the Object
    /// Database as a loose blob
    ///
    /// The Oid returned can in turn be passed to `find_blob` to get a handle to
    /// the blob.
    pub fn blob_path(&self, path: &Path) -> Result<Oid, Error> {
        // Normal file path OK (does not need Windows conversion).
        let path = path.into_c_string()?;
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_blob_create_fromdisk(&mut raw, self.raw(), path));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Create a stream to write blob
    ///
    /// This function may need to buffer the data on disk and will in general
    /// not be the right choice if you know the size of the data to write.
    ///
    /// Use `BlobWriter::commit()` to commit the write to the object db
    /// and get the object id.
    ///
    /// If the `hintpath` parameter is filled, it will be used to determine
    /// what git filters should be applied to the object before it is written
    /// to the object database.
    pub fn blob_writer(&self, hintpath: Option<&Path>) -> Result<BlobWriter<'_>, Error> {
        let path_str = match hintpath {
            Some(path) => Some(path.into_c_string()?),
            None => None,
        };
        let path = match path_str {
            Some(ref path) => path.as_ptr(),
            None => ptr::null(),
        };
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_blob_create_fromstream(&mut out, self.raw(), path));
            Ok(BlobWriter::from_raw(out))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_blob(&self, oid: Oid) -> Result<Blob<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_blob_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the object database for this repository
    pub fn odb(&self) -> Result<Odb<'_>, Error> {
        let mut odb = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_odb(&mut odb, self.raw()));
            Ok(Odb::from_raw(odb))
        }
    }

    /// Override the object database for this repository
    pub fn set_odb(&self, odb: &Odb<'_>) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_set_odb(self.raw(), odb.raw()));
        }
        Ok(())
    }

    /// Create a new branch pointing at a target commit
    ///
    /// A new direct reference will be created pointing to this target commit.
    /// If `force` is true and a reference already exists with the given name,
    /// it'll be replaced.
    pub fn branch(
        &self,
        branch_name: &str,
        target: &Commit<'_>,
        force: bool,
    ) -> Result<Branch<'_>, Error> {
        let branch_name = CString::new(branch_name)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_branch_create(
                &mut raw,
                self.raw(),
                branch_name,
                target.raw(),
                force
            ));
            Ok(Branch::wrap(Binding::from_raw(raw)))
        }
    }

    /// Create a new branch pointing at a target commit
    ///
    /// This behaves like `Repository::branch()` but takes
    /// an annotated commit, which lets you specify which
    /// extended SHA syntax string was specified by a user,
    /// allowing for more exact reflog messages.
    ///
    /// See the documentation for `Repository::branch()`
    pub fn branch_from_annotated_commit(
        &self,
        branch_name: &str,
        target: &AnnotatedCommit<'_>,
        force: bool,
    ) -> Result<Branch<'_>, Error> {
        let branch_name = CString::new(branch_name)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_branch_create_from_annotated(
                &mut raw,
                self.raw(),
                branch_name,
                target.raw(),
                force
            ));
            Ok(Branch::wrap(Binding::from_raw(raw)))
        }
    }

    /// Lookup a branch by its name in a repository.
    pub fn find_branch(&self, name: &str, branch_type: BranchType) -> Result<Branch<'_>, Error> {
        let name = CString::new(name)?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_branch_lookup(
                &mut ret,
                self.raw(),
                name,
                branch_type
            ));
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
    pub fn commit(
        &self,
        update_ref: Option<&str>,
        author: &Signature<'_>,
        committer: &Signature<'_>,
        message: &str,
        tree: &Tree<'_>,
        parents: &[&Commit<'_>],
    ) -> Result<Oid, Error> {
        let update_ref = crate::opt_cstr(update_ref)?;
        let parent_ptrs = parents
            .iter()
            .map(|p| p.raw() as *mut raw::git_commit)
            .collect::<Vec<_>>();
        let message = CString::new(message)?;
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_commit_create(
                &mut raw,
                self.raw(),
                update_ref,
                author.raw(),
                committer.raw(),
                ptr::null(),
                message,
                tree.raw(),
                parents.len() as size_t,
                parent_ptrs.as_ptr()
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Create a commit object and return that as a Buf.
    ///
    /// That can be converted to a string like this `str::from_utf8(&buf).unwrap().to_string()`.
    /// And that string can be passed to the `commit_signed` function,
    /// the arguments behave the same as in the `commit` function.
    pub fn commit_create_buffer(
        &self,
        author: &Signature<'_>,
        committer: &Signature<'_>,
        message: &str,
        tree: &Tree<'_>,
        parents: &[&Commit<'_>],
    ) -> Result<Buf, Error> {
        let parent_ptrs = parents
            .iter()
            .map(|p| p.raw() as *mut raw::git_commit)
            .collect::<Vec<_>>();
        let message = CString::new(message)?;
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_commit_create_buffer(
                buf.raw(),
                self.raw(),
                author.raw(),
                committer.raw(),
                ptr::null(),
                message,
                tree.raw(),
                parents.len() as size_t,
                parent_ptrs.as_ptr()
            ));
            Ok(buf)
        }
    }

    /// Create a commit object from the given buffer and signature
    ///
    /// Given the unsigned commit object's contents, its signature and the
    /// header field in which to store the signature, attach the signature to
    /// the commit and write it into the given repository.
    ///
    /// Use `None` in `signature_field` to use the default of `gpgsig`, which is
    /// almost certainly what you want.
    ///
    /// Returns the resulting (signed) commit id.
    pub fn commit_signed(
        &self,
        commit_content: &str,
        signature: &str,
        signature_field: Option<&str>,
    ) -> Result<Oid, Error> {
        let commit_content = CString::new(commit_content)?;
        let signature = CString::new(signature)?;
        let signature_field = crate::opt_cstr(signature_field)?;
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_commit_create_with_signature(
                &mut raw,
                self.raw(),
                commit_content,
                signature,
                signature_field
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Extract the signature from a commit
    ///
    /// Returns a tuple containing the signature in the first value and the
    /// signed data in the second.
    pub fn extract_signature(
        &self,
        commit_id: &Oid,
        signature_field: Option<&str>,
    ) -> Result<(Buf, Buf), Error> {
        let signature_field = crate::opt_cstr(signature_field)?;
        let signature = Buf::new();
        let content = Buf::new();
        unsafe {
            try_call!(raw::git_commit_extract_signature(
                signature.raw(),
                content.raw(),
                self.raw(),
                commit_id.raw() as *mut _,
                signature_field
            ));
            Ok((signature, content))
        }
    }

    /// Lookup a reference to one of the commits in a repository.
    pub fn find_commit(&self, oid: Oid) -> Result<Commit<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_commit_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the commits in a repository by short hash.
    pub fn find_commit_by_prefix(&self, prefix_hash: &str) -> Result<Commit<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_commit_lookup_prefix(
                &mut raw,
                self.raw(),
                Oid::from_str(prefix_hash)?.raw(),
                prefix_hash.len()
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Creates an `AnnotatedCommit` from the given commit id.
    pub fn find_annotated_commit(&self, id: Oid) -> Result<AnnotatedCommit<'_>, Error> {
        unsafe {
            let mut raw = ptr::null_mut();
            try_call!(raw::git_annotated_commit_lookup(
                &mut raw,
                self.raw(),
                id.raw()
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_object(&self, oid: Oid, kind: Option<ObjectType>) -> Result<Object<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_object_lookup(
                &mut raw,
                self.raw(),
                oid.raw(),
                kind
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects by id prefix in a repository.
    pub fn find_object_by_prefix(
        &self,
        prefix_hash: &str,
        kind: Option<ObjectType>,
    ) -> Result<Object<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_object_lookup_prefix(
                &mut raw,
                self.raw(),
                Oid::from_str(prefix_hash)?.raw(),
                prefix_hash.len(),
                kind
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new direct reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn reference(
        &self,
        name: &str,
        id: Oid,
        force: bool,
        log_message: &str,
    ) -> Result<Reference<'_>, Error> {
        let name = CString::new(name)?;
        let log_message = CString::new(log_message)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_create(
                &mut raw,
                self.raw(),
                name,
                id.raw(),
                force,
                log_message
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Conditionally create new direct reference.
    ///
    /// A direct reference (also called an object id reference) refers directly
    /// to a specific object id (a.k.a. OID or SHA) in the repository.  The id
    /// permanently refers to the object (although the reference itself can be
    /// moved).  For example, in libgit2 the direct ref "refs/tags/v0.17.0"
    /// refers to OID 5b9fac39d8a76b9139667c26a63e6b3f204b3977.
    ///
    /// The direct reference will be created in the repository and written to
    /// the disk.
    ///
    /// Valid reference names must follow one of two patterns:
    ///
    /// 1. Top-level names must contain only capital letters and underscores,
    ///    and must begin and end with a letter.  (e.g.  "HEAD", "ORIG_HEAD").
    /// 2. Names prefixed with "refs/" can be almost anything.  You must avoid
    ///    the characters `~`, `^`, `:`, `\\`, `?`, `[`, and `*`, and the
    ///    sequences ".." and "@{" which have special meaning to revparse.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless `force` is true, in which case it will be
    /// overwritten.
    ///
    /// The message for the reflog will be ignored if the reference does not
    /// belong in the standard set (HEAD, branches and remote-tracking
    /// branches) and it does not have a reflog.
    ///
    /// It will return GIT_EMODIFIED if the reference's value at the time of
    /// updating does not match the one passed through `current_id` (i.e. if the
    /// ref has changed since the user read it).
    pub fn reference_matching(
        &self,
        name: &str,
        id: Oid,
        force: bool,
        current_id: Oid,
        log_message: &str,
    ) -> Result<Reference<'_>, Error> {
        let name = CString::new(name)?;
        let log_message = CString::new(log_message)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_create_matching(
                &mut raw,
                self.raw(),
                name,
                id.raw(),
                force,
                current_id.raw(),
                log_message
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new symbolic reference.
    ///
    /// A symbolic reference is a reference name that refers to another
    /// reference name.  If the other name moves, the symbolic name will move,
    /// too.  As a simple example, the "HEAD" reference might refer to
    /// "refs/heads/master" while on the "master" branch of a repository.
    ///
    /// Valid reference names must follow one of two patterns:
    ///
    /// 1. Top-level names must contain only capital letters and underscores,
    ///    and must begin and end with a letter. (e.g. "HEAD", "ORIG_HEAD").
    /// 2. Names prefixed with "refs/" can be almost anything.  You must avoid
    ///    the characters '~', '^', ':', '\\', '?', '[', and '*', and the
    ///    sequences ".." and "@{" which have special meaning to revparse.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn reference_symbolic(
        &self,
        name: &str,
        target: &str,
        force: bool,
        log_message: &str,
    ) -> Result<Reference<'_>, Error> {
        let name = CString::new(name)?;
        let target = CString::new(target)?;
        let log_message = CString::new(log_message)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_symbolic_create(
                &mut raw,
                self.raw(),
                name,
                target,
                force,
                log_message
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new symbolic reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    ///
    /// It will return GIT_EMODIFIED if the reference's value at the time of
    /// updating does not match the one passed through current_value (i.e. if
    /// the ref has changed since the user read it).
    pub fn reference_symbolic_matching(
        &self,
        name: &str,
        target: &str,
        force: bool,
        current_value: &str,
        log_message: &str,
    ) -> Result<Reference<'_>, Error> {
        let name = CString::new(name)?;
        let target = CString::new(target)?;
        let current_value = CString::new(current_value)?;
        let log_message = CString::new(log_message)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_symbolic_create_matching(
                &mut raw,
                self.raw(),
                name,
                target,
                force,
                current_value,
                log_message
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_reference(&self, name: &str) -> Result<Reference<'_>, Error> {
        let name = CString::new(name)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_lookup(&mut raw, self.raw(), name));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    /// `Repository::find_reference` with teeth; give the method your reference in
    /// human-readable format e.g. 'main' instead of 'refs/heads/main', and it
    /// will do-what-you-mean, returning the `Reference`.
    pub fn resolve_reference_from_short_name(&self, refname: &str) -> Result<Reference<'_>, Error> {
        let refname = CString::new(refname)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_dwim(&mut raw, self.raw(), refname));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a reference by name and resolve immediately to OID.
    ///
    /// This function provides a quick way to resolve a reference name straight
    /// through to the object id that it refers to. This avoids having to
    /// allocate or free any `Reference` objects for simple situations.
    pub fn refname_to_id(&self, name: &str) -> Result<Oid, Error> {
        let name = CString::new(name)?;
        let mut ret = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_reference_name_to_id(&mut ret, self.raw(), name));
            Ok(Binding::from_raw(&ret as *const _))
        }
    }

    /// Creates a git_annotated_commit from the given reference.
    pub fn reference_to_annotated_commit(
        &self,
        reference: &Reference<'_>,
    ) -> Result<AnnotatedCommit<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_annotated_commit_from_ref(
                &mut ret,
                self.raw(),
                reference.raw()
            ));
            Ok(AnnotatedCommit::from_raw(ret))
        }
    }

    /// Creates a git_annotated_commit from FETCH_HEAD.
    pub fn annotated_commit_from_fetchhead(
        &self,
        branch_name: &str,
        remote_url: &str,
        id: &Oid,
    ) -> Result<AnnotatedCommit<'_>, Error> {
        let branch_name = CString::new(branch_name)?;
        let remote_url = CString::new(remote_url)?;

        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_annotated_commit_from_fetchhead(
                &mut ret,
                self.raw(),
                branch_name,
                remote_url,
                id.raw()
            ));
            Ok(AnnotatedCommit::from_raw(ret))
        }
    }

    /// Create a new action signature with default user and now timestamp.
    ///
    /// This looks up the user.name and user.email from the configuration and
    /// uses the current time as the timestamp, and creates a new signature
    /// based on that information. It will return `NotFound` if either the
    /// user.name or user.email are not set.
    pub fn signature(&self) -> Result<Signature<'static>, Error> {
        let mut ret = ptr::null_mut();
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
    /// `add_finalize()` to wrap up adding the new submodule and `.gitmodules`
    /// to the index to be ready to commit.
    pub fn submodule(
        &self,
        url: &str,
        path: &Path,
        use_gitlink: bool,
    ) -> Result<Submodule<'_>, Error> {
        let url = CString::new(url)?;
        let path = path_to_repo_path(path)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_submodule_add_setup(
                &mut raw,
                self.raw(),
                url,
                path,
                use_gitlink
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup submodule information by name or path.
    ///
    /// Given either the submodule name or path (they are usually the same),
    /// this returns a structure describing the submodule.
    pub fn find_submodule(&self, name: &str) -> Result<Submodule<'_>, Error> {
        let name = CString::new(name)?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_submodule_lookup(&mut raw, self.raw(), name));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the status for a submodule.
    ///
    /// This looks at a submodule and tries to determine the status.  It
    /// will return a combination of the `SubmoduleStatus` values.
    pub fn submodule_status(
        &self,
        name: &str,
        ignore: SubmoduleIgnore,
    ) -> Result<SubmoduleStatus, Error> {
        let mut ret = 0;
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_submodule_status(&mut ret, self.raw, name, ignore));
        }
        Ok(SubmoduleStatus::from_bits_truncate(ret as u32))
    }

    /// Set the ignore rule for the submodule in the configuration
    ///
    /// This does not affect any currently-loaded instances.
    pub fn submodule_set_ignore(
        &mut self,
        name: &str,
        ignore: SubmoduleIgnore,
    ) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_submodule_set_ignore(self.raw(), name, ignore));
        }
        Ok(())
    }

    /// Set the update rule for the submodule in the configuration
    ///
    /// This setting won't affect any existing instances.
    pub fn submodule_set_update(
        &mut self,
        name: &str,
        update: SubmoduleUpdate,
    ) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_submodule_set_update(self.raw(), name, update));
        }
        Ok(())
    }

    /// Set the URL for the submodule in the configuration
    ///
    /// After calling this, you may wish to call [`Submodule::sync`] to write
    /// the changes to the checked out submodule repository.
    pub fn submodule_set_url(&mut self, name: &str, url: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let url = CString::new(url)?;
        unsafe {
            try_call!(raw::git_submodule_set_url(self.raw(), name, url));
        }
        Ok(())
    }

    /// Set the branch for the submodule in the configuration
    ///
    /// After calling this, you may wish to call [`Submodule::sync`] to write
    /// the changes to the checked out submodule repository.
    pub fn submodule_set_branch(&mut self, name: &str, branch_name: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        let branch_name = CString::new(branch_name)?;
        unsafe {
            try_call!(raw::git_submodule_set_branch(self.raw(), name, branch_name));
        }
        Ok(())
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn find_tree(&self, oid: Oid) -> Result<Tree<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tree_lookup(&mut raw, self.raw(), oid.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new TreeBuilder, optionally initialized with the
    /// entries of the given Tree.
    ///
    /// The tree builder can be used to create or modify trees in memory and
    /// write them as tree objects to the database.
    pub fn treebuilder(&self, tree: Option<&Tree<'_>>) -> Result<TreeBuilder<'_>, Error> {
        unsafe {
            let mut ret = ptr::null_mut();
            let tree = match tree {
                Some(tree) => tree.raw(),
                None => ptr::null_mut(),
            };
            try_call!(raw::git_treebuilder_new(&mut ret, self.raw, tree));
            Ok(Binding::from_raw(ret))
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
    pub fn tag(
        &self,
        name: &str,
        target: &Object<'_>,
        tagger: &Signature<'_>,
        message: &str,
        force: bool,
    ) -> Result<Oid, Error> {
        let name = CString::new(name)?;
        let message = CString::new(message)?;
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_tag_create(
                &mut raw,
                self.raw,
                name,
                target.raw(),
                tagger.raw(),
                message,
                force
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Create a new tag in the repository from an object without creating a reference.
    ///
    /// The message will not be cleaned up.
    ///
    /// The tag name will be checked for validity. You must avoid the characters
    /// '~', '^', ':', ' \ ', '?', '[', and '*', and the sequences ".." and " @
    /// {" which have special meaning to revparse.
    pub fn tag_annotation_create(
        &self,
        name: &str,
        target: &Object<'_>,
        tagger: &Signature<'_>,
        message: &str,
    ) -> Result<Oid, Error> {
        let name = CString::new(name)?;
        let message = CString::new(message)?;
        let mut raw_oid = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_tag_annotation_create(
                &mut raw_oid,
                self.raw,
                name,
                target.raw(),
                tagger.raw(),
                message
            ));
            Ok(Binding::from_raw(&raw_oid as *const _))
        }
    }

    /// Create a new lightweight tag pointing at a target object
    ///
    /// A new direct reference will be created pointing to this target object.
    /// If force is true and a reference already exists with the given name,
    /// it'll be replaced.
    pub fn tag_lightweight(
        &self,
        name: &str,
        target: &Object<'_>,
        force: bool,
    ) -> Result<Oid, Error> {
        let name = CString::new(name)?;
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_tag_create_lightweight(
                &mut raw,
                self.raw,
                name,
                target.raw(),
                force
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Lookup a tag object from the repository.
    pub fn find_tag(&self, id: Oid) -> Result<Tag<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tag_lookup(&mut raw, self.raw, id.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Lookup a tag object by prefix hash from the repository.
    pub fn find_tag_by_prefix(&self, prefix_hash: &str) -> Result<Tag<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tag_lookup_prefix(
                &mut raw,
                self.raw,
                Oid::from_str(prefix_hash)?.raw(),
                prefix_hash.len()
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Delete an existing tag reference.
    ///
    /// The tag name will be checked for validity, see `tag` for some rules
    /// about valid names.
    pub fn tag_delete(&self, name: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
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
            strings: ptr::null_mut(),
            count: 0,
        };
        unsafe {
            match pattern {
                Some(s) => {
                    let s = CString::new(s)?;
                    try_call!(raw::git_tag_list_match(&mut arr, s, self.raw));
                }
                None => {
                    try_call!(raw::git_tag_list(&mut arr, self.raw));
                }
            }
            Ok(Binding::from_raw(arr))
        }
    }

    /// iterate over all tags calling `cb` on each.
    /// the callback is provided the tag id and name
    pub fn tag_foreach<T>(&self, cb: T) -> Result<(), Error>
    where
        T: FnMut(Oid, &[u8]) -> bool,
    {
        let mut data = TagForeachData {
            cb: Box::new(cb) as TagForeachCB<'_>,
        };

        unsafe {
            raw::git_tag_foreach(
                self.raw,
                Some(tag_foreach_cb),
                (&mut data) as *mut _ as *mut _,
            );
        }
        Ok(())
    }

    /// Updates files in the index and the working tree to match the content of
    /// the commit pointed at by HEAD.
    pub fn checkout_head(&self, opts: Option<&mut CheckoutBuilder<'_>>) -> Result<(), Error> {
        unsafe {
            let mut raw_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(
                &mut raw_opts,
                raw::GIT_CHECKOUT_OPTIONS_VERSION
            ));
            if let Some(c) = opts {
                c.configure(&mut raw_opts);
            }

            try_call!(raw::git_checkout_head(self.raw, &raw_opts));
        }
        Ok(())
    }

    /// Updates files in the working tree to match the content of the index.
    ///
    /// If the index is `None`, the repository's index will be used.
    pub fn checkout_index(
        &self,
        index: Option<&mut Index>,
        opts: Option<&mut CheckoutBuilder<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let mut raw_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(
                &mut raw_opts,
                raw::GIT_CHECKOUT_OPTIONS_VERSION
            ));
            if let Some(c) = opts {
                c.configure(&mut raw_opts);
            }

            try_call!(raw::git_checkout_index(
                self.raw,
                index.map(|i| &mut *i.raw()),
                &raw_opts
            ));
        }
        Ok(())
    }

    /// Updates files in the index and working tree to match the content of the
    /// tree pointed at by the treeish.
    pub fn checkout_tree(
        &self,
        treeish: &Object<'_>,
        opts: Option<&mut CheckoutBuilder<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let mut raw_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(
                &mut raw_opts,
                raw::GIT_CHECKOUT_OPTIONS_VERSION
            ));
            if let Some(c) = opts {
                c.configure(&mut raw_opts);
            }

            try_call!(raw::git_checkout_tree(self.raw, &*treeish.raw(), &raw_opts));
        }
        Ok(())
    }

    /// Merges the given commit(s) into HEAD, writing the results into the
    /// working directory. Any changes are staged for commit and any conflicts
    /// are written to the index. Callers should inspect the repository's index
    /// after this completes, resolve any conflicts and prepare a commit.
    ///
    /// For compatibility with git, the repository is put into a merging state.
    /// Once the commit is done (or if the user wishes to abort), you should
    /// clear this state by calling cleanup_state().
    pub fn merge(
        &self,
        annotated_commits: &[&AnnotatedCommit<'_>],
        merge_opts: Option<&mut MergeOptions>,
        checkout_opts: Option<&mut CheckoutBuilder<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let mut raw_checkout_opts = mem::zeroed();
            try_call!(raw::git_checkout_init_options(
                &mut raw_checkout_opts,
                raw::GIT_CHECKOUT_OPTIONS_VERSION
            ));
            if let Some(c) = checkout_opts {
                c.configure(&mut raw_checkout_opts);
            }

            let mut commit_ptrs = annotated_commits
                .iter()
                .map(|c| c.raw() as *const raw::git_annotated_commit)
                .collect::<Vec<_>>();

            try_call!(raw::git_merge(
                self.raw,
                commit_ptrs.as_mut_ptr(),
                annotated_commits.len() as size_t,
                merge_opts.map(|o| o.raw()).unwrap_or(ptr::null()),
                &raw_checkout_opts
            ));
        }
        Ok(())
    }

    /// Merge two commits, producing an index that reflects the result of
    /// the merge. The index may be written as-is to the working directory or
    /// checked out. If the index is to be converted to a tree, the caller
    /// should resolve any conflicts that arose as part of the merge.
    pub fn merge_commits(
        &self,
        our_commit: &Commit<'_>,
        their_commit: &Commit<'_>,
        opts: Option<&MergeOptions>,
    ) -> Result<Index, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_merge_commits(
                &mut raw,
                self.raw,
                our_commit.raw(),
                their_commit.raw(),
                opts.map(|o| o.raw())
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Merge two trees, producing an index that reflects the result of
    /// the merge. The index may be written as-is to the working directory or
    /// checked out. If the index is to be converted to a tree, the caller
    /// should resolve any conflicts that arose as part of the merge.
    pub fn merge_trees(
        &self,
        ancestor_tree: &Tree<'_>,
        our_tree: &Tree<'_>,
        their_tree: &Tree<'_>,
        opts: Option<&MergeOptions>,
    ) -> Result<Index, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_merge_trees(
                &mut raw,
                self.raw,
                ancestor_tree.raw(),
                our_tree.raw(),
                their_tree.raw(),
                opts.map(|o| o.raw())
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Remove all the metadata associated with an ongoing command like merge,
    /// revert, cherry-pick, etc. For example: MERGE_HEAD, MERGE_MSG, etc.
    pub fn cleanup_state(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_repository_state_cleanup(self.raw));
        }
        Ok(())
    }

    /// Analyzes the given branch(es) and determines the opportunities for
    /// merging them into the HEAD of the repository.
    pub fn merge_analysis(
        &self,
        their_heads: &[&AnnotatedCommit<'_>],
    ) -> Result<(MergeAnalysis, MergePreference), Error> {
        unsafe {
            let mut raw_merge_analysis = 0 as raw::git_merge_analysis_t;
            let mut raw_merge_preference = 0 as raw::git_merge_preference_t;
            let mut their_heads = their_heads
                .iter()
                .map(|v| v.raw() as *const _)
                .collect::<Vec<_>>();
            try_call!(raw::git_merge_analysis(
                &mut raw_merge_analysis,
                &mut raw_merge_preference,
                self.raw,
                their_heads.as_mut_ptr() as *mut _,
                their_heads.len()
            ));
            Ok((
                MergeAnalysis::from_bits_truncate(raw_merge_analysis as u32),
                MergePreference::from_bits_truncate(raw_merge_preference as u32),
            ))
        }
    }

    /// Analyzes the given branch(es) and determines the opportunities for
    /// merging them into a reference.
    pub fn merge_analysis_for_ref(
        &self,
        our_ref: &Reference<'_>,
        their_heads: &[&AnnotatedCommit<'_>],
    ) -> Result<(MergeAnalysis, MergePreference), Error> {
        unsafe {
            let mut raw_merge_analysis = 0 as raw::git_merge_analysis_t;
            let mut raw_merge_preference = 0 as raw::git_merge_preference_t;
            let mut their_heads = their_heads
                .iter()
                .map(|v| v.raw() as *const _)
                .collect::<Vec<_>>();
            try_call!(raw::git_merge_analysis_for_ref(
                &mut raw_merge_analysis,
                &mut raw_merge_preference,
                self.raw,
                our_ref.raw(),
                their_heads.as_mut_ptr() as *mut _,
                their_heads.len()
            ));
            Ok((
                MergeAnalysis::from_bits_truncate(raw_merge_analysis as u32),
                MergePreference::from_bits_truncate(raw_merge_preference as u32),
            ))
        }
    }

    /// Initializes a rebase operation to rebase the changes in `branch`
    /// relative to `upstream` onto another branch. To begin the rebase process,
    /// call `next()`.
    pub fn rebase(
        &self,
        branch: Option<&AnnotatedCommit<'_>>,
        upstream: Option<&AnnotatedCommit<'_>>,
        onto: Option<&AnnotatedCommit<'_>>,
        opts: Option<&mut RebaseOptions<'_>>,
    ) -> Result<Rebase<'_>, Error> {
        let mut rebase: *mut raw::git_rebase = ptr::null_mut();
        unsafe {
            try_call!(raw::git_rebase_init(
                &mut rebase,
                self.raw(),
                branch.map(|c| c.raw()),
                upstream.map(|c| c.raw()),
                onto.map(|c| c.raw()),
                opts.map(|o| o.raw()).unwrap_or(ptr::null())
            ));

            Ok(Rebase::from_raw(rebase))
        }
    }

    /// Opens an existing rebase that was previously started by either an
    /// invocation of `rebase()` or by another client.
    pub fn open_rebase(&self, opts: Option<&mut RebaseOptions<'_>>) -> Result<Rebase<'_>, Error> {
        let mut rebase: *mut raw::git_rebase = ptr::null_mut();
        unsafe {
            try_call!(raw::git_rebase_open(
                &mut rebase,
                self.raw(),
                opts.map(|o| o.raw()).unwrap_or(ptr::null())
            ));
            Ok(Rebase::from_raw(rebase))
        }
    }

    /// Add a note for an object
    ///
    /// The `notes_ref` argument is the canonical name of the reference to use,
    /// defaulting to "refs/notes/commits". If `force` is specified then
    /// previous notes are overwritten.
    pub fn note(
        &self,
        author: &Signature<'_>,
        committer: &Signature<'_>,
        notes_ref: Option<&str>,
        oid: Oid,
        note: &str,
        force: bool,
    ) -> Result<Oid, Error> {
        let notes_ref = crate::opt_cstr(notes_ref)?;
        let note = CString::new(note)?;
        let mut ret = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_note_create(
                &mut ret,
                self.raw,
                notes_ref,
                author.raw(),
                committer.raw(),
                oid.raw(),
                note,
                force
            ));
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
    pub fn notes(&self, notes_ref: Option<&str>) -> Result<Notes<'_>, Error> {
        let notes_ref = crate::opt_cstr(notes_ref)?;
        let mut ret = ptr::null_mut();
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
    pub fn find_note(&self, notes_ref: Option<&str>, id: Oid) -> Result<Note<'_>, Error> {
        let notes_ref = crate::opt_cstr(notes_ref)?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_note_read(&mut ret, self.raw, notes_ref, id.raw()));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Remove the note for an object.
    ///
    /// The `notes_ref` argument is the canonical name of the reference to use,
    /// defaulting to "refs/notes/commits".
    ///
    /// The id specified is the Oid of the git object to remove the note from.
    pub fn note_delete(
        &self,
        id: Oid,
        notes_ref: Option<&str>,
        author: &Signature<'_>,
        committer: &Signature<'_>,
    ) -> Result<(), Error> {
        let notes_ref = crate::opt_cstr(notes_ref)?;
        unsafe {
            try_call!(raw::git_note_remove(
                self.raw,
                notes_ref,
                author.raw(),
                committer.raw(),
                id.raw()
            ));
            Ok(())
        }
    }

    /// Create a revwalk that can be used to traverse the commit graph.
    pub fn revwalk(&self) -> Result<Revwalk<'_>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_revwalk_new(&mut raw, self.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the blame for a single file.
    pub fn blame_file(
        &self,
        path: &Path,
        opts: Option<&mut BlameOptions>,
    ) -> Result<Blame<'_>, Error> {
        let path = path_to_repo_path(path)?;
        let mut raw = ptr::null_mut();

        unsafe {
            try_call!(raw::git_blame_file(
                &mut raw,
                self.raw(),
                path,
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Find a merge base between two commits
    pub fn merge_base(&self, one: Oid, two: Oid) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_merge_base(
                &mut raw,
                self.raw,
                one.raw(),
                two.raw()
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Find a merge base given a list of commits
    pub fn merge_base_many(&self, oids: &[Oid]) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };

        unsafe {
            try_call!(raw::git_merge_base_many(
                &mut raw,
                self.raw,
                oids.len() as size_t,
                oids.as_ptr() as *const raw::git_oid
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Find all merge bases between two commits
    pub fn merge_bases(&self, one: Oid, two: Oid) -> Result<OidArray, Error> {
        let mut arr = raw::git_oidarray {
            ids: ptr::null_mut(),
            count: 0,
        };
        unsafe {
            try_call!(raw::git_merge_bases(
                &mut arr,
                self.raw,
                one.raw(),
                two.raw()
            ));
            Ok(Binding::from_raw(arr))
        }
    }

    /// Find all merge bases given a list of commits
    pub fn merge_bases_many(&self, oids: &[Oid]) -> Result<OidArray, Error> {
        let mut arr = raw::git_oidarray {
            ids: ptr::null_mut(),
            count: 0,
        };
        unsafe {
            try_call!(raw::git_merge_bases_many(
                &mut arr,
                self.raw,
                oids.len() as size_t,
                oids.as_ptr() as *const raw::git_oid
            ));
            Ok(Binding::from_raw(arr))
        }
    }

    /// Count the number of unique commits between two commit objects
    ///
    /// There is no need for branches containing the commits to have any
    /// upstream relationship, but it helps to think of one as a branch and the
    /// other as its upstream, the ahead and behind values will be what git
    /// would report for the branches.
    pub fn graph_ahead_behind(&self, local: Oid, upstream: Oid) -> Result<(usize, usize), Error> {
        unsafe {
            let mut ahead: size_t = 0;
            let mut behind: size_t = 0;
            try_call!(raw::git_graph_ahead_behind(
                &mut ahead,
                &mut behind,
                self.raw(),
                local.raw(),
                upstream.raw()
            ));
            Ok((ahead as usize, behind as usize))
        }
    }

    /// Determine if a commit is the descendant of another commit
    ///
    /// Note that a commit is not considered a descendant of itself, in contrast
    /// to `git merge-base --is-ancestor`.
    pub fn graph_descendant_of(&self, commit: Oid, ancestor: Oid) -> Result<bool, Error> {
        unsafe {
            let rv = try_call!(raw::git_graph_descendant_of(
                self.raw(),
                commit.raw(),
                ancestor.raw()
            ));
            Ok(rv != 0)
        }
    }

    /// Read the reflog for the given reference
    ///
    /// If there is no reflog file for the given reference yet, an empty reflog
    /// object will be returned.
    pub fn reflog(&self, name: &str) -> Result<Reflog, Error> {
        let name = CString::new(name)?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reflog_read(&mut ret, self.raw, name));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Delete the reflog for the given reference
    pub fn reflog_delete(&self, name: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_reflog_delete(self.raw, name));
        }
        Ok(())
    }

    /// Rename a reflog
    ///
    /// The reflog to be renamed is expected to already exist.
    pub fn reflog_rename(&self, old_name: &str, new_name: &str) -> Result<(), Error> {
        let old_name = CString::new(old_name)?;
        let new_name = CString::new(new_name)?;
        unsafe {
            try_call!(raw::git_reflog_rename(self.raw, old_name, new_name));
        }
        Ok(())
    }

    /// Check if the given reference has a reflog.
    pub fn reference_has_log(&self, name: &str) -> Result<bool, Error> {
        let name = CString::new(name)?;
        let ret = unsafe { try_call!(raw::git_reference_has_log(self.raw, name)) };
        Ok(ret != 0)
    }

    /// Ensure that the given reference has a reflog.
    pub fn reference_ensure_log(&self, name: &str) -> Result<(), Error> {
        let name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_reference_ensure_log(self.raw, name));
        }
        Ok(())
    }

    /// Describes a commit
    ///
    /// Performs a describe operation on the current commit and the worktree.
    /// After performing a describe on HEAD, a status is run and description is
    /// considered to be dirty if there are.
    pub fn describe(&self, opts: &DescribeOptions) -> Result<Describe<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_describe_workdir(&mut ret, self.raw, opts.raw()));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Directly run a diff on two blobs.
    ///
    /// Compared to a file, a blob lacks some contextual information. As such, the
    /// `DiffFile` given to the callback will have some fake data; i.e. mode will be
    /// 0 and path will be `None`.
    ///
    /// `None` is allowed for either `old_blob` or `new_blob` and will be treated
    /// as an empty blob, with the oid set to zero in the `DiffFile`. Passing `None`
    /// for both blobs is a noop; no callbacks will be made at all.
    ///
    /// We do run a binary content check on the blob content and if either blob looks
    /// like binary data, the `DiffFile` binary attribute will be set to 1 and no call to
    /// the `hunk_cb` nor `line_cb` will be made (unless you set the `force_text`
    /// option).
    pub fn diff_blobs(
        &self,
        old_blob: Option<&Blob<'_>>,
        old_as_path: Option<&str>,
        new_blob: Option<&Blob<'_>>,
        new_as_path: Option<&str>,
        opts: Option<&mut DiffOptions>,
        file_cb: Option<&mut FileCb<'_>>,
        binary_cb: Option<&mut BinaryCb<'_>>,
        hunk_cb: Option<&mut HunkCb<'_>>,
        line_cb: Option<&mut LineCb<'_>>,
    ) -> Result<(), Error> {
        let old_as_path = crate::opt_cstr(old_as_path)?;
        let new_as_path = crate::opt_cstr(new_as_path)?;
        let mut cbs = DiffCallbacks {
            file: file_cb,
            binary: binary_cb,
            hunk: hunk_cb,
            line: line_cb,
        };
        let ptr = &mut cbs as *mut _;
        unsafe {
            let file_cb_c: raw::git_diff_file_cb = if cbs.file.is_some() {
                Some(file_cb_c)
            } else {
                None
            };
            let binary_cb_c: raw::git_diff_binary_cb = if cbs.binary.is_some() {
                Some(binary_cb_c)
            } else {
                None
            };
            let hunk_cb_c: raw::git_diff_hunk_cb = if cbs.hunk.is_some() {
                Some(hunk_cb_c)
            } else {
                None
            };
            let line_cb_c: raw::git_diff_line_cb = if cbs.line.is_some() {
                Some(line_cb_c)
            } else {
                None
            };
            try_call!(raw::git_diff_blobs(
                old_blob.map(|s| s.raw()),
                old_as_path,
                new_blob.map(|s| s.raw()),
                new_as_path,
                opts.map(|s| s.raw()),
                file_cb_c,
                binary_cb_c,
                hunk_cb_c,
                line_cb_c,
                ptr as *mut _
            ));
            Ok(())
        }
    }

    /// Create a diff with the difference between two tree objects.
    ///
    /// This is equivalent to `git diff <old-tree> <new-tree>`
    ///
    /// The first tree will be used for the "old_file" side of the delta and the
    /// second tree will be used for the "new_file" side of the delta.  You can
    /// pass `None` to indicate an empty tree, although it is an error to pass
    /// `None` for both the `old_tree` and `new_tree`.
    pub fn diff_tree_to_tree(
        &self,
        old_tree: Option<&Tree<'_>>,
        new_tree: Option<&Tree<'_>>,
        opts: Option<&mut DiffOptions>,
    ) -> Result<Diff<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_tree_to_tree(
                &mut ret,
                self.raw(),
                old_tree.map(|s| s.raw()),
                new_tree.map(|s| s.raw()),
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a diff between a tree and repository index.
    ///
    /// This is equivalent to `git diff --cached <treeish>` or if you pass
    /// the HEAD tree, then like `git diff --cached`.
    ///
    /// The tree you pass will be used for the "old_file" side of the delta, and
    /// the index will be used for the "new_file" side of the delta.
    ///
    /// If you pass `None` for the index, then the existing index of the `repo`
    /// will be used.  In this case, the index will be refreshed from disk
    /// (if it has changed) before the diff is generated.
    ///
    /// If the tree is `None`, then it is considered an empty tree.
    pub fn diff_tree_to_index(
        &self,
        old_tree: Option<&Tree<'_>>,
        index: Option<&Index>,
        opts: Option<&mut DiffOptions>,
    ) -> Result<Diff<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_tree_to_index(
                &mut ret,
                self.raw(),
                old_tree.map(|s| s.raw()),
                index.map(|s| s.raw()),
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a diff between two index objects.
    ///
    /// The first index will be used for the "old_file" side of the delta, and
    /// the second index will be used for the "new_file" side of the delta.
    pub fn diff_index_to_index(
        &self,
        old_index: &Index,
        new_index: &Index,
        opts: Option<&mut DiffOptions>,
    ) -> Result<Diff<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_index_to_index(
                &mut ret,
                self.raw(),
                old_index.raw(),
                new_index.raw(),
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a diff between the repository index and the workdir directory.
    ///
    /// This matches the `git diff` command.  See the note below on
    /// `tree_to_workdir` for a discussion of the difference between
    /// `git diff` and `git diff HEAD` and how to emulate a `git diff <treeish>`
    /// using libgit2.
    ///
    /// The index will be used for the "old_file" side of the delta, and the
    /// working directory will be used for the "new_file" side of the delta.
    ///
    /// If you pass `None` for the index, then the existing index of the `repo`
    /// will be used.  In this case, the index will be refreshed from disk
    /// (if it has changed) before the diff is generated.
    pub fn diff_index_to_workdir(
        &self,
        index: Option<&Index>,
        opts: Option<&mut DiffOptions>,
    ) -> Result<Diff<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_index_to_workdir(
                &mut ret,
                self.raw(),
                index.map(|s| s.raw()),
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a diff between a tree and the working directory.
    ///
    /// The tree you provide will be used for the "old_file" side of the delta,
    /// and the working directory will be used for the "new_file" side.
    ///
    /// This is not the same as `git diff <treeish>` or `git diff-index
    /// <treeish>`.  Those commands use information from the index, whereas this
    /// function strictly returns the differences between the tree and the files
    /// in the working directory, regardless of the state of the index.  Use
    /// `tree_to_workdir_with_index` to emulate those commands.
    ///
    /// To see difference between this and `tree_to_workdir_with_index`,
    /// consider the example of a staged file deletion where the file has then
    /// been put back into the working dir and further modified.  The
    /// tree-to-workdir diff for that file is 'modified', but `git diff` would
    /// show status 'deleted' since there is a staged delete.
    ///
    /// If `None` is passed for `tree`, then an empty tree is used.
    pub fn diff_tree_to_workdir(
        &self,
        old_tree: Option<&Tree<'_>>,
        opts: Option<&mut DiffOptions>,
    ) -> Result<Diff<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_tree_to_workdir(
                &mut ret,
                self.raw(),
                old_tree.map(|s| s.raw()),
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a diff between a tree and the working directory using index data
    /// to account for staged deletes, tracked files, etc.
    ///
    /// This emulates `git diff <tree>` by diffing the tree to the index and
    /// the index to the working directory and blending the results into a
    /// single diff that includes staged deleted, etc.
    pub fn diff_tree_to_workdir_with_index(
        &self,
        old_tree: Option<&Tree<'_>>,
        opts: Option<&mut DiffOptions>,
    ) -> Result<Diff<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_tree_to_workdir_with_index(
                &mut ret,
                self.raw(),
                old_tree.map(|s| s.raw()),
                opts.map(|s| s.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Create a PackBuilder
    pub fn packbuilder(&self) -> Result<PackBuilder<'_>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_packbuilder_new(&mut ret, self.raw()));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Save the local modifications to a new stash.
    pub fn stash_save(
        &mut self,
        stasher: &Signature<'_>,
        message: &str,
        flags: Option<StashFlags>,
    ) -> Result<Oid, Error> {
        self.stash_save2(stasher, Some(message), flags)
    }

    /// Save the local modifications to a new stash.
    /// unlike `stash_save` it allows to pass a null `message`
    pub fn stash_save2(
        &mut self,
        stasher: &Signature<'_>,
        message: Option<&str>,
        flags: Option<StashFlags>,
    ) -> Result<Oid, Error> {
        unsafe {
            let mut raw_oid = raw::git_oid {
                id: [0; raw::GIT_OID_RAWSZ],
            };
            let message = crate::opt_cstr(message)?;
            let flags = flags.unwrap_or_else(StashFlags::empty);
            try_call!(raw::git_stash_save(
                &mut raw_oid,
                self.raw(),
                stasher.raw(),
                message,
                flags.bits() as c_uint
            ));
            Ok(Binding::from_raw(&raw_oid as *const _))
        }
    }

    /// Like `stash_save` but with more options like selective statshing via path patterns.
    pub fn stash_save_ext(
        &mut self,
        opts: Option<&mut StashSaveOptions<'_>>,
    ) -> Result<Oid, Error> {
        unsafe {
            let mut raw_oid = raw::git_oid {
                id: [0; raw::GIT_OID_RAWSZ],
            };
            let opts = opts.map(|opts| opts.raw());
            try_call!(raw::git_stash_save_with_opts(
                &mut raw_oid,
                self.raw(),
                opts
            ));
            Ok(Binding::from_raw(&raw_oid as *const _))
        }
    }

    /// Apply a single stashed state from the stash list.
    pub fn stash_apply(
        &mut self,
        index: usize,
        opts: Option<&mut StashApplyOptions<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let opts = opts.map(|opts| opts.raw());
            try_call!(raw::git_stash_apply(self.raw(), index, opts));
            Ok(())
        }
    }

    /// Loop over all the stashed states and issue a callback for each one.
    ///
    /// Return `true` to continue iterating or `false` to stop.
    pub fn stash_foreach<C>(&mut self, mut callback: C) -> Result<(), Error>
    where
        C: FnMut(usize, &str, &Oid) -> bool,
    {
        unsafe {
            let mut data = StashCbData {
                callback: &mut callback,
            };
            let cb: raw::git_stash_cb = Some(stash_cb);
            try_call!(raw::git_stash_foreach(
                self.raw(),
                cb,
                &mut data as *mut _ as *mut _
            ));
            Ok(())
        }
    }

    /// Remove a single stashed state from the stash list.
    pub fn stash_drop(&mut self, index: usize) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_stash_drop(self.raw(), index));
            Ok(())
        }
    }

    /// Apply a single stashed state from the stash list and remove it from the list if successful.
    pub fn stash_pop(
        &mut self,
        index: usize,
        opts: Option<&mut StashApplyOptions<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            let opts = opts.map(|opts| opts.raw());
            try_call!(raw::git_stash_pop(self.raw(), index, opts));
            Ok(())
        }
    }

    /// Add ignore rules for a repository.
    ///
    /// The format of the rules is the same one of the .gitignore file.
    pub fn add_ignore_rule(&self, rules: &str) -> Result<(), Error> {
        let rules = CString::new(rules)?;
        unsafe {
            try_call!(raw::git_ignore_add_rule(self.raw, rules));
        }
        Ok(())
    }

    /// Clear ignore rules that were explicitly added.
    pub fn clear_ignore_rules(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_ignore_clear_internal_rules(self.raw));
        }
        Ok(())
    }

    /// Test if the ignore rules apply to a given path.
    pub fn is_path_ignored<P: AsRef<Path>>(&self, path: P) -> Result<bool, Error> {
        let path = util::cstring_to_repo_path(path.as_ref())?;
        let mut ignored: c_int = 0;
        unsafe {
            try_call!(raw::git_ignore_path_is_ignored(
                &mut ignored,
                self.raw,
                path
            ));
        }
        Ok(ignored == 1)
    }

    /// Perform a cherrypick
    pub fn cherrypick(
        &self,
        commit: &Commit<'_>,
        options: Option<&mut CherrypickOptions<'_>>,
    ) -> Result<(), Error> {
        let raw_opts = options.map(|o| o.raw());
        let ptr_raw_opts = match raw_opts.as_ref() {
            Some(v) => v,
            None => std::ptr::null(),
        };
        unsafe {
            try_call!(raw::git_cherrypick(self.raw(), commit.raw(), ptr_raw_opts));

            Ok(())
        }
    }

    /// Create an index of uncommitted changes, representing the result of
    /// cherry-picking.
    pub fn cherrypick_commit(
        &self,
        cherrypick_commit: &Commit<'_>,
        our_commit: &Commit<'_>,
        mainline: u32,
        options: Option<&MergeOptions>,
    ) -> Result<Index, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_cherrypick_commit(
                &mut ret,
                self.raw(),
                cherrypick_commit.raw(),
                our_commit.raw(),
                mainline,
                options.map(|o| o.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Find the remote name of a remote-tracking branch
    pub fn branch_remote_name(&self, refname: &str) -> Result<Buf, Error> {
        let refname = CString::new(refname)?;
        unsafe {
            let buf = Buf::new();
            try_call!(raw::git_branch_remote_name(buf.raw(), self.raw, refname));
            Ok(buf)
        }
    }

    /// Retrieves the name of the reference supporting the remote tracking branch,
    /// given the name of a local branch reference.
    pub fn branch_upstream_name(&self, refname: &str) -> Result<Buf, Error> {
        let refname = CString::new(refname)?;
        unsafe {
            let buf = Buf::new();
            try_call!(raw::git_branch_upstream_name(buf.raw(), self.raw, refname));
            Ok(buf)
        }
    }

    /// Retrieve the name of the upstream remote of a local branch.
    ///
    /// `refname` must be in the form `refs/heads/{branch_name}`
    pub fn branch_upstream_remote(&self, refname: &str) -> Result<Buf, Error> {
        let refname = CString::new(refname)?;
        unsafe {
            let buf = Buf::new();
            try_call!(raw::git_branch_upstream_remote(
                buf.raw(),
                self.raw,
                refname
            ));
            Ok(buf)
        }
    }

    /// Apply a Diff to the given repo, making changes directly in the working directory, the index, or both.
    pub fn apply(
        &self,
        diff: &Diff<'_>,
        location: ApplyLocation,
        options: Option<&mut ApplyOptions<'_>>,
    ) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_apply(
                self.raw,
                diff.raw(),
                location.raw(),
                options.map(|s| s.raw()).unwrap_or(ptr::null())
            ));

            Ok(())
        }
    }

    /// Apply a Diff to the provided tree, and return the resulting Index.
    pub fn apply_to_tree(
        &self,
        tree: &Tree<'_>,
        diff: &Diff<'_>,
        options: Option<&mut ApplyOptions<'_>>,
    ) -> Result<Index, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_apply_to_tree(
                &mut ret,
                self.raw,
                tree.raw(),
                diff.raw(),
                options.map(|s| s.raw()).unwrap_or(ptr::null())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Reverts the given commit, producing changes in the index and working directory.
    pub fn revert(
        &self,
        commit: &Commit<'_>,
        options: Option<&mut RevertOptions<'_>>,
    ) -> Result<(), Error> {
        let raw_opts = options.map(|o| o.raw());
        let ptr_raw_opts = match raw_opts.as_ref() {
            Some(v) => v,
            None => 0 as *const _,
        };
        unsafe {
            try_call!(raw::git_revert(self.raw(), commit.raw(), ptr_raw_opts));
            Ok(())
        }
    }

    /// Reverts the given commit against the given "our" commit,
    /// producing an index that reflects the result of the revert.
    pub fn revert_commit(
        &self,
        revert_commit: &Commit<'_>,
        our_commit: &Commit<'_>,
        mainline: u32,
        options: Option<&MergeOptions>,
    ) -> Result<Index, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_revert_commit(
                &mut ret,
                self.raw(),
                revert_commit.raw(),
                our_commit.raw(),
                mainline,
                options.map(|o| o.raw())
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Lists all the worktrees for the repository
    pub fn worktrees(&self) -> Result<StringArray, Error> {
        let mut arr = raw::git_strarray {
            strings: ptr::null_mut(),
            count: 0,
        };
        unsafe {
            try_call!(raw::git_worktree_list(&mut arr, self.raw));
            Ok(Binding::from_raw(arr))
        }
    }

    /// Opens a worktree by name for the given repository
    ///
    /// This can open any worktree that the worktrees method returns.
    pub fn find_worktree(&self, name: &str) -> Result<Worktree, Error> {
        let mut raw = ptr::null_mut();
        let raw_name = CString::new(name)?;
        unsafe {
            try_call!(raw::git_worktree_lookup(&mut raw, self.raw, raw_name));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Creates a new worktree for the repository
    pub fn worktree<'a>(
        &'a self,
        name: &str,
        path: &Path,
        opts: Option<&WorktreeAddOptions<'a>>,
    ) -> Result<Worktree, Error> {
        let mut raw = ptr::null_mut();
        let raw_name = CString::new(name)?;
        let raw_path = path.into_c_string()?;

        unsafe {
            try_call!(raw::git_worktree_add(
                &mut raw,
                self.raw,
                raw_name,
                raw_path,
                opts.map(|o| o.raw())
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new transaction
    pub fn transaction<'a>(&'a self) -> Result<Transaction<'a>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_transaction_new(&mut raw, self.raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Gets this repository's mailmap.
    pub fn mailmap(&self) -> Result<Mailmap, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_mailmap_from_repository(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    ///  If a merge is in progress, invoke 'callback' for each commit ID in the
    ///  MERGE_HEAD file.
    pub fn mergehead_foreach<C>(&mut self, mut callback: C) -> Result<(), Error>
    where
        C: FnMut(&Oid) -> bool,
    {
        unsafe {
            let mut data = MergeheadForeachCbData {
                callback: &mut callback,
            };
            let cb: raw::git_repository_mergehead_foreach_cb = Some(mergehead_foreach_cb);
            try_call!(raw::git_repository_mergehead_foreach(
                self.raw(),
                cb,
                &mut data as *mut _ as *mut _
            ));
            Ok(())
        }
    }

    /// Invoke 'callback' for each entry in the given FETCH_HEAD file.
    ///
    /// `callback` will be called with with following arguments:
    ///
    /// - `&str`: the reference name
    /// - `&[u8]`: the remote URL
    /// - `&Oid`: the reference target OID
    /// - `bool`: was the reference the result of a merge
    pub fn fetchhead_foreach<C>(&self, mut callback: C) -> Result<(), Error>
    where
        C: FnMut(&str, &[u8], &Oid, bool) -> bool,
    {
        unsafe {
            let mut data = FetchheadForeachCbData {
                callback: &mut callback,
            };
            let cb: raw::git_repository_fetchhead_foreach_cb = Some(fetchhead_foreach_cb);
            try_call!(raw::git_repository_fetchhead_foreach(
                self.raw(),
                cb,
                &mut data as *mut _ as *mut _
            ));
            Ok(())
        }
    }
}

impl Binding for Repository {
    type Raw = *mut raw::git_repository;
    unsafe fn from_raw(ptr: *mut raw::git_repository) -> Repository {
        Repository { raw: ptr }
    }
    fn raw(&self) -> *mut raw::git_repository {
        self.raw
    }
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
            flags: raw::GIT_REPOSITORY_INIT_MKDIR as u32
                | raw::GIT_REPOSITORY_INIT_MKPATH as u32
                | raw::GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE as u32,
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

    /// Normally a '/.git/' will be appended to the repo path for non-bare repos
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

    /// Recursively make all components of the repo and workdir path as
    /// necessary.
    ///
    /// Defaults to true.
    pub fn mkpath(&mut self, enabled: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_MKPATH, enabled)
    }

    /// Set to one of the `RepositoryInit` constants, or a custom value.
    pub fn mode(&mut self, mode: RepositoryInitMode) -> &mut RepositoryInitOptions {
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
    pub fn external_template(&mut self, enabled: bool) -> &mut RepositoryInitOptions {
        self.flag(raw::GIT_REPOSITORY_INIT_EXTERNAL_TEMPLATE, enabled)
    }

    fn flag(
        &mut self,
        flag: raw::git_repository_init_flag_t,
        on: bool,
    ) -> &mut RepositoryInitOptions {
        if on {
            self.flags |= flag as u32;
        } else {
            self.flags &= !(flag as u32);
        }
        self
    }

    /// The path to the working directory.
    ///
    /// If this is a relative path it will be evaluated relative to the repo
    /// path. If this is not the "natural" working directory, a .git gitlink
    /// file will be created here linking to the repo path.
    pub fn workdir_path(&mut self, path: &Path) -> &mut RepositoryInitOptions {
        // Normal file path OK (does not need Windows conversion).
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
        // Normal file path OK (does not need Windows conversion).
        self.template_path = Some(path.into_c_string().unwrap());
        self
    }

    /// The name of the head to point HEAD at.
    ///
    /// If not configured, this will be taken from your git configuration.
    /// If this begins with `refs/` it will be used verbatim;
    /// otherwise `refs/heads/` will be prefixed
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
        assert_eq!(
            raw::git_repository_init_init_options(
                &mut opts,
                raw::GIT_REPOSITORY_INIT_OPTIONS_VERSION
            ),
            0
        );
        opts.flags = self.flags;
        opts.mode = self.mode;
        opts.workdir_path = crate::call::convert(&self.workdir_path);
        opts.description = crate::call::convert(&self.description);
        opts.template_path = crate::call::convert(&self.template_path);
        opts.initial_head = crate::call::convert(&self.initial_head);
        opts.origin_url = crate::call::convert(&self.origin_url);
        opts
    }
}

#[cfg(test)]
mod tests {
    use crate::build::CheckoutBuilder;
    use crate::CherrypickOptions;
    use crate::{
        ObjectType, Oid, Repository, ResetType, Signature, SubmoduleIgnore, SubmoduleUpdate,
    };
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn smoke_init() {
        let td = TempDir::new().unwrap();
        let path = td.path();

        let repo = Repository::init(path).unwrap();
        assert!(!repo.is_bare());
    }

    #[test]
    fn smoke_init_bare() {
        let td = TempDir::new().unwrap();
        let path = td.path();

        let repo = Repository::init_bare(path).unwrap();
        assert!(repo.is_bare());
        assert!(repo.namespace().is_none());
    }

    #[test]
    fn smoke_open() {
        let td = TempDir::new().unwrap();
        let path = td.path();
        Repository::init(td.path()).unwrap();
        let repo = Repository::open(path).unwrap();
        assert!(!repo.is_bare());
        assert!(!repo.is_shallow());
        assert!(repo.is_empty().unwrap());
        assert_eq!(
            crate::test::realpath(&repo.path()).unwrap(),
            crate::test::realpath(&td.path().join(".git/")).unwrap()
        );
        assert_eq!(repo.state(), crate::RepositoryState::Clean);
    }

    #[test]
    fn smoke_open_bare() {
        let td = TempDir::new().unwrap();
        let path = td.path();
        Repository::init_bare(td.path()).unwrap();

        let repo = Repository::open(path).unwrap();
        assert!(repo.is_bare());
        assert_eq!(
            crate::test::realpath(&repo.path()).unwrap(),
            crate::test::realpath(&td.path().join("")).unwrap()
        );
    }

    #[test]
    fn smoke_checkout() {
        let (_td, repo) = crate::test::repo_init();
        repo.checkout_head(None).unwrap();
    }

    #[test]
    fn smoke_revparse() {
        let (_td, repo) = crate::test::repo_init();
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
        let td = TempDir::new().unwrap();
        Repository::init(&td.path().join("a/b/c/d")).unwrap();
    }

    #[test]
    fn smoke_discover() {
        let td = TempDir::new().unwrap();
        let subdir = td.path().join("subdi");
        fs::create_dir(&subdir).unwrap();
        Repository::init_bare(td.path()).unwrap();
        let repo = Repository::discover(&subdir).unwrap();
        assert_eq!(
            crate::test::realpath(&repo.path()).unwrap(),
            crate::test::realpath(&td.path().join("")).unwrap()
        );
    }

    #[test]
    fn smoke_discover_path() {
        let td = TempDir::new().unwrap();
        let subdir = td.path().join("subdi");
        fs::create_dir(&subdir).unwrap();
        Repository::init_bare(td.path()).unwrap();
        let path = Repository::discover_path(&subdir, &[] as &[&OsStr]).unwrap();
        assert_eq!(
            crate::test::realpath(&path).unwrap(),
            crate::test::realpath(&td.path().join("")).unwrap()
        );
    }

    #[test]
    fn smoke_discover_path_ceiling_dir() {
        let td = TempDir::new().unwrap();
        let subdir = td.path().join("subdi");
        fs::create_dir(&subdir).unwrap();
        let ceilingdir = subdir.join("ceiling");
        fs::create_dir(&ceilingdir).unwrap();
        let testdir = ceilingdir.join("testdi");
        fs::create_dir(&testdir).unwrap();
        Repository::init_bare(td.path()).unwrap();
        let path = Repository::discover_path(&testdir, &[ceilingdir.as_os_str()]);

        assert!(path.is_err());
    }

    #[test]
    fn smoke_open_ext() {
        let td = TempDir::new().unwrap();
        let subdir = td.path().join("subdir");
        fs::create_dir(&subdir).unwrap();
        Repository::init(td.path()).unwrap();

        let repo = Repository::open_ext(
            &subdir,
            crate::RepositoryOpenFlags::empty(),
            &[] as &[&OsStr],
        )
        .unwrap();
        assert!(!repo.is_bare());
        assert_eq!(
            crate::test::realpath(&repo.path()).unwrap(),
            crate::test::realpath(&td.path().join(".git")).unwrap()
        );

        let repo =
            Repository::open_ext(&subdir, crate::RepositoryOpenFlags::BARE, &[] as &[&OsStr])
                .unwrap();
        assert!(repo.is_bare());
        assert_eq!(
            crate::test::realpath(&repo.path()).unwrap(),
            crate::test::realpath(&td.path().join(".git")).unwrap()
        );

        let err = Repository::open_ext(
            &subdir,
            crate::RepositoryOpenFlags::NO_SEARCH,
            &[] as &[&OsStr],
        )
        .err()
        .unwrap();
        assert_eq!(err.code(), crate::ErrorCode::NotFound);

        assert!(
            Repository::open_ext(&subdir, crate::RepositoryOpenFlags::empty(), &[&subdir]).is_ok()
        );
    }

    fn graph_repo_init() -> (TempDir, Repository) {
        let (_td, repo) = crate::test::repo_init();
        {
            let head = repo.head().unwrap().target().unwrap();
            let head = repo.find_commit(head).unwrap();

            let mut index = repo.index().unwrap();
            let id = index.write_tree().unwrap();

            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "second", &tree, &[&head])
                .unwrap();
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
        let (ahead, behind) = repo.graph_ahead_behind(head_id, head_parent_id).unwrap();
        assert_eq!(ahead, 1);
        assert_eq!(behind, 0);
        let (ahead, behind) = repo.graph_ahead_behind(head_parent_id, head_id).unwrap();
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
    fn smoke_reference_has_log_ensure_log() {
        let (_td, repo) = crate::test::repo_init();

        assert_eq!(repo.reference_has_log("HEAD").unwrap(), true);
        assert_eq!(repo.reference_has_log("refs/heads/main").unwrap(), true);
        assert_eq!(repo.reference_has_log("NOT_HEAD").unwrap(), false);
        let main_oid = repo.revparse_single("main").unwrap().id();
        assert!(repo
            .reference("NOT_HEAD", main_oid, false, "creating a new branch")
            .is_ok());
        assert_eq!(repo.reference_has_log("NOT_HEAD").unwrap(), false);
        assert!(repo.reference_ensure_log("NOT_HEAD").is_ok());
        assert_eq!(repo.reference_has_log("NOT_HEAD").unwrap(), true);
    }

    #[test]
    fn smoke_set_head() {
        let (_td, repo) = crate::test::repo_init();

        assert!(repo.set_head("refs/heads/does-not-exist").is_ok());
        assert!(repo.head().is_err());

        assert!(repo.set_head("refs/heads/main").is_ok());
        assert!(repo.head().is_ok());

        assert!(repo.set_head("*").is_err());
    }

    #[test]
    fn smoke_set_head_bytes() {
        let (_td, repo) = crate::test::repo_init();

        assert!(repo.set_head_bytes(b"refs/heads/does-not-exist").is_ok());
        assert!(repo.head().is_err());

        assert!(repo.set_head_bytes(b"refs/heads/main").is_ok());
        assert!(repo.head().is_ok());

        assert!(repo.set_head_bytes(b"*").is_err());
    }

    #[test]
    fn smoke_set_head_detached() {
        let (_td, repo) = crate::test::repo_init();

        let void_oid = Oid::from_bytes(b"00000000000000000000").unwrap();
        assert!(repo.set_head_detached(void_oid).is_err());

        let main_oid = repo.revparse_single("main").unwrap().id();
        assert!(repo.set_head_detached(main_oid).is_ok());
        assert_eq!(repo.head().unwrap().target().unwrap(), main_oid);
    }

    #[test]
    fn smoke_find_object_by_prefix() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap().target().unwrap();
        let head = repo.find_commit(head).unwrap();
        let head_id = head.id();
        let head_prefix = &head_id.to_string()[..7];
        let obj = repo.find_object_by_prefix(head_prefix, None).unwrap();
        assert_eq!(obj.id(), head_id);
    }

    /// create the following:
    ///    /---o4
    ///   /---o3
    /// o1---o2
    #[test]
    fn smoke_merge_base() {
        let (_td, repo) = graph_repo_init();
        let sig = repo.signature().unwrap();

        // let oid1 = head
        let oid1 = repo.head().unwrap().target().unwrap();
        let commit1 = repo.find_commit(oid1).unwrap();
        println!("created oid1 {:?}", oid1);

        repo.branch("branch_a", &commit1, true).unwrap();
        repo.branch("branch_b", &commit1, true).unwrap();
        repo.branch("branch_c", &commit1, true).unwrap();

        // create commit oid2 on branch_a
        let mut index = repo.index().unwrap();
        let p = Path::new(repo.workdir().unwrap()).join("file_a");
        println!("using path {:?}", p);
        fs::File::create(&p).unwrap();
        index.add_path(Path::new("file_a")).unwrap();
        let id_a = index.write_tree().unwrap();
        let tree_a = repo.find_tree(id_a).unwrap();
        let oid2 = repo
            .commit(
                Some("refs/heads/branch_a"),
                &sig,
                &sig,
                "commit 2",
                &tree_a,
                &[&commit1],
            )
            .unwrap();
        repo.find_commit(oid2).unwrap();
        println!("created oid2 {:?}", oid2);

        t!(repo.reset(commit1.as_object(), ResetType::Hard, None));

        // create commit oid3 on branch_b
        let mut index = repo.index().unwrap();
        let p = Path::new(repo.workdir().unwrap()).join("file_b");
        fs::File::create(&p).unwrap();
        index.add_path(Path::new("file_b")).unwrap();
        let id_b = index.write_tree().unwrap();
        let tree_b = repo.find_tree(id_b).unwrap();
        let oid3 = repo
            .commit(
                Some("refs/heads/branch_b"),
                &sig,
                &sig,
                "commit 3",
                &tree_b,
                &[&commit1],
            )
            .unwrap();
        repo.find_commit(oid3).unwrap();
        println!("created oid3 {:?}", oid3);

        t!(repo.reset(commit1.as_object(), ResetType::Hard, None));

        // create commit oid4 on branch_c
        let mut index = repo.index().unwrap();
        let p = Path::new(repo.workdir().unwrap()).join("file_c");
        fs::File::create(&p).unwrap();
        index.add_path(Path::new("file_c")).unwrap();
        let id_c = index.write_tree().unwrap();
        let tree_c = repo.find_tree(id_c).unwrap();
        let oid4 = repo
            .commit(
                Some("refs/heads/branch_c"),
                &sig,
                &sig,
                "commit 3",
                &tree_c,
                &[&commit1],
            )
            .unwrap();
        repo.find_commit(oid4).unwrap();
        println!("created oid4 {:?}", oid4);

        // the merge base of (oid2,oid3) should be oid1
        let merge_base = repo.merge_base(oid2, oid3).unwrap();
        assert_eq!(merge_base, oid1);

        // the merge base of (oid2,oid3,oid4) should be oid1
        let merge_base = repo.merge_base_many(&[oid2, oid3, oid4]).unwrap();
        assert_eq!(merge_base, oid1);
    }

    /// create an octopus:
    ///   /---o2-o4
    /// o1      X
    ///   \---o3-o5
    /// and checks that the merge bases of (o4,o5) are (o2,o3)
    #[test]
    fn smoke_merge_bases() {
        let (_td, repo) = graph_repo_init();
        let sig = repo.signature().unwrap();

        // let oid1 = head
        let oid1 = repo.head().unwrap().target().unwrap();
        let commit1 = repo.find_commit(oid1).unwrap();
        println!("created oid1 {:?}", oid1);

        repo.branch("branch_a", &commit1, true).unwrap();
        repo.branch("branch_b", &commit1, true).unwrap();

        // create commit oid2 on branchA
        let mut index = repo.index().unwrap();
        let p = Path::new(repo.workdir().unwrap()).join("file_a");
        println!("using path {:?}", p);
        fs::File::create(&p).unwrap();
        index.add_path(Path::new("file_a")).unwrap();
        let id_a = index.write_tree().unwrap();
        let tree_a = repo.find_tree(id_a).unwrap();
        let oid2 = repo
            .commit(
                Some("refs/heads/branch_a"),
                &sig,
                &sig,
                "commit 2",
                &tree_a,
                &[&commit1],
            )
            .unwrap();
        let commit2 = repo.find_commit(oid2).unwrap();
        println!("created oid2 {:?}", oid2);

        t!(repo.reset(commit1.as_object(), ResetType::Hard, None));

        // create commit oid3 on branchB
        let mut index = repo.index().unwrap();
        let p = Path::new(repo.workdir().unwrap()).join("file_b");
        fs::File::create(&p).unwrap();
        index.add_path(Path::new("file_b")).unwrap();
        let id_b = index.write_tree().unwrap();
        let tree_b = repo.find_tree(id_b).unwrap();
        let oid3 = repo
            .commit(
                Some("refs/heads/branch_b"),
                &sig,
                &sig,
                "commit 3",
                &tree_b,
                &[&commit1],
            )
            .unwrap();
        let commit3 = repo.find_commit(oid3).unwrap();
        println!("created oid3 {:?}", oid3);

        // create merge commit oid4 on branchA with parents oid2 and oid3
        //let mut index4 = repo.merge_commits(&commit2, &commit3, None).unwrap();
        repo.set_head("refs/heads/branch_a").unwrap();
        repo.checkout_head(None).unwrap();
        let oid4 = repo
            .commit(
                Some("refs/heads/branch_a"),
                &sig,
                &sig,
                "commit 4",
                &tree_a,
                &[&commit2, &commit3],
            )
            .unwrap();
        //index4.write_tree_to(&repo).unwrap();
        println!("created oid4 {:?}", oid4);

        // create merge commit oid5 on branchB with parents oid2 and oid3
        //let mut index5 = repo.merge_commits(&commit3, &commit2, None).unwrap();
        repo.set_head("refs/heads/branch_b").unwrap();
        repo.checkout_head(None).unwrap();
        let oid5 = repo
            .commit(
                Some("refs/heads/branch_b"),
                &sig,
                &sig,
                "commit 5",
                &tree_a,
                &[&commit3, &commit2],
            )
            .unwrap();
        //index5.write_tree_to(&repo).unwrap();
        println!("created oid5 {:?}", oid5);

        // merge bases of (oid4,oid5) should be (oid2,oid3)
        let merge_bases = repo.merge_bases(oid4, oid5).unwrap();
        let mut found_oid2 = false;
        let mut found_oid3 = false;
        for mg in merge_bases.iter() {
            println!("found merge base {:?}", mg);
            if mg == &oid2 {
                found_oid2 = true;
            } else if mg == &oid3 {
                found_oid3 = true;
            } else {
                assert!(false);
            }
        }
        assert!(found_oid2);
        assert!(found_oid3);
        assert_eq!(merge_bases.len(), 2);

        // merge bases of (oid4,oid5) should be (oid2,oid3)
        let merge_bases = repo.merge_bases_many(&[oid4, oid5]).unwrap();
        let mut found_oid2 = false;
        let mut found_oid3 = false;
        for mg in merge_bases.iter() {
            println!("found merge base {:?}", mg);
            if mg == &oid2 {
                found_oid2 = true;
            } else if mg == &oid3 {
                found_oid3 = true;
            } else {
                assert!(false);
            }
        }
        assert!(found_oid2);
        assert!(found_oid3);
        assert_eq!(merge_bases.len(), 2);
    }

    #[test]
    fn smoke_revparse_ext() {
        let (_td, repo) = graph_repo_init();

        {
            let short_refname = "main";
            let expected_refname = "refs/heads/main";
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

    #[test]
    fn smoke_is_path_ignored() {
        let (_td, repo) = graph_repo_init();

        assert!(!repo.is_path_ignored(Path::new("foo")).unwrap());

        let _ = repo.add_ignore_rule("/foo");
        assert!(repo.is_path_ignored(Path::new("foo")).unwrap());
        if cfg!(windows) {
            assert!(repo.is_path_ignored(Path::new("foo\\thing")).unwrap());
        }

        let _ = repo.clear_ignore_rules();
        assert!(!repo.is_path_ignored(Path::new("foo")).unwrap());
        if cfg!(windows) {
            assert!(!repo.is_path_ignored(Path::new("foo\\thing")).unwrap());
        }
    }

    #[test]
    fn smoke_cherrypick() {
        let (_td, repo) = crate::test::repo_init();
        let sig = repo.signature().unwrap();

        let oid1 = repo.head().unwrap().target().unwrap();
        let commit1 = repo.find_commit(oid1).unwrap();

        repo.branch("branch_a", &commit1, true).unwrap();

        // Add 2 commits on top of the initial one in branch_a
        let mut index = repo.index().unwrap();
        let p1 = Path::new(repo.workdir().unwrap()).join("file_c");
        fs::File::create(&p1).unwrap();
        index.add_path(Path::new("file_c")).unwrap();
        let id = index.write_tree().unwrap();
        let tree_c = repo.find_tree(id).unwrap();
        let oid2 = repo
            .commit(
                Some("refs/heads/branch_a"),
                &sig,
                &sig,
                "commit 2",
                &tree_c,
                &[&commit1],
            )
            .unwrap();
        let commit2 = repo.find_commit(oid2).unwrap();
        println!("created oid2 {:?}", oid2);
        assert!(p1.exists());

        let mut index = repo.index().unwrap();
        let p2 = Path::new(repo.workdir().unwrap()).join("file_d");
        fs::File::create(&p2).unwrap();
        index.add_path(Path::new("file_d")).unwrap();
        let id = index.write_tree().unwrap();
        let tree_d = repo.find_tree(id).unwrap();
        let oid3 = repo
            .commit(
                Some("refs/heads/branch_a"),
                &sig,
                &sig,
                "commit 3",
                &tree_d,
                &[&commit2],
            )
            .unwrap();
        let commit3 = repo.find_commit(oid3).unwrap();
        println!("created oid3 {:?}", oid3);
        assert!(p1.exists());
        assert!(p2.exists());

        // cherry-pick commit3 on top of commit1 in branch b
        repo.reset(commit1.as_object(), ResetType::Hard, None)
            .unwrap();
        let mut cherrypick_opts = CherrypickOptions::new();
        repo.cherrypick(&commit3, Some(&mut cherrypick_opts))
            .unwrap();
        let id = repo.index().unwrap().write_tree().unwrap();
        let tree_d = repo.find_tree(id).unwrap();
        let oid4 = repo
            .commit(Some("HEAD"), &sig, &sig, "commit 4", &tree_d, &[&commit1])
            .unwrap();
        let commit4 = repo.find_commit(oid4).unwrap();
        // should have file from commit3, but not the file from commit2
        assert_eq!(commit4.parent(0).unwrap().id(), commit1.id());
        assert!(!p1.exists());
        assert!(p2.exists());
    }

    #[test]
    fn smoke_revert() {
        let (_td, repo) = crate::test::repo_init();
        let foo_file = Path::new(repo.workdir().unwrap()).join("foo");
        assert!(!foo_file.exists());

        let (oid1, _id) = crate::test::commit(&repo);
        let commit1 = repo.find_commit(oid1).unwrap();
        t!(repo.reset(commit1.as_object(), ResetType::Hard, None));
        assert!(foo_file.exists());

        repo.revert(&commit1, None).unwrap();
        let id = repo.index().unwrap().write_tree().unwrap();
        let tree2 = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "commit 1", &tree2, &[&commit1])
            .unwrap();
        // reverting once removes `foo` file
        assert!(!foo_file.exists());

        let oid2 = repo.head().unwrap().target().unwrap();
        let commit2 = repo.find_commit(oid2).unwrap();
        repo.revert(&commit2, None).unwrap();
        let id = repo.index().unwrap().write_tree().unwrap();
        let tree3 = repo.find_tree(id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "commit 2", &tree3, &[&commit2])
            .unwrap();
        // reverting twice restores `foo` file
        assert!(foo_file.exists());
    }

    #[test]
    fn smoke_config_write_and_read() {
        let (td, repo) = crate::test::repo_init();

        let mut config = repo.config().unwrap();

        config.set_bool("commit.gpgsign", false).unwrap();

        let c = fs::read_to_string(td.path().join(".git").join("config")).unwrap();

        assert!(c.contains("[commit]"));
        assert!(c.contains("gpgsign = false"));

        let config = repo.config().unwrap();

        assert!(!config.get_bool("commit.gpgsign").unwrap());
    }

    #[test]
    fn smoke_merge_analysis_for_ref() -> Result<(), crate::Error> {
        let (_td, repo) = graph_repo_init();

        // Set up this repo state:
        // * second (their-branch)
        // * initial (HEAD -> main)
        //
        // We expect that their-branch can be fast-forward merged into main.

        // git checkout --detach HEAD
        let head_commit = repo.head()?.peel_to_commit()?;
        repo.set_head_detached(head_commit.id())?;

        // git branch their-branch HEAD
        let their_branch = repo.branch("their-branch", &head_commit, false)?;

        // git branch -f main HEAD~
        let mut parents_iter = head_commit.parents();
        let parent = parents_iter.next().unwrap();
        assert!(parents_iter.next().is_none());

        let main = repo.branch("main", &parent, true)?;

        // git checkout main
        repo.set_head(main.get().name().expect("should be utf-8"))?;

        let (merge_analysis, _merge_preference) = repo.merge_analysis_for_ref(
            main.get(),
            &[&repo.reference_to_annotated_commit(their_branch.get())?],
        )?;

        assert!(merge_analysis.contains(crate::MergeAnalysis::ANALYSIS_FASTFORWARD));

        Ok(())
    }

    #[test]
    fn smoke_submodule_set() -> Result<(), crate::Error> {
        let (td1, _repo) = crate::test::repo_init();
        let (td2, mut repo2) = crate::test::repo_init();
        let url = crate::test::path2url(td1.path());
        let name = "bar";
        {
            let mut s = repo2.submodule(&url, Path::new(name), true)?;
            fs::remove_dir_all(td2.path().join("bar")).unwrap();
            Repository::clone(&url, td2.path().join("bar"))?;
            s.add_to_index(false)?;
            s.add_finalize()?;
        }

        // update strategy
        repo2.submodule_set_update(name, SubmoduleUpdate::None)?;
        assert!(matches!(
            repo2.find_submodule(name)?.update_strategy(),
            SubmoduleUpdate::None
        ));
        repo2.submodule_set_update(name, SubmoduleUpdate::Rebase)?;
        assert!(matches!(
            repo2.find_submodule(name)?.update_strategy(),
            SubmoduleUpdate::Rebase
        ));

        // ignore rule
        repo2.submodule_set_ignore(name, SubmoduleIgnore::Untracked)?;
        assert!(matches!(
            repo2.find_submodule(name)?.ignore_rule(),
            SubmoduleIgnore::Untracked
        ));
        repo2.submodule_set_ignore(name, SubmoduleIgnore::Dirty)?;
        assert!(matches!(
            repo2.find_submodule(name)?.ignore_rule(),
            SubmoduleIgnore::Dirty
        ));

        // url
        repo2.submodule_set_url(name, "fake-url")?;
        assert_eq!(repo2.find_submodule(name)?.url(), Some("fake-url"));

        // branch
        repo2.submodule_set_branch(name, "fake-branch")?;
        assert_eq!(repo2.find_submodule(name)?.branch(), Some("fake-branch"));

        Ok(())
    }

    #[test]
    fn smoke_mailmap_from_repository() {
        let (_td, repo) = crate::test::repo_init();

        let commit = {
            let head = t!(repo.head()).target().unwrap();
            t!(repo.find_commit(head))
        };

        // This is our baseline for HEAD.
        let author = commit.author();
        let committer = commit.committer();
        assert_eq!(author.name(), Some("name"));
        assert_eq!(author.email(), Some("email"));
        assert_eq!(committer.name(), Some("name"));
        assert_eq!(committer.email(), Some("email"));

        // There is no .mailmap file in the test repo so all signature identities are equal.
        let mailmap = t!(repo.mailmap());
        let mailmapped_author = t!(commit.author_with_mailmap(&mailmap));
        let mailmapped_committer = t!(commit.committer_with_mailmap(&mailmap));
        assert_eq!(mailmapped_author.name(), author.name());
        assert_eq!(mailmapped_author.email(), author.email());
        assert_eq!(mailmapped_committer.name(), committer.name());
        assert_eq!(mailmapped_committer.email(), committer.email());

        let commit = {
            // - Add a .mailmap file to the repository.
            // - Commit with a signature identity different from the author's.
            // - Include entries for both author and committer to prove we call
            //   the right raw functions.
            let mailmap_file = Path::new(".mailmap");
            let p = Path::new(repo.workdir().unwrap()).join(&mailmap_file);
            t!(fs::write(
                p,
                r#"
Author Name <author.proper@email> name <email>
Committer Name <committer.proper@email> <committer@email>"#,
            ));
            let mut index = t!(repo.index());
            t!(index.add_path(&mailmap_file));
            let id_mailmap = t!(index.write_tree());
            let tree_mailmap = t!(repo.find_tree(id_mailmap));

            let head = t!(repo.commit(
                Some("HEAD"),
                &author,
                t!(&Signature::now("committer", "committer@email")),
                "Add mailmap",
                &tree_mailmap,
                &[&commit],
            ));
            t!(repo.find_commit(head))
        };

        // Sanity check that we're working with the right commit and that its
        // author and committer identities differ.
        let author = commit.author();
        let committer = commit.committer();
        assert_ne!(author.name(), committer.name());
        assert_ne!(author.email(), committer.email());
        assert_eq!(author.name(), Some("name"));
        assert_eq!(author.email(), Some("email"));
        assert_eq!(committer.name(), Some("committer"));
        assert_eq!(committer.email(), Some("committer@email"));

        // Fetch the newly added .mailmap from the repository.
        let mailmap = t!(repo.mailmap());
        let mailmapped_author = t!(commit.author_with_mailmap(&mailmap));
        let mailmapped_committer = t!(commit.committer_with_mailmap(&mailmap));

        let mm_resolve_author = t!(mailmap.resolve_signature(&author));
        let mm_resolve_committer = t!(mailmap.resolve_signature(&committer));

        // Mailmap Signature lifetime is independent of Commit lifetime.
        drop(author);
        drop(committer);
        drop(commit);

        // author_with_mailmap() + committer_with_mailmap() work
        assert_eq!(mailmapped_author.name(), Some("Author Name"));
        assert_eq!(mailmapped_author.email(), Some("author.proper@email"));
        assert_eq!(mailmapped_committer.name(), Some("Committer Name"));
        assert_eq!(mailmapped_committer.email(), Some("committer.proper@email"));

        // resolve_signature() works
        assert_eq!(mm_resolve_author.email(), mailmapped_author.email());
        assert_eq!(mm_resolve_committer.email(), mailmapped_committer.email());
    }

    #[test]
    fn smoke_find_tag_by_prefix() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let tag_oid = repo
            .tag(
                "tag",
                &repo
                    .find_object(head.peel_to_commit().unwrap().id(), None)
                    .unwrap(),
                &repo.signature().unwrap(),
                "message",
                false,
            )
            .unwrap();
        let tag = repo.find_tag(tag_oid).unwrap();
        let found_tag = repo
            .find_tag_by_prefix(&tag.id().to_string()[0..7])
            .unwrap();
        assert_eq!(tag.id(), found_tag.id());
    }
}
