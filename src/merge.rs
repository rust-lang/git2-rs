use std::marker;
use std::mem;
use libc::c_uint;

use {raw, Oid, Commit, FileFavor};
use util::Binding;
use call::Convert;

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

bitflags! {
    /// The results of `merge_analysis` indicating the merge opportunities.
    pub struct MergeAnalysis: u32 {
        /// No merge is possible.
        const MERGE_ANALYSIS_NONE = raw::GIT_MERGE_ANALYSIS_NONE as u32;
        /// A "normal" merge; both HEAD and the given merge input have diverged
        /// from their common ancestor. The divergent commits must be merged.
        const MERGE_ANALYSIS_NORMAL = raw::GIT_MERGE_ANALYSIS_NORMAL as u32;
        /// All given merge inputs are reachable from HEAD, meaning the
        /// repository is up-to-date and no merge needs to be performed.
        const MERGE_ANALYSIS_UP_TO_DATE = raw::GIT_MERGE_ANALYSIS_UP_TO_DATE as u32;   
        /// The given merge input is a fast-forward from HEAD and no merge
        /// needs to be performed.  Instead, the client can check out the
        /// given merge input.
        const MERGE_ANALYSIS_FASTFORWARD = raw::GIT_MERGE_ANALYSIS_FASTFORWARD as u32;
        /// The HEAD of the current repository is "unborn" and does not point to
        /// a valid commit.  No merge can be performed, but the caller may wish
        /// to simply set HEAD to the target commit(s).
        const MERGE_ANALYSIS_UNBORN = raw::GIT_MERGE_ANALYSIS_UNBORN as u32;
    }
}

bitflags! {
    /// The user's stated preference for merges.
    pub struct MergePreference: u32 {
        /// No configuration was found that suggests a preferred behavior for
        /// merge.
        const MERGE_PREFERENCE_NONE = raw::GIT_MERGE_PREFERENCE_NONE;
        /// There is a `merge.ff=false` configuration setting, suggesting that
        /// the user does not want to allow a fast-forward merge.
        const MERGE_PREFERENCE_NO_FAST_FORWARD = raw::GIT_MERGE_PREFERENCE_NO_FASTFORWARD;
        /// There is a `merge.ff=only` configuration setting, suggesting that
        /// the user only wants fast-forward merges.
        const MERGE_PREFERENCE_FASTFORWARD_ONLY = raw::GIT_MERGE_PREFERENCE_FASTFORWARD_ONLY;
    }
}

impl<'repo> AnnotatedCommit<'repo> {
    /// Gets the commit ID that the given git_annotated_commit refers to
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_annotated_commit_id(self.raw)) }
    }
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

    /// Detect file renames
    pub fn find_renames(&mut self, find: bool) -> &mut MergeOptions {
        if find {
            self.raw.flags |= raw::GIT_MERGE_FIND_RENAMES;
        } else {
            self.raw.flags &= !raw::GIT_MERGE_FIND_RENAMES;
        }
        self
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

    fn flag(&mut self, opt: raw::git_merge_file_flag_t, val: bool) -> &mut MergeOptions {
        if val {
            self.raw.file_flags |= opt;
        } else {
            self.raw.file_flags &= !opt;
        }
        self
    }

    /// Create standard conflicted merge files
    pub fn standard_style(&mut self, standard: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_STYLE_MERGE, standard)
    }

    /// Create diff3-style file
    pub fn diff3_style(&mut self, diff3: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_STYLE_DIFF3, diff3)
    }

    /// Condense non-alphanumeric regions for simplified diff file
    pub fn simplify_alnum(&mut self, simplify: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_SIMPLIFY_ALNUM, simplify)
    }

    /// Ignore all whitespace
    pub fn ignore_whitespace(&mut self, ignore: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE, ignore)
    }

    /// Ignore changes in amount of whitespace
    pub fn ignore_whitespace_change(&mut self, ignore: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE, ignore)
    }

    /// Ignore whitespace at end of line
    pub fn ignore_whitespace_eol(&mut self, ignore: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL, ignore)
    }

    /// Use the "patience diff" algorithm
    pub fn patience(&mut self, patience: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_DIFF_PATIENCE, patience)
    }

    /// Take extra time to find minimal diff
    pub fn minimal(&mut self, minimal: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FILE_DIFF_MINIMAL, minimal)
    }

    /// Acquire a pointer to the underlying raw options.
    pub unsafe fn raw(&self) -> *const raw::git_merge_options {
        &self.raw as *const _
    }
}

impl<'repo> Binding for AnnotatedCommit<'repo> {
    type Raw = *mut raw::git_annotated_commit;
    unsafe fn from_raw(raw: *mut raw::git_annotated_commit)
                       -> AnnotatedCommit<'repo> {
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
