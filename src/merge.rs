use libc::{c_uint, c_ushort};
use std::ffi::CString;
use std::marker;
use std::mem;
use std::ptr;
use std::str;

use crate::call::Convert;
use crate::util::Binding;
use crate::IntoCString;
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

/// Options for merging a file.
pub struct MergeFileOptions {
    ancestor_label: Option<CString>,
    our_label: Option<CString>,
    their_label: Option<CString>,
    raw: raw::git_merge_file_options,
}

/// Information about file-level merging.
pub struct MergeFileResult {
    raw: raw::git_merge_file_result,
}

impl<'repo> AnnotatedCommit<'repo> {
    /// Gets the commit ID that the given git_annotated_commit refers to
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_annotated_commit_id(self.raw)) }
    }

    /// Get the refname that the given git_annotated_commit refers to
    ///
    /// Returns None if it is not valid utf8
    pub fn refname(&self) -> Option<&str> {
        str::from_utf8(self.refname_bytes()).ok()
    }

    /// Get the refname that the given git_annotated_commit refers to.
    pub fn refname_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_annotated_commit_ref(&*self.raw)).unwrap() }
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

    fn flag(&mut self, opt: u32, val: bool) -> &mut MergeOptions {
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Detect file renames
    pub fn find_renames(&mut self, find: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FIND_RENAMES as u32, find)
    }

    /// If a conflict occurs, exit immediately instead of attempting to continue
    /// resolving conflicts
    pub fn fail_on_conflict(&mut self, fail: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FAIL_ON_CONFLICT as u32, fail)
    }

    /// Do not write the REUC extension on the generated index
    pub fn skip_reuc(&mut self, skip: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_FAIL_ON_CONFLICT as u32, skip)
    }

    /// If the commits being merged have multiple merge bases, do not build a
    /// recursive merge base (by merging the multiple merge bases), instead
    /// simply use the first base.
    pub fn no_recursive(&mut self, disable: bool) -> &mut MergeOptions {
        self.flag(raw::GIT_MERGE_NO_RECURSIVE as u32, disable)
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

    fn file_flag(&mut self, opt: u32, val: bool) -> &mut MergeOptions {
        if val {
            self.raw.file_flags |= opt;
        } else {
            self.raw.file_flags &= !opt;
        }
        self
    }

    /// Create standard conflicted merge files
    pub fn standard_style(&mut self, standard: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_STYLE_MERGE as u32, standard)
    }

    /// Create diff3-style file
    pub fn diff3_style(&mut self, diff3: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_STYLE_DIFF3 as u32, diff3)
    }

    /// Condense non-alphanumeric regions for simplified diff file
    pub fn simplify_alnum(&mut self, simplify: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_SIMPLIFY_ALNUM as u32, simplify)
    }

    /// Ignore all whitespace
    pub fn ignore_whitespace(&mut self, ignore: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE as u32, ignore)
    }

    /// Ignore changes in amount of whitespace
    pub fn ignore_whitespace_change(&mut self, ignore: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE as u32, ignore)
    }

    /// Ignore whitespace at end of line
    pub fn ignore_whitespace_eol(&mut self, ignore: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL as u32, ignore)
    }

    /// Use the "patience diff" algorithm
    pub fn patience(&mut self, patience: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_DIFF_PATIENCE as u32, patience)
    }

    /// Take extra time to find minimal diff
    pub fn minimal(&mut self, minimal: bool) -> &mut MergeOptions {
        self.file_flag(raw::GIT_MERGE_FILE_DIFF_MINIMAL as u32, minimal)
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
            raw,
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

impl Default for MergeFileOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeFileOptions {
    /// Creates a default set of merge file options.
    pub fn new() -> MergeFileOptions {
        let mut opts = MergeFileOptions {
            ancestor_label: None,
            our_label: None,
            their_label: None,
            raw: unsafe { mem::zeroed() },
        };
        assert_eq!(
            unsafe { raw::git_merge_file_options_init(&mut opts.raw, 1) },
            0
        );
        opts
    }

    /// Label for the ancestor file side of the conflict which will be prepended
    /// to labels in diff3-format merge files.
    pub fn ancestor_label<T: IntoCString>(&mut self, t: T) -> &mut MergeFileOptions {
        self.ancestor_label = Some(t.into_c_string().unwrap());

        self.raw.ancestor_label = self
            .ancestor_label
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        self
    }

    /// Label for our file side of the conflict which will be prepended to labels
    /// in merge files.
    pub fn our_label<T: IntoCString>(&mut self, t: T) -> &mut MergeFileOptions {
        self.our_label = Some(t.into_c_string().unwrap());

        self.raw.our_label = self
            .our_label
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        self
    }

    /// Label for their file side of the conflict which will be prepended to labels
    /// in merge files.
    pub fn their_label<T: IntoCString>(&mut self, t: T) -> &mut MergeFileOptions {
        self.their_label = Some(t.into_c_string().unwrap());

        self.raw.their_label = self
            .their_label
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        self
    }

    /// Specify a side to favor for resolving conflicts
    pub fn favor(&mut self, favor: FileFavor) -> &mut MergeFileOptions {
        self.raw.favor = favor.convert();
        self
    }

    fn flag(&mut self, opt: raw::git_merge_file_flag_t, val: bool) -> &mut MergeFileOptions {
        if val {
            self.raw.flags |= opt as u32;
        } else {
            self.raw.flags &= !opt as u32;
        }
        self
    }

    /// Create standard conflicted merge files
    pub fn style_standard(&mut self, standard: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_STYLE_MERGE, standard)
    }

    /// Create diff3-style file
    pub fn style_diff3(&mut self, diff3: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_STYLE_DIFF3, diff3)
    }

    /// Condense non-alphanumeric regions for simplified diff file
    pub fn simplify_alnum(&mut self, simplify: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_SIMPLIFY_ALNUM, simplify)
    }

    /// Ignore all whitespace
    pub fn ignore_whitespace(&mut self, ignore: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE, ignore)
    }

    /// Ignore changes in amount of whitespace
    pub fn ignore_whitespace_change(&mut self, ignore: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_CHANGE, ignore)
    }

    /// Ignore whitespace at end of line
    pub fn ignore_whitespace_eol(&mut self, ignore: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_IGNORE_WHITESPACE_EOL, ignore)
    }

    /// Use the "patience diff" algorithm
    pub fn patience(&mut self, patience: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_DIFF_PATIENCE, patience)
    }

    /// Take extra time to find minimal diff
    pub fn minimal(&mut self, minimal: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_DIFF_MINIMAL, minimal)
    }

    /// Create zdiff3 ("zealous diff3")-style files
    pub fn style_zdiff3(&mut self, zdiff3: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_STYLE_ZDIFF3, zdiff3)
    }

    /// Do not produce file conflicts when common regions have changed
    pub fn accept_conflicts(&mut self, accept: bool) -> &mut MergeFileOptions {
        self.flag(raw::GIT_MERGE_FILE_ACCEPT_CONFLICTS, accept)
    }

    /// The size of conflict markers (eg, "<<<<<<<"). Default is 7.
    pub fn marker_size(&mut self, size: u16) -> &mut MergeFileOptions {
        self.raw.marker_size = size as c_ushort;
        self
    }

    /// Acquire a pointer to the underlying raw options.
    ///
    /// # Safety
    /// The pointer used here (or its contents) should not outlive self.
    pub(crate) unsafe fn raw(&mut self) -> *const raw::git_merge_file_options {
        &self.raw
    }
}

