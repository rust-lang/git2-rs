//! Builder-pattern objects for configuration various git operations.

use libc::{c_char, c_int, c_uint, c_void, size_t};
use std::ffi::{CStr, CString};
use std::mem;
use std::path::Path;
use std::ptr;

use crate::util::{self, Binding};
use crate::{panic, raw, Error, FetchOptions, IntoCString, Oid, Repository, Tree};
use crate::{CheckoutNotificationType, DiffFile, FileMode, Remote};

/// A builder struct which is used to build configuration for cloning a new git
/// repository.
///
/// # Example
///
/// Cloning using SSH:
///
/// ```no_run
/// use git2::{Cred, Error, RemoteCallbacks};
/// use std::env;
/// use std::path::Path;
///
///   // Prepare callbacks.
///   let mut callbacks = RemoteCallbacks::new();
///   callbacks.credentials(|_url, username_from_url, _allowed_types| {
///     Cred::ssh_key(
///       username_from_url.unwrap(),
///       None,
///       Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
///       None,
///     )
///   });
///
///   // Prepare fetch options.
///   let mut fo = git2::FetchOptions::new();
///   fo.remote_callbacks(callbacks);
///
///   // Prepare builder.
///   let mut builder = git2::build::RepoBuilder::new();
///   builder.fetch_options(fo);
///
///   // Clone the project.
///   builder.clone(
///     "git@github.com:rust-lang/git2-rs.git",
///     Path::new("/tmp/git2-rs"),
///   );
/// ```
pub struct RepoBuilder<'cb> {
    bare: bool,
    branch: Option<CString>,
    local: bool,
    hardlinks: bool,
    checkout: Option<CheckoutBuilder<'cb>>,
    fetch_opts: Option<FetchOptions<'cb>>,
    clone_local: Option<CloneLocal>,
    remote_create: Option<Box<RemoteCreate<'cb>>>,
}

/// Type of callback passed to `RepoBuilder::remote_create`.
///
/// The second and third arguments are the remote's name and the remote's URL.
pub type RemoteCreate<'cb> =
    dyn for<'a> FnMut(&'a Repository, &str, &str) -> Result<Remote<'a>, Error> + 'cb;

/// A builder struct for git tree updates.
///
/// Paths passed to `remove` and `upsert` can be multi-component paths, i.e. they
/// may contain slashes.
///
/// This is a higher-level tree update facility.  There is also [`TreeBuilder`]
/// which is lower-level (and operates only on one level of the tree at a time).
///
/// [`TreeBuilder`]: crate::TreeBuilder
pub struct TreeUpdateBuilder {
    updates: Vec<raw::git_tree_update>,
    paths: Vec<CString>,
}

/// A builder struct for configuring checkouts of a repository.
pub struct CheckoutBuilder<'cb> {
    their_label: Option<CString>,
    our_label: Option<CString>,
    ancestor_label: Option<CString>,
    target_dir: Option<CString>,
    paths: Vec<CString>,
    path_ptrs: Vec<*const c_char>,
    file_perm: Option<i32>,
    dir_perm: Option<i32>,
    disable_filters: bool,
    checkout_opts: u32,
    progress: Option<Box<Progress<'cb>>>,
    notify: Option<Box<Notify<'cb>>>,
    notify_flags: CheckoutNotificationType,
}

/// Checkout progress notification callback.
///
/// The first argument is the path for the notification, the next is the number
/// of completed steps so far, and the final is the total number of steps.
pub type Progress<'a> = dyn FnMut(Option<&Path>, usize, usize) + 'a;

/// Checkout notifications callback.
///
/// The first argument is the notification type, the next is the path for the
/// the notification, followed by the baseline diff, target diff, and workdir diff.
///
/// The callback must return a bool specifying whether the checkout should
/// continue.
pub type Notify<'a> = dyn FnMut(
        CheckoutNotificationType,
        Option<&Path>,
        Option<DiffFile<'_>>,
        Option<DiffFile<'_>>,
        Option<DiffFile<'_>>,
    ) -> bool
    + 'a;

