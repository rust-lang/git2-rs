use std::marker;
use std::mem;

use {raw, Object};
use util::Binding;

/// annotated commits, the input to merge and rebase
pub struct AnnotatedCommit<'repo> {
    raw: *mut raw::git_annotated_commit,
    _marker: marker::PhantomData<Object<'repo>>,
}

/// merge options
// modeled after DiffFindOptions
pub struct MergeOptions {
    raw: raw::git_merge_options,
}

impl MergeOptions {
    /// Creates a default set of merge options.
    pub fn new() -> MergeOptions {
        let mut opts = MergeOptions {
            raw: unsafe { mem::zeroed() },
        };
        assert_eq!(unsafe {
            raw::git_merge_init_options(&mut opts.raw, 1)
        }, 0);
        opts
    }

    /// Acquire a pointer to the underlying raw options.
    ///
    /// This function is unsafe as the pointer is only valid so long as this
    /// structure is not moved, modified, or used elsewhere.
    // modeled after DiffOptions.raw()
    pub unsafe fn raw(&self) -> *const raw::git_merge_options {
        &self.raw as *const _
    }
}

impl<'repo> Binding for AnnotatedCommit<'repo> {
    type Raw = *mut raw::git_annotated_commit;
    unsafe fn from_raw(raw: *mut raw::git_annotated_commit) -> AnnotatedCommit<'repo> {
        AnnotatedCommit {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_annotated_commit { self.raw }
}

impl<'repo> Drop for AnnotatedCommit<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_annotated_commit_free(self.raw) }
    }
}
