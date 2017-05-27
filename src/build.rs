//! Builder-pattern objects for configuration various git operations.

use std::ffi::{CStr, CString};
use std::mem;
use std::path::Path;
use std::ptr;
use libc::{c_char, size_t, c_void, c_uint, c_int};

use {raw, panic, Error, Repository, FetchOptions, IntoCString};
use {CheckoutNotificationType, DiffFile};
use util::{self, Binding};

/// A builder struct which is used to build configuration for cloning a new git
/// repository.
pub struct RepoBuilder<'cb> {
    bare: bool,
    branch: Option<CString>,
    local: bool,
    hardlinks: bool,
    checkout: Option<CheckoutBuilder<'cb>>,
    fetch_opts: Option<FetchOptions<'cb>>,
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
/// The first argument is the path for the notification, the next is the numver
/// of completed steps so far, and the final is the total number of steps.
pub type Progress<'a> = FnMut(Option<&Path>, usize, usize) + 'a;

/// Checkout notifications callback.
///
/// The first argument is the notification type, the next is the path for the
/// the notification, followed by the baseline diff, target diff, and workdir diff.
///
/// The callback must return a bool specifying whether the checkout should
/// continue.
pub type Notify<'a> = FnMut(CheckoutNotificationType, Option<&Path>, DiffFile,
                            DiffFile, DiffFile) -> bool + 'a;


impl<'cb> Default for RepoBuilder<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'cb> RepoBuilder<'cb> {
    /// Creates a new repository builder with all of the default configuration.
    ///
    /// When ready, the `clone()` method can be used to clone a new repository
    /// using this configuration.
    pub fn new() -> RepoBuilder<'cb> {
        ::init();
        RepoBuilder {
            bare: false,
            branch: None,
            local: true,
            hardlinks: true,
            checkout: None,
            fetch_opts: None,
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

    /// Set the flag for bypassing the git aware transport mechanism for local
    /// paths.
    ///
    /// If `true`, the git-aware transport will be bypassed for local paths. If
    /// `false`, the git-aware transport will not be bypassed.
    pub fn local(&mut self, local: bool) -> &mut RepoBuilder<'cb> {
        self.local = local;
        self
    }

    /// Set the flag for whether hardlinks are used when using a local git-aware
    /// transport mechanism.
    pub fn hardlinks(&mut self, links: bool) -> &mut RepoBuilder<'cb> {
        self.hardlinks = links;
        self
    }

    /// Configure the checkout which will be performed by consuming a checkout
    /// builder.
    pub fn with_checkout(&mut self, checkout: CheckoutBuilder<'cb>)
                         -> &mut RepoBuilder<'cb> {
        self.checkout = Some(checkout);
        self
    }

    /// Options which control the fetch, including callbacks.
    ///
    /// The callbacks are used for reporting fetch progress, and for acquiring
    /// credentials in the event they are needed.
    pub fn fetch_options(&mut self, fetch_opts: FetchOptions<'cb>)
                            -> &mut RepoBuilder<'cb> {
        self.fetch_opts = Some(fetch_opts);
        self
    }

    /// Clone a remote repository.
    ///
    /// This will use the options configured so far to clone the specified url
    /// into the specified local path.
    pub fn clone(&mut self, url: &str, into: &Path) -> Result<Repository, Error> {
        let mut opts: raw::git_clone_options = unsafe { mem::zeroed() };
        unsafe {
            try_call!(raw::git_clone_init_options(&mut opts,
                                                  raw::GIT_CLONE_OPTIONS_VERSION));
        }
        opts.bare = self.bare as c_int;
        opts.checkout_branch = self.branch.as_ref().map(|s| {
            s.as_ptr()
        }).unwrap_or(ptr::null());

        opts.local = match (self.local, self.hardlinks) {
            (true, false) => raw::GIT_CLONE_LOCAL_NO_LINKS,
            (false, _) => raw::GIT_CLONE_NO_LOCAL,
            (true, _) => raw::GIT_CLONE_LOCAL_AUTO,
        };
        opts.checkout_opts.checkout_strategy =
            raw::GIT_CHECKOUT_SAFE as c_uint;

        match self.fetch_opts {
            Some(ref mut cbs) => {
                opts.fetch_opts = cbs.raw();
            },
            None => {}
        }

        match self.checkout {
            Some(ref mut c) => unsafe { c.configure(&mut opts.checkout_opts) },
            None => {}
        }

        let url = try!(CString::new(url));
        let into = try!(into.into_c_string());
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_clone(&mut raw, url, into, &opts));
            Ok(Binding::from_raw(raw))
        }
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
        ::init();
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
    /// files to be created but not overwriting extisting files or changes.
    ///
    /// This is the default.
    pub fn safe(&mut self) -> &mut CheckoutBuilder<'cb> {
        self.checkout_opts &= !((1 << 4) - 1);
        self.checkout_opts |= raw::GIT_CHECKOUT_SAFE as u32;
        self
    }

    fn flag(&mut self, bit: raw::git_checkout_strategy_t,
            on: bool) -> &mut CheckoutBuilder<'cb> {
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
    pub fn remove_untracked(&mut self, remove: bool)
                            -> &mut CheckoutBuilder<'cb> {
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
    pub fn overwrite_ignored(&mut self, overwrite: bool)
                             -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_DONT_OVERWRITE_IGNORED, !overwrite)
    }

    /// Indicate whether a normal merge file should be written for conflicts.
    ///
    /// Defaults to false.
    pub fn conflict_style_merge(&mut self, on: bool)
                                -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_CONFLICT_STYLE_MERGE, on)
    }

    /// Specify for which notification types to invoke the notification
    /// callback.
    ///
    /// Defaults to none.
    pub fn notify_on(&mut self, notification_types: CheckoutNotificationType)
                     -> &mut CheckoutBuilder<'cb> {
        self.notify_flags = notification_types;
        self
    }

    /// Indicates whether to include common ancestor data in diff3 format files
    /// for conflicts.
    ///
    /// Defaults to false.
    pub fn conflict_style_diff3(&mut self, on: bool)
                                -> &mut CheckoutBuilder<'cb> {
        self.flag(raw::GIT_CHECKOUT_CONFLICT_STYLE_DIFF3, on)
    }

    /// Indicate whether to apply filters like CRLF conversion.
    pub fn disable_filters(&mut self, disable: bool)
                           -> &mut CheckoutBuilder<'cb> {
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
    pub fn path<T: IntoCString>(&mut self, path: T)
                                   -> &mut CheckoutBuilder<'cb> {
        let path = path.into_c_string().unwrap();
        self.path_ptrs.push(path.as_ptr());
        self.paths.push(path);
        self
    }

    /// Set the directory to check out to
    pub fn target_dir(&mut self, dst: &Path) -> &mut CheckoutBuilder<'cb> {
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
                       where F: FnMut(Option<&Path>, usize, usize) + 'cb {
        self.progress = Some(Box::new(cb) as Box<Progress<'cb>>);
        self
    }

    /// Set a callback to receive checkout notifications.
    ///
    /// Callbacks are invoked prior to modifying any files on disk.
    /// Returning `false` from the callback will cancel the checkout.
    pub fn notify<F>(&mut self, cb: F) -> &mut CheckoutBuilder<'cb>
        where F: FnMut(CheckoutNotificationType, Option<&Path>, DiffFile,
                       DiffFile, DiffFile) -> bool + 'cb
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

        if self.path_ptrs.len() > 0 {
            opts.paths.strings = self.path_ptrs.as_ptr() as *mut _;
            opts.paths.count = self.path_ptrs.len() as size_t;
        }

        match self.target_dir {
            Some(ref c) => opts.target_directory = c.as_ptr(),
            None => {}
        }
        match self.ancestor_label {
            Some(ref c) => opts.ancestor_label = c.as_ptr(),
            None => {}
        }
        match self.our_label {
            Some(ref c) => opts.our_label = c.as_ptr(),
            None => {}
        }
        match self.their_label {
            Some(ref c) => opts.their_label = c.as_ptr(),
            None => {}
        }
        if self.progress.is_some() {
            let f: raw::git_checkout_progress_cb = progress_cb;
            opts.progress_cb = Some(f);
            opts.progress_payload = self as *mut _ as *mut _;
        }
        if self.notify.is_some() {
            let f: raw::git_checkout_notify_cb = notify_cb;
            opts.notify_cb = Some(f);
            opts.notify_payload = self as *mut _ as *mut _;
            opts.notify_flags = self.notify_flags.bits() as c_uint;
        }
        opts.checkout_strategy = self.checkout_opts as c_uint;
    }
}