impl<'cb> Default for RepoBuilder<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

/// Options that can be passed to `RepoBuilder::clone_local`.
#[derive(Clone, Copy)]
pub enum CloneLocal {
    /// Auto-detect (default)
    ///
    /// Here libgit2 will bypass the git-aware transport for local paths, but
    /// use a normal fetch for `file://` URLs.
    Auto = raw::GIT_CLONE_LOCAL_AUTO as isize,

    /// Bypass the git-aware transport even for `file://` URLs.
    Local = raw::GIT_CLONE_LOCAL as isize,

    /// Never bypass the git-aware transport
    None = raw::GIT_CLONE_NO_LOCAL as isize,

    /// Bypass the git-aware transport, but don't try to use hardlinks.
    NoLinks = raw::GIT_CLONE_LOCAL_NO_LINKS as isize,

    #[doc(hidden)]
    __Nonexhaustive = 0xff,
}

impl<'cb> RepoBuilder<'cb> {
    /// Creates a new repository builder with all of the default configuration.
    ///
    /// When ready, the `clone()` method can be used to clone a new repository
    /// using this configuration.
    pub fn new() -> RepoBuilder<'cb> {
        crate::init();
        RepoBuilder {
            bare: false,
            branch: None,
            local: true,
            clone_local: None,
            hardlinks: true,
            checkout: None,
            fetch_opts: None,
            remote_create: None,
        }
    }

    /// Indicate whether the repository will be cloned as a bare repository or
    /// not.
    pub fn bare(&mut self, bare: bool) -> &mut RepoBuilder<'cb> {
        self.bare = bare;
        self
    }

    /// Specify the name of the branch to check out after the clone.
    ///
    /// If not specified, the remote's default branch will be used.
    pub fn branch(&mut self, branch: &str) -> &mut RepoBuilder<'cb> {
        self.branch = Some(CString::new(branch).unwrap());
        self
    }

    /// Configures options for bypassing the git-aware transport on clone.
    ///
    /// Bypassing it means that instead of a fetch libgit2 will copy the object
    /// database directory instead of figuring out what it needs, which is
    /// faster. If possible, it will hardlink the files to save space.
    pub fn clone_local(&mut self, clone_local: CloneLocal) -> &mut RepoBuilder<'cb> {
        self.clone_local = Some(clone_local);
        self
    }

    /// Set the flag for bypassing the git aware transport mechanism for local
    /// paths.
    ///
    /// If `true`, the git-aware transport will be bypassed for local paths. If
    /// `false`, the git-aware transport will not be bypassed.
    #[deprecated(note = "use `clone_local` instead")]
    #[doc(hidden)]
    pub fn local(&mut self, local: bool) -> &mut RepoBuilder<'cb> {
        self.local = local;
        self
    }

    /// Set the flag for whether hardlinks are used when using a local git-aware
    /// transport mechanism.
    #[deprecated(note = "use `clone_local` instead")]
    #[doc(hidden)]
    pub fn hardlinks(&mut self, links: bool) -> &mut RepoBuilder<'cb> {
        self.hardlinks = links;
        self
    }

    /// Configure the checkout which will be performed by consuming a checkout
    /// builder.
    pub fn with_checkout(&mut self, checkout: CheckoutBuilder<'cb>) -> &mut RepoBuilder<'cb> {
        self.checkout = Some(checkout);
        self
    }

    /// Options which control the fetch, including callbacks.
    ///
    /// The callbacks are used for reporting fetch progress, and for acquiring
    /// credentials in the event they are needed.
    pub fn fetch_options(&mut self, fetch_opts: FetchOptions<'cb>) -> &mut RepoBuilder<'cb> {
        self.fetch_opts = Some(fetch_opts);
        self
    }

    /// Configures a callback used to create the git remote, prior to its being
    /// used to perform the clone operation.
    pub fn remote_create<F>(&mut self, f: F) -> &mut RepoBuilder<'cb>
    where
        F: for<'a> FnMut(&'a Repository, &str, &str) -> Result<Remote<'a>, Error> + 'cb,
    {
        self.remote_create = Some(Box::new(f));
        self
    }

    /// Clone a remote repository.
    ///
    /// This will use the options configured so far to clone the specified URL
    /// into the specified local path.
    pub fn clone(&mut self, url: &str, into: &Path) -> Result<Repository, Error> {
        let mut opts: raw::git_clone_options = unsafe { mem::zeroed() };
        unsafe {
            try_call!(raw::git_clone_init_options(
                &mut opts,
                raw::GIT_CLONE_OPTIONS_VERSION
            ));
        }
        opts.bare = self.bare as c_int;
        opts.checkout_branch = self
            .branch
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        if let Some(ref local) = self.clone_local {
            opts.local = *local as raw::git_clone_local_t;
        } else {
            opts.local = match (self.local, self.hardlinks) {
                (true, false) => raw::GIT_CLONE_LOCAL_NO_LINKS,
                (false, _) => raw::GIT_CLONE_NO_LOCAL,
                (true, _) => raw::GIT_CLONE_LOCAL_AUTO,
            };
        }

        if let Some(ref mut cbs) = self.fetch_opts {
            opts.fetch_opts = cbs.raw();
        }

        if let Some(ref mut c) = self.checkout {
            unsafe {
                c.configure(&mut opts.checkout_opts);
            }
        }

        if let Some(ref mut callback) = self.remote_create {
            opts.remote_cb = Some(remote_create_cb);
            opts.remote_cb_payload = callback as *mut _ as *mut _;
        }

        let url = CString::new(url)?;
        // Normal file path OK (does not need Windows conversion).
        let into = into.into_c_string()?;
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_clone(&mut raw, url, into, &opts));
            Ok(Binding::from_raw(raw))
        }
    }
}

