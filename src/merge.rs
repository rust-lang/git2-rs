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
pub struct MergeOptions;

impl MergeOptions {
    /// Creates a default set of merge options.
    pub fn new() -> MergeOptions {
        MergeOptions
    }

    /// Creates a set of raw merge options to be used with
    /// `git_merge`.
    ///
    /// This function is unsafe as the pointer is only valid so long as this
    /// structure is not moved, modified, or used elsewhere.
    pub unsafe fn raw(&self) -> raw::git_merge_options {
        let mut opts = mem::zeroed();
        assert_eq!(raw::git_merge_init_options(&mut opts, raw::GIT_MERGE_OPTIONS_VERSION), 0);
        return opts;
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