extern fn progress_cb(path: *const c_char,
                      completed: size_t,
                      total: size_t,
                      data: *mut c_void) {
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut CheckoutBuilder);
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

extern fn notify_cb(why: raw::git_checkout_notify_t,
                    path: *const c_char,
                    baseline: *const raw::git_diff_file,
                    target: *const raw::git_diff_file,
                    workdir: *const raw::git_diff_file,
                    data: *mut c_void) -> c_int {
    // pack callback etc
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut CheckoutBuilder);
        let callback = match payload.notify {
            Some(ref mut c) => c,
            None => return 0,
        };
        let path = if path.is_null() {
            None
        } else {
            Some(util::bytes2path(CStr::from_ptr(path).to_bytes()))
        };

        let why = CheckoutNotificationType::from_bits_truncate(why as u32);
        let keep_going = callback(why,
                                  path,
                                  DiffFile::from_raw(baseline),
                                  DiffFile::from_raw(target),
                                  DiffFile::from_raw(workdir));
        if keep_going {0} else {1}
    }).unwrap_or(2)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;
    use tempdir::TempDir;
    use super::RepoBuilder;
    use Repository;

    #[test]
    fn smoke() {
        let r = RepoBuilder::new().clone("/path/to/nowhere", Path::new("foo"));
        assert!(r.is_err());
    }

    #[test]
    fn smoke2() {
        let td = TempDir::new("test").unwrap();
        Repository::init_bare(&td.path().join("bare")).unwrap();
        let url = if cfg!(unix) {
            format!("file://{}/bare", td.path().display())
        } else {
            format!("file:///{}/bare", td.path().display().to_string()
                                         .replace("\\", "/"))
        };

        let dst = td.path().join("foo");
        RepoBuilder::new().clone(&url, &dst).unwrap();
        fs::remove_dir_all(&dst).unwrap();
        RepoBuilder::new().local(false).clone(&url, &dst).unwrap();
        fs::remove_dir_all(&dst).unwrap();
        RepoBuilder::new().local(false).hardlinks(false).bare(true)
                          .clone(&url, &dst).unwrap();
        fs::remove_dir_all(&dst).unwrap();
        assert!(RepoBuilder::new().branch("foo")
                                  .clone(&url, &dst).is_err());
    }

}