extern "C" fn remote_create_cb(
    out: *mut *mut raw::git_remote,
    repo: *mut raw::git_repository,
    name: *const c_char,
    url: *const c_char,
    payload: *mut c_void,
) -> c_int {
    unsafe {
        let repo = Repository::from_raw(repo);
        let code = panic::wrap(|| {
            let name = CStr::from_ptr(name).to_str().unwrap();
            let url = CStr::from_ptr(url).to_str().unwrap();
            let f = payload as *mut Box<RemoteCreate<'_>>;
            match (*f)(&repo, name, url) {
                Ok(remote) => {
                    *out = crate::remote::remote_into_raw(remote);
                    0
                }
                Err(e) => e.raw_code(),
            }
        });
        mem::forget(repo);
        code.unwrap_or(-1)
    }
}

impl<'cb> Default for CheckoutBuilder<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'cb> CheckoutBuilder<'cb> {
    /// Creates a new builder for checkouts with all of its default
    /// configuration.
    pub fn new() -> CheckoutBuilder<'cb> {
        crate::init();
        CheckoutBuilder {
            disable_filters: false,
            dir_perm: None,
            file_perm: None,
            path_ptrs: Vec::new(),
            paths: Vec::new(),
            target_dir: None,
            ancestor_label: None,
            our_label: None,
            their_label: None,
            checkout_opts: raw::GIT_CHECKOUT_SAFE as u32,
            progress: None,
            notify: None,
            notify_flags: CheckoutNotificationType::empty(),
        }
    }

    /// Indicate that this checkout should perform a dry run by checking for
    /// conflicts but not make any actual changes.
    pub fn dry_run(&mut self) -> &mut CheckoutBuilder<'cb> {
        self.checkout_opts &= !((1 << 4) - 1);
        self.checkout_opts |= raw::GIT_CHECKOUT_NONE as u32;
        self
    }

    /// Take any action necessary to get the working directory to match the
    /// target including potentially discarding modified files.
    pub fn force(&mut self) -> &mut CheckoutBuilder<'cb> {
        self.checkout_opts &= !((1 << 4) - 1);
        self.checkout_opts |= raw::GIT_CHECKOUT_FORCE as u32;
        self
    }

    /// Indicate that the checkout should be performed safely, allowing new
    /// files to be created but not overwriting existing files or changes.
    ///
    /// This is the default.
    pub fn safe(&mut self) -> &mut CheckoutBuilder<'cb> {
        self.checkout_opts &= !((1 << 4) - 1);
        self.checkout_opts |= raw::GIT_CHECKOUT_SAFE as u32;
        self
    }

    fn flag(&mut self, bit: raw::git_checkout_strategy_t, on: bool) -> &mut CheckoutBuilder<'cb> {
        if on {
            self.checkout_opts |= bit as u32;
        } else {
            self.checkout_opts &= !(bit as u32);
        }
        self
    }

    /// In safe mode, create files that don't exist.
    ///
    /// Defaults to false.
    pub fn recreate_missing(&mut self, allow: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_RECREATE_MISSING, allow)
    }

    /// In safe mode, apply safe file updates even when there are conflicts
    /// instead of canceling the checkout.
    ///
    /// Defaults to false.
    pub fn allow_conflicts(&mut self, allow: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_ALLOW_CONFLICTS, allow)
    }

    /// Remove untracked files from the working dir.
    ///
    /// Defaults to false.
    pub fn remove_untracked(&mut self, remove: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_REMOVE_UNTRACKED, remove)
    }

    /// Remove ignored files from the working dir.
    ///
    /// Defaults to false.
    pub fn remove_ignored(&mut self, remove: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_REMOVE_IGNORED, remove)
    }

    /// Only update the contents of files that already exist.
    ///
    /// If set, files will not be created or deleted.
    ///
    /// Defaults to false.
    pub fn update_only(&mut self, update: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_UPDATE_ONLY, update)
    }

    /// Prevents checkout from writing the updated files' information to the
    /// index.
    ///
    /// Defaults to true.
    pub fn update_index(&mut self, update: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_DONT_UPDATE_INDEX, !update)
    }

    /// Indicate whether the index and git attributes should be refreshed from
    /// disk before any operations.
    ///
    /// Defaults to true,
    pub fn refresh(&mut self, refresh: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_NO_REFRESH, !refresh)
    }

    /// Skip files with unmerged index entries.
    ///
    /// Defaults to false.
    pub fn skip_unmerged(&mut self, skip: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_SKIP_UNMERGED, skip)
    }

    /// Indicate whether the checkout should proceed on conflicts by using the
    /// stage 2 version of the file ("ours").
    ///
    /// Defaults to false.
    pub fn use_ours(&mut self, ours: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_USE_OURS, ours)
    }

    /// Indicate whether the checkout should proceed on conflicts by using the
    /// stage 3 version of the file ("theirs").
    ///
    /// Defaults to false.
    pub fn use_theirs(&mut self, theirs: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_USE_THEIRS, theirs)
    }

    /// Indicate whether ignored files should be overwritten during the checkout.
    ///
    /// Defaults to true.
    pub fn overwrite_ignored(&mut self, overwrite: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_DONT_OVERWRITE_IGNORED, !overwrite)
    }

    /// Indicate whether a normal merge file should be written for conflicts.
    ///
    /// Defaults to false.
    pub fn conflict_style_merge(&mut self, on: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_CONFLICT_STYLE_MERGE, on)
    }

    /// Specify for which notification types to invoke the notification
    /// callback.
    ///
    /// Defaults to none.
    pub fn notify_on(
        &mut self,
        notification_types: CheckoutNotificationType,
    ) -> &mut CheckoutBuilder<'cb> {
        self.notify_flags = notification_types;
        self
    }

    /// Indicates whether to include common ancestor data in diff3 format files
    /// for conflicts.
    ///
    /// Defaults to false.
    pub fn conflict_style_diff3(&mut self, on: bool) -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_CONFLICT_STYLE_DIFF3, on)
    }

    /// Indicate whether to apply filters like CRLF conversion.
    pub fn disable_filters(&mut self, disable: bool) -> &mut CheckoutBuilder<'cb> {
        self.disable_filters = disable;
        self
    }

    /// Set the mode with which new directories are created.
    ///
    /// Default is 0755
    pub fn dir_perm(&mut self, perm: i32) -> &mut CheckoutBuilder<'cb> {
        self.dir_perm = Some(perm);
        self
    }

    /// Set the mode with which new files are created.
    ///
    /// The default is 0644 or 0755 as dictated by the blob.
    pub fn file_perm(&mut self, perm: i32) -> &mut CheckoutBuilder<'cb> {
        self.file_perm = Some(perm);
        self
    }

    /// Add a path to be checked out.
    ///
    /// If no paths are specified, then all files are checked out. Otherwise
    /// only these specified paths are checked out.
    pub fn path<T: IntoCString>(&mut self, path: T) -> &mut CheckoutBuilder<'cb> {
        let path = util::cstring_to_repo_path(path).unwrap();
        self.path_ptrs.push(path.as_ptr());
        self.paths.push(path);
        self
    }

    /// Set the directory to check out to
    pub fn target_dir(&mut self, dst: &Path) -> &mut CheckoutBuilder<'cb> {
        // Normal file path OK (does not need Windows conversion).
        self.target_dir = Some(dst.into_c_string().unwrap());
        self
    }

    /// The name of the common ancestor side of conflicts
    pub fn ancestor_label(&mut self, label: &str) -> &mut CheckoutBuilder<'cb> {
        self.ancestor_label = Some(CString::new(label).unwrap());
        self
    }

    /// The name of the common our side of conflicts
    pub fn our_label(&mut self, label: &str) -> &mut CheckoutBuilder<'cb> {
        self.our_label = Some(CString::new(label).unwrap());
        self
    }

    /// The name of the common their side of conflicts
    pub fn their_label(&mut self, label: &str) -> &mut CheckoutBuilder<'cb> {
        self.their_label = Some(CString::new(label).unwrap());
        self
    }

    /// Set a callback to receive notifications of checkout progress.
    pub fn progress<F>(&mut self, cb: F) -> &mut CheckoutBuilder<'cb>
    where
        F: FnMut(Option<&Path>, usize, usize) + 'cb,
    {
        self.progress = Some(Box::new(cb) as Box<Progress<'cb>>);
        self
    }

    /// Set a callback to receive checkout notifications.
    ///
    /// Callbacks are invoked prior to modifying any files on disk.
    /// Returning `false` from the callback will cancel the checkout.
    pub fn notify<F>(&mut self, cb: F) -> &mut CheckoutBuilder<'cb>
    where
        F: FnMut(
                CheckoutNotificationType,
                Option<&Path>,
                Option<DiffFile<'_>>,
                Option<DiffFile<'_>>,
                Option<DiffFile<'_>>,
            ) -> bool
            + 'cb,
    {
        self.notify = Some(Box::new(cb) as Box<Notify<'cb>>);
        self
    }

    /// Configure a raw checkout options based on this configuration.
    ///
    /// This method is unsafe as there is no guarantee that this structure will
    /// outlive the provided checkout options.
    pub unsafe fn configure(&mut self, opts: &mut raw::git_checkout_options) {
        opts.version = raw::GIT_CHECKOUT_OPTIONS_VERSION;
        opts.disable_filters = self.disable_filters as c_int;
        opts.dir_mode = self.dir_perm.unwrap_or(0) as c_uint;
        opts.file_mode = self.file_perm.unwrap_or(0) as c_uint;

        if !self.path_ptrs.is_empty() {
            opts.paths.strings = self.path_ptrs.as_ptr() as *mut _;
            opts.paths.count = self.path_ptrs.len() as size_t;
        }

        if let Some(ref c) = self.target_dir {
            opts.target_directory = c.as_ptr();
        }
        if let Some(ref c) = self.ancestor_label {
            opts.ancestor_label = c.as_ptr();
        }
        if let Some(ref c) = self.our_label {
            opts.our_label = c.as_ptr();
        }
        if let Some(ref c) = self.their_label {
            opts.their_label = c.as_ptr();
        }
        if self.progress.is_some() {
            opts.progress_cb = Some(progress_cb);
            opts.progress_payload = self as *mut _ as *mut _;
        }
        if self.notify.is_some() {
            opts.notify_cb = Some(notify_cb);
            opts.notify_payload = self as *mut _ as *mut _;
            opts.notify_flags = self.notify_flags.bits() as c_uint;
        }
        opts.checkout_strategy = self.checkout_opts as c_uint;
    }
}

extern "C" fn progress_cb(
    path: *const c_char,
    completed: size_t,
    total: size_t,
    data: *mut c_void,
) {
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut CheckoutBuilder<'_>);
        let callback = match payload.progress {
            Some(ref mut c) => c,
            None => return,
        };
        let path = if path.is_null() {
            None
        } else {
            Some(util::bytes2path(CStr::from_ptr(path).to_bytes()))
        };
        callback(path, completed as usize, total as usize)
    });
}

