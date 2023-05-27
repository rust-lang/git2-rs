use libc::{c_char, c_uint, size_t};
use std::ffi::CString;
use std::iter::FusedIterator;
use std::marker;
use std::mem;
use std::ops::Range;
use std::str;

use crate::util::{self, Binding};
use crate::{raw, DiffDelta, IntoCString, Repository, Status};

/// Options that can be provided to `repo.statuses()` to control how the status
/// information is gathered.
pub struct StatusOptions {
    raw: raw::git_status_options,
    pathspec: Vec<CString>,
    ptrs: Vec<*const c_char>,
}

/// Enumeration of possible methods of what can be shown through a status
/// operation.
#[derive(Copy, Clone)]
pub enum StatusShow {
    /// Only gives status based on HEAD to index comparison, not looking at
    /// working directory changes.
    Index,

    /// Only gives status based on index to working directory comparison, not
    /// comparing the index to the HEAD.
    Workdir,

    /// The default, this roughly matches `git status --porcelain` regarding
    /// which files are included and in what order.
    IndexAndWorkdir,
}

/// A container for a list of status information about a repository.
///
/// Each instance appears as if it were a collection, having a length and
/// allowing indexing, as well as providing an iterator.
pub struct Statuses<'repo> {
    raw: *mut raw::git_status_list,

    // Hm, not currently present, but can't hurt?
    _marker: marker::PhantomData<&'repo Repository>,
}

/// An iterator over the statuses in a `Statuses` instance.
pub struct StatusIter<'statuses> {
    statuses: &'statuses Statuses<'statuses>,
    range: Range<usize>,
}

/// A structure representing an entry in the `Statuses` structure.
///
/// Instances are created through the `.iter()` method or the `.get()` method.
pub struct StatusEntry<'statuses> {
    raw: *const raw::git_status_entry,
    _marker: marker::PhantomData<&'statuses DiffDelta<'statuses>>,
}

impl Default for StatusOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusOptions {
    /// Creates a new blank set of status options.
    pub fn new() -> StatusOptions {
        unsafe {
            let mut raw = mem::zeroed();
            let r = raw::git_status_init_options(&mut raw, raw::GIT_STATUS_OPTIONS_VERSION);
            assert_eq!(r, 0);
            StatusOptions {
                raw,
                pathspec: Vec::new(),
                ptrs: Vec::new(),
            }
        }
    }

    /// Select the files on which to report status.
    ///
    /// The default, if unspecified, is to show the index and the working
    /// directory.
    pub fn show(&mut self, show: StatusShow) -> &mut StatusOptions {
        self.raw.show = match show {
            StatusShow::Index => raw::GIT_STATUS_SHOW_INDEX_ONLY,
            StatusShow::Workdir => raw::GIT_STATUS_SHOW_WORKDIR_ONLY,
            StatusShow::IndexAndWorkdir => raw::GIT_STATUS_SHOW_INDEX_AND_WORKDIR,
        };
        self
    }

    /// Add a path pattern to match (using fnmatch-style matching).
    ///
    /// If the `disable_pathspec_match` option is given, then this is a literal
    /// path to match. If this is not called, then there will be no patterns to
    /// match and the entire directory will be used.
    pub fn pathspec<T: IntoCString>(&mut self, pathspec: T) -> &mut StatusOptions {
        let s = util::cstring_to_repo_path(pathspec).unwrap();
        self.ptrs.push(s.as_ptr());
        self.pathspec.push(s);
        self
    }

    fn flag(&mut self, flag: raw::git_status_opt_t, val: bool) -> &mut StatusOptions {
        if val {
            self.raw.flags |= flag as c_uint;
        } else {
            self.raw.flags &= !(flag as c_uint);
        }
        self
    }

