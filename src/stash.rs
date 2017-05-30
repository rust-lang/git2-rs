use {raw, panic, Oid, StashApplyProgress};
use std::ffi::{CStr};
use util::{Binding};
use libc::{c_int, c_char, size_t, c_void};
use build::{CheckoutBuilder};
use std::mem;

/// Stash application progress notification function.
///
/// Return `true` to continue processing, or `false` to
/// abort the stash application.
pub type StashApplyProgressCb<'a> = FnMut(StashApplyProgress) -> bool + 'a;

/// This is a callback function you can provide to iterate over all the
/// stashed states that will be invoked per entry.
pub type StashCb<'a> = FnMut(usize, &str, &Oid) -> bool + 'a;

#[allow(unused)]
/// Stash application options structure
pub struct StashApplyOptions<'cb> {
    progress: Option<Box<StashApplyProgressCb<'cb>>>,
    checkout_options: Option<CheckoutBuilder<'cb>>,
    raw_opts: raw::git_stash_apply_options
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
        assert_eq!(unsafe {
            raw::git_stash_apply_init_options(&mut opts.raw_opts, 1)
        }, 0);
        opts
    }

    /// Set stash application flag to GIT_STASH_APPLY_REINSTATE_INDEX
    pub fn reinstantiate_index(&mut self) -> &mut StashApplyOptions<'cb> {
        self.raw_opts.flags = raw::GIT_STASH_APPLY_REINSTATE_INDEX;
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
        where C: FnMut(StashApplyProgress) -> bool + 'cb
    {
        self.progress = Some(Box::new(callback) as Box<StashApplyProgressCb<'cb>>);
        self.raw_opts.progress_cb = stash_apply_progress_cb;
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

#[allow(unused)]
pub struct StashCbData<'a> {
    pub callback: &'a mut StashCb<'a>
}

#[allow(unused)]
pub extern fn stash_cb(index: size_t,
                        message: *const c_char,
                        stash_id: *const raw::git_oid,
                        payload: *mut c_void)
                        -> c_int
{
    panic::wrap(|| unsafe {
        let mut data = &mut *(payload as *mut StashCbData);
        let res = {
            let mut callback = &mut data.callback;
            callback(index,
                     CStr::from_ptr(message).to_str().unwrap(),
                     &Binding::from_raw(stash_id))
        };

        if res { 0 } else { 1 }
    }).unwrap_or(1)
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

        _ => StashApplyProgress::None
    }
}

#[allow(unused)]
extern fn stash_apply_progress_cb(progress: raw::git_stash_apply_progress_t,
                                  payload: *mut c_void)
                                  -> c_int
{
    panic::wrap(|| unsafe {
        let mut options = &mut *(payload as *mut StashApplyOptions);
        let res = {
            let mut callback = options.progress.as_mut().unwrap();
            callback(convert_progress(progress))
        };

        if res { 0 } else { -1 }
    }).unwrap_or(-1)
}

#[cfg(test)]
mod tests {
    use stash::{StashApplyOptions};
    use std::io::{Write};
    use std::fs;
    use std::path::Path;
    use test::{repo_init};
    use {Repository, STATUS_WT_NEW, STASH_INCLUDE_UNTRACKED};

    fn make_stash<C>(next: C) where C: FnOnce(&mut Repository) {
        let (_td, mut repo) = repo_init();
        let signature = repo.signature().unwrap();

        let p = Path::new(repo.workdir().unwrap()).join("file_b.txt");
        println!("using path {:?}", p);
        fs::File::create(&p).unwrap()
            .write("data".as_bytes()).unwrap();

        let rel_p = Path::new("file_b.txt");
        assert!(repo.status_file(&rel_p).unwrap() == STATUS_WT_NEW);

        repo.stash_save(&signature, "msg1", Some(STASH_INCLUDE_UNTRACKED)).unwrap();

        assert!(repo.status_file(&rel_p).is_err());

        let mut count = 0;
        repo.stash_foreach(|index, name, _oid| {
            count += 1;
            assert!(index == 0);
            assert!(name == "On master: msg1");
            true
        }).unwrap();

        assert!(count == 1);
        next(&mut repo);
    }

    fn count_stash(repo: &mut Repository) -> usize {
        let mut count = 0;
        repo.stash_foreach(|_, _, _| { count += 1; true }).unwrap();
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
}