extern "C" fn notify_cb(
    why: raw::git_checkout_notify_t,
    path: *const c_char,
    baseline: *const raw::git_diff_file,
    target: *const raw::git_diff_file,
    workdir: *const raw::git_diff_file,
    data: *mut c_void,
) -> c_int {
    // pack callback etc
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut CheckoutBuilder<'_>);
        let callback = match payload.notify {
            Some(ref mut c) => c,
            None => return 0,
        };
        let path = if path.is_null() {
            None
        } else {
            Some(util::bytes2path(CStr::from_ptr(path).to_bytes()))
        };

        let baseline = if baseline.is_null() {
            None
        } else {
            Some(DiffFile::from_raw(baseline))
        };

        let target = if target.is_null() {
            None
        } else {
            Some(DiffFile::from_raw(target))
        };

        let workdir = if workdir.is_null() {
            None
        } else {
            Some(DiffFile::from_raw(workdir))
        };

        let why = CheckoutNotificationType::from_bits_truncate(why as u32);
        let keep_going = callback(why, path, baseline, target, workdir);
        if keep_going {
            0
        } else {
            1
        }
    })
    .unwrap_or(2)
}

unsafe impl Send for TreeUpdateBuilder {}

impl Default for TreeUpdateBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TreeUpdateBuilder {
    /// Create a new empty series of updates.
    pub fn new() -> Self {
        Self {
            updates: Vec::new(),
            paths: Vec::new(),
        }
    }

