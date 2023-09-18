use crate::build::CheckoutBuilder;
use crate::util::{self, Binding};
use crate::{panic, raw, IntoCString, Oid, Signature, StashApplyProgress, StashFlags};
use libc::{c_char, c_int, c_void, size_t};
use std::ffi::{c_uint, CStr, CString};
use std::mem;

/// Stash application options structure
pub struct StashSaveOptions<'a> {
    message: Option<CString>,
    flags: Option<StashFlags>,
    stasher: Signature<'a>,
    pathspec: Vec<CString>,
    pathspec_ptrs: Vec<*const c_char>,
    raw_opts: raw::git_stash_save_options,
}

impl<'a> StashSaveOptions<'a> {
    /// Creates a default
    pub fn new(stasher: Signature<'a>) -> Self {
        let mut opts = Self {
            message: None,
            flags: None,
            stasher,
            pathspec: Vec::new(),
            pathspec_ptrs: Vec::new(),
            raw_opts: unsafe { mem::zeroed() },
        };
        assert_eq!(
            unsafe {
                raw::git_stash_save_options_init(
                    &mut opts.raw_opts,
                    raw::GIT_STASH_SAVE_OPTIONS_VERSION,
                )
            },
            0
        );
        opts
    }

    /// Customize optional `flags` field
    pub fn flags(&mut self, flags: Option<StashFlags>) -> &mut Self {
        self.flags = flags;
        self
    }

    /// Add to the array of paths patterns to build the stash.
    pub fn pathspec<T: IntoCString>(&mut self, pathspec: T) -> &mut Self {
        let s = util::cstring_to_repo_path(pathspec).unwrap();
        self.pathspec_ptrs.push(s.as_ptr());
        self.pathspec.push(s);
        self
    }

    /// Acquire a pointer to the underlying raw options.
    ///
    /// This function is unsafe as the pointer is only valid so long as this
    /// structure is not moved, modified, or used elsewhere.
    pub unsafe fn raw(&mut self) -> *const raw::git_stash_save_options {
        self.raw_opts.flags = self.flags.unwrap_or_else(StashFlags::empty).bits() as c_uint;
        self.raw_opts.message = crate::call::convert(&self.message);
        self.raw_opts.paths.count = self.pathspec_ptrs.len() as size_t;
        self.raw_opts.paths.strings = self.pathspec_ptrs.as_ptr() as *mut _;
        self.raw_opts.stasher = self.stasher.raw();

        &self.raw_opts as *const _
    }
}

/// Stash application progress notification function.
///
/// Return `true` to continue processing, or `false` to
/// abort the stash application.
// FIXME: This probably should have been pub(crate) since it is not used anywhere.
pub type StashApplyProgressCb<'a> = dyn FnMut(StashApplyProgress) -> bool + 'a;

/// This is a callback function you can provide to iterate over all the
/// stashed states that will be invoked per entry.
// FIXME: This probably should have been pub(crate) since it is not used anywhere.
pub type StashCb<'a> = dyn FnMut(usize, &str, &Oid) -> bool + 'a;

/// Stash application options structure
pub struct StashApplyOptions<'cb> {
    progress: Option<Box<StashApplyProgressCb<'cb>>>,
    checkout_options: Option<CheckoutBuilder<'cb>>,
    raw_opts: raw::git_stash_apply_options,
}

impl<'cb> Default for StashApplyOptions<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'cb> StashApplyOptions<'cb> {
    /// Creates a default set of merge options.
    pub fn new() -> StashApplyOptions<'cb> {
        let mut opts = StashApplyOptions {
            progress: None,
            checkout_options: None,
            raw_opts: unsafe { mem::zeroed() },
        };
        assert_eq!(
            unsafe { raw::git_stash_apply_init_options(&mut opts.raw_opts, 1) },
            0
        );
        opts
    }

    /// Set stash application flag to GIT_STASH_APPLY_REINSTATE_INDEX
    pub fn reinstantiate_index(&mut self) -> &mut StashApplyOptions<'cb> {
        self.raw_opts.flags = raw::GIT_STASH_APPLY_REINSTATE_INDEX as u32;
        self
    }

    /// Options to use when writing files to the working directory
    pub fn checkout_options(&mut self, opts: CheckoutBuilder<'cb>) -> &mut StashApplyOptions<'cb> {
        self.checkout_options = Some(opts);
        self
    }

    /// Optional callback to notify the consumer of application progress.
    ///
    /// Return `true` to continue processing, or `false` to
    /// abort the stash application.
    pub fn progress_cb<C>(&mut self, callback: C) -> &mut StashApplyOptions<'cb>
    where
        C: FnMut(StashApplyProgress) -> bool + 'cb,
    {
        self.progress = Some(Box::new(callback) as Box<StashApplyProgressCb<'cb>>);
        self.raw_opts.progress_cb = Some(stash_apply_progress_cb);
        self.raw_opts.progress_payload = self as *mut _ as *mut _;
        self
    }

    /// Pointer to a raw git_stash_apply_options
    pub fn raw(&mut self) -> &raw::git_stash_apply_options {
        unsafe {
            if let Some(opts) = self.checkout_options.as_mut() {
                opts.configure(&mut self.raw_opts.checkout_options);
            }
        }
        &self.raw_opts
    }
}

pub(crate) struct StashCbData<'a> {
    pub callback: &'a mut StashCb<'a>,
}