    /// Flag whether untracked files will be included.
    ///
    /// Untracked files will only be included if the workdir files are included
    /// in the status "show" option.
    pub fn include_untracked(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_INCLUDE_UNTRACKED, include)
    }

    /// Flag whether ignored files will be included.
    ///
    /// The files will only be included if the workdir files are included
    /// in the status "show" option.
    pub fn include_ignored(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_INCLUDE_IGNORED, include)
    }

    /// Flag to include unmodified files.
    pub fn include_unmodified(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_INCLUDE_UNMODIFIED, include)
    }

    /// Flag that submodules should be skipped.
    ///
    /// This only applies if there are no pending typechanges to the submodule
    /// (either from or to another type).
    pub fn exclude_submodules(&mut self, exclude: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_EXCLUDE_SUBMODULES, exclude)
    }

    /// Flag that all files in untracked directories should be included.
    ///
    /// Normally if an entire directory is new then just the top-level directory
    /// is included (with a trailing slash on the entry name).
    pub fn recurse_untracked_dirs(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RECURSE_UNTRACKED_DIRS, include)
    }

    /// Indicates that the given paths should be treated as literals paths, note
    /// patterns.
    pub fn disable_pathspec_match(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_DISABLE_PATHSPEC_MATCH, include)
    }

    /// Indicates that the contents of ignored directories should be included in
    /// the status.
    pub fn recurse_ignored_dirs(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RECURSE_IGNORED_DIRS, include)
    }

    /// Indicates that rename detection should be processed between the head.
    pub fn renames_head_to_index(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RENAMES_HEAD_TO_INDEX, include)
    }

    /// Indicates that rename detection should be run between the index and the
    /// working directory.
    pub fn renames_index_to_workdir(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RENAMES_INDEX_TO_WORKDIR, include)
    }

    /// Override the native case sensitivity for the file system and force the
    /// output to be in case sensitive order.
    pub fn sort_case_sensitively(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_SORT_CASE_SENSITIVELY, include)
    }

    /// Override the native case sensitivity for the file system and force the
    /// output to be in case-insensitive order.
    pub fn sort_case_insensitively(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_SORT_CASE_INSENSITIVELY, include)
    }

    /// Indicates that rename detection should include rewritten files.
    pub fn renames_from_rewrites(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RENAMES_FROM_REWRITES, include)
    }

    /// Bypasses the default status behavior of doing a "soft" index reload.
    pub fn no_refresh(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_NO_REFRESH, include)
    }

    /// Refresh the stat cache in the index for files are unchanged but have
    /// out of date stat information in the index.
    ///
    /// This will result in less work being done on subsequent calls to fetching
    /// the status.
    pub fn update_index(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_UPDATE_INDEX, include)
    }

    // erm...
    #[allow(missing_docs)]
    pub fn include_unreadable(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_INCLUDE_UNREADABLE, include)
    }

    // erm...
    #[allow(missing_docs)]
    pub fn include_unreadable_as_untracked(&mut self, include: bool) -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_INCLUDE_UNREADABLE_AS_UNTRACKED, include)
    }

    /// Set threshold above which similar files will be considered renames.
    ///
    /// This is equivalent to the `-M` option. Defaults to 50.
    pub fn rename_threshold(&mut self, threshold: u16) -> &mut StatusOptions {
        self.raw.rename_threshold = threshold;
        self
    }

    /// Get a pointer to the inner list of status options.
    ///
    /// This function is unsafe as the returned structure has interior pointers
    /// and may no longer be valid if these options continue to be mutated.
    pub unsafe fn raw(&mut self) -> *const raw::git_status_options {
        self.raw.pathspec.strings = self.ptrs.as_ptr() as *mut _;
        self.raw.pathspec.count = self.ptrs.len() as size_t;
        &self.raw
    }
}

impl<'repo> Statuses<'repo> {
    /// Gets a status entry from this list at the specified index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: usize) -> Option<StatusEntry<'_>> {
        unsafe {
            let p = raw::git_status_byindex(self.raw, index as size_t);
            Binding::from_raw_opt(p)
        }
    }

    /// Gets the count of status entries in this list.
    ///
    /// If there are no changes in status (according to the options given
    /// when the status list was created), this should return 0.
    pub fn len(&self) -> usize {
        unsafe { raw::git_status_list_entrycount(self.raw) as usize }
    }

    /// Return `true` if there is no status entry in this list.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the statuses in this list.
    pub fn iter(&self) -> StatusIter<'_> {
        StatusIter {
            statuses: self,
            range: 0..self.len(),
        }
    }
}

impl<'repo> Binding for Statuses<'repo> {
    type Raw = *mut raw::git_status_list;
    unsafe fn from_raw(raw: *mut raw::git_status_list) -> Statuses<'repo> {
        Statuses {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_status_list {
        self.raw
    }
}

impl<'repo> Drop for Statuses<'repo> {
    fn drop(&mut self) {
        unsafe {
            raw::git_status_list_free(self.raw);
        }
    }
}

impl<'a> Iterator for StatusIter<'a> {
    type Item = StatusEntry<'a>;
    fn next(&mut self) -> Option<StatusEntry<'a>> {
        self.range.next().and_then(|i| self.statuses.get(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}
impl<'a> DoubleEndedIterator for StatusIter<'a> {
    fn next_back(&mut self) -> Option<StatusEntry<'a>> {
        self.range.next_back().and_then(|i| self.statuses.get(i))
    }
}
impl<'a> FusedIterator for StatusIter<'a> {}
impl<'a> ExactSizeIterator for StatusIter<'a> {}

impl<'a> IntoIterator for &'a Statuses<'a> {
    type Item = StatusEntry<'a>;
    type IntoIter = StatusIter<'a>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl<'statuses> StatusEntry<'statuses> {
    /// Access the bytes for this entry's corresponding pathname
    pub fn path_bytes(&self) -> &[u8] {
        unsafe {
            if (*self.raw).head_to_index.is_null() {
                crate::opt_bytes(self, (*(*self.raw).index_to_workdir).old_file.path)
            } else {
                crate::opt_bytes(self, (*(*self.raw).head_to_index).old_file.path)
            }
            .unwrap()
        }
    }