    /// Add an update removing the specified `path` from a tree.
    pub fn remove<T: IntoCString>(&mut self, path: T) -> &mut Self {
        let path = util::cstring_to_repo_path(path).unwrap();
        let path_ptr = path.as_ptr();
        self.paths.push(path);
        self.updates.push(raw::git_tree_update {
            action: raw::GIT_TREE_UPDATE_REMOVE,
            id: raw::git_oid {
                id: [0; raw::GIT_OID_RAWSZ],
            },
            filemode: raw::GIT_FILEMODE_UNREADABLE,
            path: path_ptr,
        });
        self
    }

    /// Add an update setting the specified `path` to a specific Oid, whether it currently exists
    /// or not.
    ///
    /// Note that libgit2 does not support an upsert of a previously removed path, or an upsert
    /// that changes the type of an object (such as from tree to blob or vice versa).
    pub fn upsert<T: IntoCString>(&mut self, path: T, id: Oid, filemode: FileMode) -> &mut Self {
        let path = util::cstring_to_repo_path(path).unwrap();
        let path_ptr = path.as_ptr();
        self.paths.push(path);
        self.updates.push(raw::git_tree_update {
            action: raw::GIT_TREE_UPDATE_UPSERT,
            id: unsafe { *id.raw() },
            filemode: u32::from(filemode) as raw::git_filemode_t,
            path: path_ptr,
        });
        self
    }

