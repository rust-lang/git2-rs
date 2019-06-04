use libc::c_uint;
use std::marker;
use std::mem;

use crate::call::Convert;
use crate::util::Binding;
use crate::{raw, Commit, FileFavor, Oid};

/// A structure to represent an annotated commit, the input to merge and rebase.
///
/// An annotated commit contains information about how it was looked up, which
/// may be useful for functions like merge or rebase to provide context to the
/// operation.
pub struct AnnotatedCommit<'repo> {
    raw: *mut raw::git_annotated_commit,
    _marker: marker::PhantomData<Commit<'repo>>,
}

/// Options to specify when merging.
pub struct MergeOptions {
    raw: raw::git_merge_options,
}

impl<'repo> AnnotatedCommit<'repo> {
    /// Gets the commit ID that the given git_annotated_commit refers to
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_annotated_commit_id(self.raw)) }
    }
}

impl Default for MergeOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeOptions {
    /// Creates a default set of merge options.
    pub fn new() -> MergeOptions {
        let mut opts = MergeOptions {
            raw: unsafe { mem::zeroed() },
        };
        assert_eq!(unsafe { raw::git_merge_init_options(&mut opts.raw, 1) }, 0);
        opts
    }

    fn flag(&mut self, opt: raw::git_merge_flag_t, val: bool) -> &mut MergeOptions {
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Detect file renames
    pub fn find_renames(&mut self, find: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FIND_RENAMES, find)
    }

    /// If a conflict occurs, exit immediately instead of attempting to continue
    /// resolving conflicts
    pub fn fail_on_conflict(&mut self, fail: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FAIL_ON_CONFLICT, fail)
    }

    /// Do not write the REUC extension on the generated index
    pub fn skip_reuc(&mut self, skip: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FAIL_ON_CONFLICT, skip)
    }

    /// If the commits being merged have multiple merge bases, do not build a
    /// recursive merge base (by merging the multiple merge bases), instead
    /// simply use the first base.
    pub fn no_recursive(&mut self, disable: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_NO_RECURSIVE, disable)
    }

    /// Similarity to consider a file renamed (default 50)
    pub fn rename_threshold(&mut self, thresh: u32) -> &mut MergeOptions {
        self.raw.rename_threshold = thresh;
        self
    }

    ///  Maximum similarity sources to examine for renames (default 200).
    /// If the number of rename candidates (add / delete pairs) is greater
    /// than this value, inexact rename detection is aborted. This setting
    /// overrides the `merge.renameLimit` configuration value.
    pub fn target_limit(&mut self, limit: u32) -> &mut MergeOptions {
        self.raw.target_limit = limit as c_uint;
        self
    }

    /// Maximum number of times to merge common ancestors to build a
    /// virtual merge base when faced with criss-cross merges.  When
    /// this limit is reached, the next ancestor will simply be used
    /// instead of attempting to merge it.  The default is unlimited.
    pub fn recursion_limit(&mut self, limit: u32) -> &mut MergeOptions {
        self.raw.recursion_limit = limit as c_uint;
        self
    }

    /// Specify a side to favor for resolving conflicts
    pub fn file_favor(&mut self, favor: FileFavor) -> &mut MergeOptions {
        self.raw.file_favor = favor.convert();
        self
    }

    fn file_flag(&mut self, opt: raw::git_merge_file_flag_t, val: bool) -> &mut MergeOptions {
        if val {
            self.raw.file_flags |= opt;
        } else {
            self.raw.file_flags &= !opt;
        }
        self
    }

    /// Create standard conflicted merge files
    pub fn standard_style(&mut self, standard: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_STYLE_MERGE, standard)
    }

    /// Create diff3-style file
    pub fn diff3_style(&mut self, diff3: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_STYLE_DIFF3, diff3)
    }

    /// Condense non-alphanumeric regions for simplified diff file
    pub fn simplify_alnum(&mut self, simplify: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_SIMPLIFY_ALNUM, simplify)
    }

    /// Ignore all whitespace
    pub fn ignore_whitespace(&mut self, ignore: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE, ignore)
    }

    /// Ignore changes in amount of whitespace
    pub fn ignore_whitespace_change(&mut self, ignore: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE, ignore)
    }

    /// Ignore whitespace at end of line
    pub fn ignore_whitespace_eol(&mut self, ignore: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL, ignore)
    }

    /// Use the "patience diff" algorithm
    pub fn patience(&mut self, patience: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_DIFF_PATIENCE, patience)
    }

    /// Take extra time to find minimal diff
    pub fn minimal(&mut self, minimal: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_DIFF_MINIMAL, minimal)
    }

    /// Acquire a pointer to the underlying raw options.
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
    fn raw(&self) -> *mut raw::git_annotated_commit {
        self.raw
    }
}

impl<'repo> Drop for AnnotatedCommit<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_annotated_commit_free(self.raw) }
    }
}