    /// Access this entry's path name as a string.
    ///
    /// Returns `None` if the path is not valid utf-8.
    pub fn path(&self) -> Option<&str> {
        str::from_utf8(self.path_bytes()).ok()
    }

    /// Access the status flags for this file
    pub fn status(&self) -> Status {
        Status::from_bits_truncate(unsafe { (*self.raw).status as u32 })
    }

    /// Access detailed information about the differences between the file in
    /// HEAD and the file in the index.
    pub fn head_to_index(&self) -> Option<DiffDelta<'statuses>> {
        unsafe { Binding::from_raw_opt((*self.raw).head_to_index) }
    }

    /// Access detailed information about the differences between the file in
    /// the index and the file in the working directory.
    pub fn index_to_workdir(&self) -> Option<DiffDelta<'statuses>> {
        unsafe { Binding::from_raw_opt((*self.raw).index_to_workdir) }
    }
}

impl<'statuses> Binding for StatusEntry<'statuses> {
    type Raw = *const raw::git_status_entry;

    unsafe fn from_raw(raw: *const raw::git_status_entry) -> StatusEntry<'statuses> {
        StatusEntry {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_status_entry {
        self.raw
    }
}

#[cfg(test)]
mod tests {
    use super::StatusOptions;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;

    #[test]
    fn smoke() {
        let (td, repo) = crate::test::repo_init();
        assert_eq!(repo.statuses(None).unwrap().len(), 0);
        File::create(&td.path().join("foo")).unwrap();
        let statuses = repo.statuses(None).unwrap();
        assert_eq!(statuses.iter().count(), 1);
        let status = statuses.iter().next().unwrap();
        assert_eq!(status.path(), Some("foo"));
        assert!(status.status().contains(crate::Status::WT_NEW));
        assert!(!status.status().contains(crate::Status::INDEX_NEW));
        assert!(status.head_to_index().is_none());
        let diff = status.index_to_workdir().unwrap();
        assert_eq!(diff.old_file().path_bytes().unwrap(), b"foo");
        assert_eq!(diff.new_file().path_bytes().unwrap(), b"foo");
    }

    #[test]
    fn filter() {
        let (td, repo) = crate::test::repo_init();
        t!(File::create(&td.path().join("foo")));
        t!(File::create(&td.path().join("bar")));
        let mut opts = StatusOptions::new();
        opts.include_untracked(true).pathspec("foo");

        let statuses = t!(repo.statuses(Some(&mut opts)));
        assert_eq!(statuses.iter().count(), 1);
        let status = statuses.iter().next().unwrap();
        assert_eq!(status.path(), Some("foo"));
    }

    #[test]
    fn gitignore() {
        let (td, repo) = crate::test::repo_init();
        t!(t!(File::create(td.path().join(".gitignore"))).write_all(b"foo\n"));
        assert!(!t!(repo.status_should_ignore(Path::new("bar"))));
        assert!(t!(repo.status_should_ignore(Path::new("foo"))));
    }

    #[test]
    fn status_file() {
        let (td, repo) = crate::test::repo_init();
        assert!(repo.status_file(Path::new("foo")).is_err());
        if cfg!(windows) {
            assert!(repo.status_file(Path::new("bar\\foo.txt")).is_err());
        }
        t!(File::create(td.path().join("foo")));
        if cfg!(windows) {
            t!(::std::fs::create_dir_all(td.path().join("bar")));
            t!(File::create(td.path().join("bar").join("foo.txt")));
        }
        let status = t!(repo.status_file(Path::new("foo")));
        assert!(status.contains(crate::Status::WT_NEW));
        if cfg!(windows) {
            let status = t!(repo.status_file(Path::new("bar\\foo.txt")));
            assert!(status.contains(crate::Status::WT_NEW));
        }
    }
}