pub(crate) extern "C" fn stash_cb(
    index: size_t,
    message: *const c_char,
    stash_id: *const raw::git_oid,
    payload: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let data = &mut *(payload as *mut StashCbData<'_>);
        let res = {
            let callback = &mut data.callback;
            callback(
                index,
                CStr::from_ptr(message).to_str().unwrap(),
                &Binding::from_raw(stash_id),
            )
        };

        if res {
            0
        } else {
            1
        }
    })
    .unwrap_or(1)
}

fn convert_progress(progress: raw::git_stash_apply_progress_t) -> StashApplyProgress {
    match progress {
        raw::GIT_STASH_APPLY_PROGRESS_NONE => StashApplyProgress::None,
        raw::GIT_STASH_APPLY_PROGRESS_LOADING_STASH => StashApplyProgress::LoadingStash,
        raw::GIT_STASH_APPLY_PROGRESS_ANALYZE_INDEX => StashApplyProgress::AnalyzeIndex,
        raw::GIT_STASH_APPLY_PROGRESS_ANALYZE_MODIFIED => StashApplyProgress::AnalyzeModified,
        raw::GIT_STASH_APPLY_PROGRESS_ANALYZE_UNTRACKED => StashApplyProgress::AnalyzeUntracked,
        raw::GIT_STASH_APPLY_PROGRESS_CHECKOUT_UNTRACKED => StashApplyProgress::CheckoutUntracked,
        raw::GIT_STASH_APPLY_PROGRESS_CHECKOUT_MODIFIED => StashApplyProgress::CheckoutModified,
        raw::GIT_STASH_APPLY_PROGRESS_DONE => StashApplyProgress::Done,

        _ => StashApplyProgress::None,
    }
}

extern "C" fn stash_apply_progress_cb(
    progress: raw::git_stash_apply_progress_t,
    payload: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let options = &mut *(payload as *mut StashApplyOptions<'_>);
        let res = {
            let callback = options.progress.as_mut().unwrap();
            callback(convert_progress(progress))
        };

        if res {
            0
        } else {
            -1
        }
    })
    .unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use crate::stash::{StashApplyOptions, StashSaveOptions};
    use crate::test::repo_init;
    use crate::{IndexAddOption, Repository, StashFlags, Status};
    use std::fs;
    use std::path::{Path, PathBuf};

    fn make_stash<C>(next: C)
    where
        C: FnOnce(&mut Repository),
    {
        let (_td, mut repo) = repo_init();
        let signature = repo.signature().unwrap();

        let p = Path::new(repo.workdir().unwrap()).join("file_b.txt");
        println!("using path {:?}", p);

        fs::write(&p, "data".as_bytes()).unwrap();

        let rel_p = Path::new("file_b.txt");
        assert!(repo.status_file(&rel_p).unwrap() == Status::WT_NEW);

        repo.stash_save(&signature, "msg1", Some(StashFlags::INCLUDE_UNTRACKED))
            .unwrap();

        assert!(repo.status_file(&rel_p).is_err());

        let mut count = 0;
        repo.stash_foreach(|index, name, _oid| {
            count += 1;
            assert!(index == 0);
            assert!(name == "On main: msg1");
            true
        })
        .unwrap();

        assert!(count == 1);
        next(&mut repo);
    }

    fn count_stash(repo: &mut Repository) -> usize {
        let mut count = 0;
        repo.stash_foreach(|_, _, _| {
            count += 1;
            true
        })
        .unwrap();
        count
    }

    #[test]
    fn smoke_stash_save_drop() {
        make_stash(|repo| {
            repo.stash_drop(0).unwrap();
            assert!(count_stash(repo) == 0)
        })
    }

    #[test]
    fn smoke_stash_save_pop() {
        make_stash(|repo| {
            repo.stash_pop(0, None).unwrap();
            assert!(count_stash(repo) == 0)
        })
    }

    #[test]
    fn smoke_stash_save_apply() {
        make_stash(|repo| {
            let mut options = StashApplyOptions::new();
            options.progress_cb(|progress| {
                println!("{:?}", progress);
                true
            });

            repo.stash_apply(0, Some(&mut options)).unwrap();
            assert!(count_stash(repo) == 1)
        })
    }

    #[test]
    fn test_stash_save2_msg_none() {
        let (_td, mut repo) = repo_init();
        let signature = repo.signature().unwrap();

        let p = Path::new(repo.workdir().unwrap()).join("file_b.txt");

        fs::write(&p, "data".as_bytes()).unwrap();

        repo.stash_save2(&signature, None, Some(StashFlags::INCLUDE_UNTRACKED))
            .unwrap();

        let mut stash_name = String::new();
        repo.stash_foreach(|index, name, _oid| {
            assert_eq!(index, 0);
            stash_name = name.to_string();
            true
        })
        .unwrap();

        assert!(stash_name.starts_with("WIP on main:"));
    }

    fn create_file(r: &Repository, name: &str, data: &str) -> PathBuf {
        let p = Path::new(r.workdir().unwrap()).join(name);
        fs::write(&p, data).unwrap();
        p
    }

    #[test]
    fn test_stash_save_ext() {
        let (_td, mut repo) = repo_init();
        let signature = repo.signature().unwrap();

        create_file(&repo, "file_a", "foo");
        create_file(&repo, "file_b", "foo");

        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();

        assert_eq!(repo.statuses(None).unwrap().len(), 2);

        let mut opt = StashSaveOptions::new(signature);
        opt.pathspec("file_a");
        repo.stash_save_ext(Some(&mut opt)).unwrap();

        assert_eq!(repo.statuses(None).unwrap().len(), 0);

        repo.stash_pop(0, None).unwrap();

        assert_eq!(repo.statuses(None).unwrap().len(), 1);
    }
}