    /// Create a new tree from the specified baseline and this series of updates.
    ///
    /// The baseline tree must exist in the specified repository.
    pub fn create_updated(&mut self, repo: &Repository, baseline: &Tree<'_>) -> Result<Oid, Error> {
        let mut ret = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_tree_create_updated(
                &mut ret,
                repo.raw(),
                baseline.raw(),
                self.updates.len(),
                self.updates.as_ptr()
            ));
            Ok(Binding::from_raw(&ret as *const _))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CheckoutBuilder, RepoBuilder, TreeUpdateBuilder};
    use crate::{CheckoutNotificationType, FileMode, Repository};
    use std::fs;
    use std::path::Path;
    use tempfile::TempDir;

    #[test]
    fn smoke() {
        let r = RepoBuilder::new().clone("/path/to/nowhere", Path::new("foo"));
        assert!(r.is_err());
    }

    #[test]
    fn smoke2() {
        let td = TempDir::new().unwrap();
        Repository::init_bare(&td.path().join("bare")).unwrap();
        let url = if cfg!(unix) {
            format!("file://{}/bare", td.path().display())
        } else {
            format!(
                "file:///{}/bare",
                td.path().display().to_string().replace("\\", "/")
            )
        };

        let dst = td.path().join("foo");
        RepoBuilder::new().clone(&url, &dst).unwrap();
        fs::remove_dir_all(&dst).unwrap();
        assert!(RepoBuilder::new().branch("foo").clone(&url, &dst).is_err());
    }

    #[test]
    fn smoke_tree_create_updated() {
        let (_tempdir, repo) = crate::test::repo_init();
        let (_, tree_id) = crate::test::commit(&repo);
        let tree = t!(repo.find_tree(tree_id));
        assert!(tree.get_name("bar").is_none());
        let foo_id = tree.get_name("foo").unwrap().id();
        let tree2_id = t!(TreeUpdateBuilder::new()
            .remove("foo")
            .upsert("bar/baz", foo_id, FileMode::Blob)
            .create_updated(&repo, &tree));
        let tree2 = t!(repo.find_tree(tree2_id));
        assert!(tree2.get_name("foo").is_none());
        let baz_id = tree2.get_path(Path::new("bar/baz")).unwrap().id();
        assert_eq!(foo_id, baz_id);
    }

    /// Issue regression test #365
    #[test]
    fn notify_callback() {
        let td = TempDir::new().unwrap();
        let cd = TempDir::new().unwrap();

        {
            let mut opts = crate::RepositoryInitOptions::new();
            opts.initial_head("main");
            let repo = Repository::init_opts(&td.path(), &opts).unwrap();

            let mut config = repo.config().unwrap();
            config.set_str("user.name", "name").unwrap();
            config.set_str("user.email", "email").unwrap();

            let mut index = repo.index().unwrap();
            let p = Path::new(td.path()).join("file");
            println!("using path {:?}", p);
            fs::File::create(&p).unwrap();
            index.add_path(&Path::new("file")).unwrap();
            let id = index.write_tree().unwrap();

            let tree = repo.find_tree(id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }

        let repo = Repository::open_bare(&td.path().join(".git")).unwrap();
        let tree = repo
            .revparse_single(&"main")
            .unwrap()
            .peel_to_tree()
            .unwrap();
        let mut index = repo.index().unwrap();
        index.read_tree(&tree).unwrap();

        let mut checkout_opts = CheckoutBuilder::new();
        checkout_opts.target_dir(&cd.path());
        checkout_opts.notify_on(CheckoutNotificationType::all());
        checkout_opts.notify(|_notif, _path, baseline, target, workdir| {
            assert!(baseline.is_none());
            assert_eq!(target.unwrap().path(), Some(Path::new("file")));
            assert!(workdir.is_none());
            true
        });
        repo.checkout_index(Some(&mut index), Some(&mut checkout_opts))
            .unwrap();
    }
}
