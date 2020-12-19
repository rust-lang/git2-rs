use libc::c_uint;
use std::marker;
use std::mem;
use std::str;

use crate::call::Convert;
use crate::util::Binding;
use crate::{raw, Commit, FileFavor, FileMode, IntoCString, Oid};
use core::{ptr, slice};
use std::convert::TryInto;
use std::ffi::{CStr, CString};

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

/// Options for merging files
pub struct MergeFileOptions {
    /// Label for the ancestor file side of the conflict which will be prepended
    /// to labels in diff3-format merge files.
    ancestor_label: Option<CString>,

    /// Label for our file side of the conflict which will be prepended
    /// to labels in merge files.
    our_label: Option<CString>,

    /// Label for their file side of the conflict which will be prepended
    /// to labels in merge files.
    their_label: Option<CString>,

    // raw data
    raw: raw::git_merge_file_options,
}

impl Default for MergeFileOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl MergeFileOptions {
    /// Creates a default set of merge options.
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

    /// Specify ancestor label, default is "ancestor"
    pub fn ancestor_label<T: IntoCString>(&mut self, t: T) -> &mut MergeFileOptions {
        self.ancestor_label = Some(t.into_c_string().unwrap());

        self.raw.ancestor_label = self
            .ancestor_label
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        self
    }

    /// Specify ancestor label, default is "ours"
    pub fn our_label<T: IntoCString>(&mut self, t: T) -> &mut MergeFileOptions {
        self.our_label = Some(t.into_c_string().unwrap());

        self.raw.our_label = self
            .our_label
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        self
    }

    /// Specify ancestor label, default is "theirs"
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
    pub fn file_favor(&mut self, favor: FileFavor) -> &mut MergeFileOptions {
        self.raw.favor = favor.convert();
        self
    }

    /// Specify marker size, default is 7: <<<<<<< ours
    pub fn marker_size(&mut self, size: u16) -> &mut MergeFileOptions {
        self.raw.marker_size = size;
        self
    }

    /// Acquire a pointer to the underlying raw options.
    pub unsafe fn raw(&self) -> *const raw::git_merge_file_options {
        &self.raw as *const _
    }
}

/// For git_merge_file_input
pub struct MergeFileInput<'a> {
    raw: raw::git_merge_file_input,

    /// File name of the conflicted file, or `NULL` to not merge the path.
    ///
    /// You can turn this value into a `std::ffi::CString` with
    /// `CString::new(&entry.path[..]).unwrap()`. To turn a reference into a
    /// `&std::path::Path`, see the `bytes2path()` function in the private,
    /// internal `util` module in this crate’s source code.
    path: Option<CString>,

    /// File content
    content: Option<&'a [u8]>,
}

impl Default for MergeFileInput<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> MergeFileInput<'a> {
    /// Creates a new set of empty diff options.
    pub fn new() -> MergeFileInput<'a> {
        let mut input = MergeFileInput {
            raw: unsafe { mem::zeroed() },
            path: None,
            content: None,
        };
        assert_eq!(
            unsafe { raw::git_merge_file_input_init(&mut input.raw, 1) },
            0
        );
        input
    }

    /// File name of the conflicted file, or `None` to not merge the path.
    pub fn path<T: IntoCString>(&mut self, t: T) -> &mut MergeFileInput<'a> {
        self.path = Some(t.into_c_string().unwrap());

        self.raw.path = self
            .path
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());

        self
    }

    /// File mode of the conflicted file, or `0` to not merge the mode.
    pub fn mode(&mut self, mode: Option<FileMode>) -> &mut MergeFileInput<'a> {
        if let Some(mode) = mode {
            self.raw.mode = mode as u32;
        }

        self
    }

    /// File content, text or binary
    pub fn content(&mut self, content: Option<&'a [u8]>) -> &mut MergeFileInput<'a> {
        self.content = content;

        self.raw.size = self.content.as_ref().map(|c| c.len()).unwrap_or(0);
        self.raw.ptr = self
            .content
            .as_ref()
            .map(|c| c.as_ptr() as *const _)
            .unwrap_or(ptr::null());

        self
    }

    /// Get the raw struct in C
    pub fn raw(&self) -> *const raw::git_merge_file_input {
        &self.raw as *const _
    }
}

impl std::fmt::Debug for MergeFileInput<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut ds = f.debug_struct("MergeFileInput");
        if let Some(path) = &self.path {
            ds.field("path", path);
        }
        ds.field("mode", &FileMode::from(self.raw.mode.try_into().unwrap()));

        match FileMode::from(self.raw.mode.try_into().unwrap()) {
            FileMode::Unreadable => {}
            FileMode::Tree => {}
            FileMode::Blob => {
                let content = self
                    .content
                    .as_ref()
                    .map(|s| String::from_utf8_lossy(&s).to_string())
                    .unwrap_or("unknown content".to_string());

                ds.field("content", &content);
            }
            FileMode::BlobExecutable => {}
            FileMode::Link => {}
            FileMode::Commit => {}
        }
        ds.finish()
    }
}

/// For git_merge_file_result
pub struct MergeFileResult {
    raw: raw::git_merge_file_result,
}

impl MergeFileResult {
    /// Create MergeFileResult from C
    pub unsafe fn from_raw(raw: raw::git_merge_file_result) -> MergeFileResult {
        MergeFileResult { raw }
    }

    /// True if the output was automerged, false if the output contains
    /// conflict markers.
    pub fn automergeable(&self) -> bool {
        self.raw.automergeable > 0
    }

    /// The path that the resultant merge file should use, or NULL if a
    /// filename conflict would occur.
    pub unsafe fn path(&self) -> Option<String> {
        let c_str: &CStr = CStr::from_ptr(self.raw.path);
        let str_slice: &str = c_str.to_str().unwrap();
        let path: String = str_slice.to_owned();
        Some(path)
    }

    /// The mode that the resultant merge file should use.
    pub fn mode(&self) -> FileMode {
        FileMode::from(self.raw.mode.try_into().unwrap())
    }

    /// The contents of the merge.
    pub unsafe fn content(&self) -> Option<Vec<u8>> {
        let content =
            slice::from_raw_parts(self.raw.ptr as *const u8, self.raw.len as usize).to_vec();
        Some(content)
    }
}

impl std::fmt::Debug for MergeFileResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut ds = f.debug_struct("MergeFileResult");
        unsafe {
            if let Some(path) = &self.path() {
                ds.field("path", path);
            }
        }
        ds.field("mode", &self.mode());

        match self.mode() {
            FileMode::Unreadable => {}
            FileMode::Tree => {}
            FileMode::Blob => unsafe {
                let content = self
                    .content()
                    .as_ref()
                    .map(|c| String::from_utf8_unchecked(c.clone()))
                    .unwrap_or("unknown content".to_string());
                ds.field("content", &content);
            },
            FileMode::BlobExecutable => {}
            FileMode::Link => {}
            FileMode::Commit => {}
        }

        ds.finish()
    }
}