impl MergeFileResult {
    /// True if the output was automerged, false if the output contains
    /// conflict markers.
    pub fn is_automergeable(&self) -> bool {
        self.raw.automergeable > 0
    }

    /// The path that the resultant merge file should use.
    ///
    /// returns `None` if a filename conflict would occur,
    /// or if the path is not valid utf-8
    pub fn path(&self) -> Option<&str> {
        self.path_bytes()
            .and_then(|bytes| str::from_utf8(bytes).ok())
    }

    /// Gets the path as a byte slice.
    pub fn path_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, self.raw.path) }
    }

    /// The mode that the resultant merge file should use.
    pub fn mode(&self) -> u32 {
        self.raw.mode as u32
    }

    /// The contents of the merge.
    pub fn content(&self) -> &[u8] {
        unsafe { std::slice::from_raw_parts(self.raw.ptr as *const u8, self.raw.len as usize) }
    }
}

impl Binding for MergeFileResult {
    type Raw = raw::git_merge_file_result;
    unsafe fn from_raw(raw: raw::git_merge_file_result) -> MergeFileResult {
        MergeFileResult { raw }
    }
    fn raw(&self) -> raw::git_merge_file_result {
        unimplemented!()
    }
}

impl Drop for MergeFileResult {
    fn drop(&mut self) {
        unsafe { raw::git_merge_file_result_free(&mut self.raw) }
    }
}

impl std::fmt::Debug for MergeFileResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut ds = f.debug_struct("MergeFileResult");
        if let Some(path) = &self.path() {
            ds.field("path", path);
        }
        ds.field("automergeable", &self.is_automergeable());
        ds.field("mode", &self.mode());
        ds.finish()
    }
}
