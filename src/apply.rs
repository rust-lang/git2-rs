//! git_apply support
//! see original: <https://github.com/libgit2/libgit2/blob/master/include/git2/apply.h>

use crate::{panic, raw, util::Binding, DiffDelta, DiffHunk};
use libc::c_int;
use std::{ffi::c_void, mem};

/// Possible application locations for git_apply
/// see <https://libgit2.org/libgit2/#HEAD/type/git_apply_options>
#[derive(Copy, Clone, Debug)]
pub enum ApplyLocation {
    /// Apply the patch to the workdir
    WorkDir,
    /// Apply the patch to the index
    Index,
    /// Apply the patch to both the working directory and the index
    Both,
}

impl Binding for ApplyLocation {
    type Raw = raw::git_apply_location_t;
    unsafe fn from_raw(raw: raw::git_apply_location_t) -> Self {
        match raw {
            raw::GIT_APPLY_LOCATION_WORKDIR => Self::WorkDir,
            raw::GIT_APPLY_LOCATION_INDEX => Self::Index,
            raw::GIT_APPLY_LOCATION_BOTH => Self::Both,
            _ => panic!("Unknown git diff binary kind"),
        }
    }
    fn raw(&self) -> raw::git_apply_location_t {
        match *self {
            Self::WorkDir => raw::GIT_APPLY_LOCATION_WORKDIR,
            Self::Index => raw::GIT_APPLY_LOCATION_INDEX,
            Self::Both => raw::GIT_APPLY_LOCATION_BOTH,
        }
    }
}

/// Options to specify when applying a diff
pub struct ApplyOptions<'cb> {
    raw: raw::git_apply_options,
    hunk_cb: Option<Box<HunkCB<'cb>>>,
    delta_cb: Option<Box<DeltaCB<'cb>>>,
}

type HunkCB<'a> = dyn FnMut(Option<DiffHunk<'_>>) -> bool + 'a;
type DeltaCB<'a> = dyn FnMut(Option<DiffDelta<'_>>) -> bool + 'a;

extern "C" fn delta_cb_c(delta: *const raw::git_diff_delta, data: *mut c_void) -> c_int {
    panic::wrap(|| unsafe {
        let delta = Binding::from_raw_opt(delta as *mut _);

        let payload = &mut *(data as *mut ApplyOptions<'_>);
        let callback = match payload.delta_cb {
            Some(ref mut c) => c,
            None => return -1,
        };

        let apply = callback(delta);
        if apply {
            0
        } else {
            1
        }
    })
    .unwrap_or(-1)
}

extern "C" fn hunk_cb_c(hunk: *const raw::git_diff_hunk, data: *mut c_void) -> c_int {
    panic::wrap(|| unsafe {
        let hunk = Binding::from_raw_opt(hunk);

        let payload = &mut *(data as *mut ApplyOptions<'_>);
        let callback = match payload.hunk_cb {
            Some(ref mut c) => c,
            None => return -1,
        };

        let apply = callback(hunk);
        if apply {
            0
        } else {
            1
        }
    })
    .unwrap_or(-1)
}

impl<'cb> ApplyOptions<'cb> {
    /// Creates a new set of empty options (zeroed).
    pub fn new() -> Self {
        let mut opts = Self {
            raw: unsafe { mem::zeroed() },
            hunk_cb: None,
            delta_cb: None,
        };
        assert_eq!(
            unsafe { raw::git_apply_options_init(&mut opts.raw, raw::GIT_APPLY_OPTIONS_VERSION) },
            0
        );
        opts
    }

    fn flag(&mut self, opt: raw::git_apply_flags_t, val: bool) -> &mut Self {
        let opt = opt as u32;
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Don't actually make changes, just test that the patch applies.
    pub fn check(&mut self, check: bool) -> &mut Self {
        self.flag(raw::GIT_APPLY_CHECK, check)
    }

    /// When applying a patch, callback that will be made per hunk.
    pub fn hunk_callback<F>(&mut self, cb: F) -> &mut Self
    where
        F: FnMut(Option<DiffHunk<'_>>) -> bool + 'cb,
    {
        self.hunk_cb = Some(Box::new(cb) as Box<HunkCB<'cb>>);

        self.raw.hunk_cb = Some(hunk_cb_c);
        self.raw.payload = self as *mut _ as *mut _;

        self
    }

    /// When applying a patch, callback that will be made per delta (file).
    pub fn delta_callback<F>(&mut self, cb: F) -> &mut Self
    where
        F: FnMut(Option<DiffDelta<'_>>) -> bool + 'cb,
    {
        self.delta_cb = Some(Box::new(cb) as Box<DeltaCB<'cb>>);

        self.raw.delta_cb = Some(delta_cb_c);
        self.raw.payload = self as *mut _ as *mut _;

        self
    }

    /// Pointer to a raw git_stash_apply_options
    pub unsafe fn raw(&mut self) -> *const raw::git_apply_options {
        &self.raw as *const _
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, io::Write, path::Path};

    #[test]
    fn smoke_test() {
        let (_td, repo) = crate::test::repo_init();
        let diff = t!(repo.diff_tree_to_workdir(None, None));
        let mut count_hunks = 0;
        let mut count_delta = 0;
        {
            let mut opts = ApplyOptions::new();
            opts.hunk_callback(|_hunk| {
                count_hunks += 1;
                true
            });
            opts.delta_callback(|_delta| {
                count_delta += 1;
                true
            });
            t!(repo.apply(&diff, ApplyLocation::Both, Some(&mut opts)));
        }
        assert_eq!(count_hunks, 0);
        assert_eq!(count_delta, 0);
    }

    #[test]
    fn apply_hunks_and_delta() {
        let file_path = Path::new("foo.txt");
        let (td, repo) = crate::test::repo_init();
        // create new file
        t!(t!(File::create(&td.path().join(file_path))).write_all(b"bar"));
        // stage the new file
        t!(t!(repo.index()).add_path(file_path));
        // now change workdir version
        t!(t!(File::create(&td.path().join(file_path))).write_all(b"foo\nbar"));

        let diff = t!(repo.diff_index_to_workdir(None, None));
        assert_eq!(diff.deltas().len(), 1);
        let mut count_hunks = 0;
        let mut count_delta = 0;
        {
            let mut opts = ApplyOptions::new();
            opts.hunk_callback(|_hunk| {
                count_hunks += 1;
                true
            });
            opts.delta_callback(|_delta| {
                count_delta += 1;
                true
            });
            t!(repo.apply(&diff, ApplyLocation::Index, Some(&mut opts)));
        }
        assert_eq!(count_delta, 1);
        assert_eq!(count_hunks, 1);
    }
}
