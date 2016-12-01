use {raw, panic, StashFlags, Oid, Error, Repository, Signature, StashApplyProgress};
use std::ffi::{CString, CStr};
use util::{Binding};
use libc::{c_int, c_char, size_t, c_void, c_uint};
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
    raw_opts: raw::git_stash_apply_options
}

impl<'cb> StashApplyOptions<'cb> {
    /// Creates a default set of merge options.
    pub fn new() -> StashApplyOptions<'cb> {
        let mut opts = StashApplyOptions {
            progress: None,
            raw_opts: unsafe { mem::zeroed() },
        };
        assert_eq!(unsafe {
            raw::git_stash_apply_init_options(&mut opts.raw_opts, 1)
        }, 0);
        opts
    }

    /// Set stash application flag
    pub fn flags(&mut self, flags: raw::git_stash_apply_flags) -> &mut StashApplyOptions<'cb> {
        self.raw_opts.flags = flags;
        self
    }

    /// Options to use when writing files to the working directory
    pub fn checkout_options(&mut self, mut opts: CheckoutBuilder) -> &mut StashApplyOptions<'cb> {
        unsafe {
            opts.configure(&mut self.raw_opts.checkout_options);
        }
        self
    }

    /// Optional callback to notify the consumer of application progress.
    ///
    /// Return `true` to continue processing, or `false` to
    /// abort the stash application.
    pub fn progress_cb(&mut self, callback: Box<StashApplyProgressCb<'cb>>) -> &mut StashApplyOptions<'cb> {
        self.progress = Some(callback);
        self.raw_opts.progress_cb = stash_apply_progress_cb;
        self.raw_opts.progress_payload = self as *mut _ as *mut _;
        self
    }

    /// Pointer to a raw git_stash_apply_options
    fn raw(&self) -> &raw::git_stash_apply_options {
        &self.raw_opts
    }
}

/// A set of stash methods
pub struct Stash {

}

impl Stash {
    /// Save the local modifications to a new stash.
    pub fn save(repo: &mut Repository,
                stasher: &Signature,
                message: &str,
                flags: Option<StashFlags>)
                -> Result<Oid, Error> {
        unsafe {
            let mut raw_oid = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
            let message = try!(CString::new(message));
            let flags = flags.unwrap_or(StashFlags::empty());
            try_call!(raw::git_stash_save(
                &mut raw_oid,
                repo.raw(),
                stasher.raw(),
                message,
                flags.bits() as c_uint
            ));

            Ok(Binding::from_raw(&raw_oid as *const _))
        }
    }

    /// Apply a single stashed state from the stash list.
    pub fn apply(repo: &mut Repository,
                 index: usize,
                 opts: Option<&mut StashApplyOptions>)
                 -> Result<(), Error> {
        unsafe {
            let opts = opts.map(|opts| opts.raw());
            try_call!(raw::git_stash_apply(
                repo.raw(),
                index,
                opts
            ));

            Ok(())
        }
    }

    /// Loop over all the stashed states and issue a callback for each one.
    ///
    /// Return `true` to continue iterating or `false` to stop.
    pub fn foreach(repo: &mut Repository, callback: Box<StashCb>) -> Result<(), Error> {
        unsafe {
            let mut data = Box::new(StashCbData { callback: callback });
            try_call!(raw::git_stash_foreach(
                repo.raw(),
                stash_cb,
                &mut *data as *mut _ as *mut _
            ));
            Ok(())
        }
    }

    /// Remove a single stashed state from the stash list.
    pub fn drop(repo: &mut Repository, index: usize) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_stash_drop(
                repo.raw(),
                index
            ));
            Ok(())
        }
    }

    /// Apply a single stashed state from the stash list and remove it from the list if successful.
    pub fn pop(repo: &mut Repository,
               index: usize,
               opts: Option<&mut StashApplyOptions>)
               -> Result<(), Error> {
        unsafe {
            let opts = opts.map(|opts| opts.raw());
            try_call!(raw::git_stash_pop(
                repo.raw(),
                index,
                opts
            ));
            Ok(())
        }
    }
}

#[allow(unused)]
struct StashCbData<'a> {
    callback: Box<StashCb<'a>>
}

#[allow(unused)]
extern fn stash_cb(index: size_t,
                   message: *const c_char,
                   stash_id: *const raw::git_oid,
                   payload: *mut c_void)
                   -> c_int
{
    panic::wrap(|| unsafe {
        let mut data = &mut *(payload as *mut StashCbData);
        let res = {
            let mut callback = &mut data.callback;
            callback(
                index,
                CStr::from_ptr(message).to_str().unwrap(),
                &Binding::from_raw(stash_id)
            )
        };

        if res { 0 } else { 1 }
    }).unwrap()
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
    }).unwrap()
}

#[cfg(test)]
mod tests {
    use stash::{Stash, StashApplyOptions};
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

        Stash::save(&mut repo, &signature, "msg1", Some(STASH_INCLUDE_UNTRACKED)).unwrap();

        assert!(repo.status_file(&rel_p).is_err());

        let mut count = 0;
        Stash::foreach(&mut repo, Box::new(|index, name, _oid| {
            count += 1;
            assert!(index == 0);
            assert!(name == "On master: msg1");
            true
        })).unwrap();

        assert!(count == 1);
        next(&mut repo);
    }

    fn count_stash(repo: &mut Repository) -> usize {
        let mut count = 0;
        Stash::foreach(repo, Box::new(|_, _, _| { count += 1; true })).unwrap();
        count
    }

    #[test]
    fn smoke_stash_save_drop() {
        make_stash(|repo| {
            Stash::drop(repo, 0).unwrap();
            assert!(count_stash(repo) == 0)
        })
    }

    #[test]
    fn smoke_stash_save_pop() {
        make_stash(|repo| {
            Stash::pop(repo, 0, None).unwrap();
            assert!(count_stash(repo) == 0)
        })
    }

    #[test]
    fn smoke_stash_save_apply() {
        make_stash(|repo| {
            let mut options = StashApplyOptions::new();
            options.progress_cb(Box::new(|progress| {
                println!("{:?}", progress);
                true
            }));

            Stash::apply(repo, 0, Some(&mut options)).unwrap();
            assert!(count_stash(repo) == 1)
        })
    }
}