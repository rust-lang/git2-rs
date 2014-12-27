use std::c_str::CString;
use std::iter::{range, Range};
use std::kinds::marker;
use std::mem;
use std::str;
use libc::{c_char, size_t, c_uint};

use {raw, Status, DiffDelta};

/// Options that can be provided to `repo.statuses()` to control how the status
/// information is gathered.
pub struct StatusOptions {
    raw: raw::git_status_options,
    pathspec: Vec<CString>,
    ptrs: Vec<*const c_char>,
}

/// Enumeration of possible methods of what can be shown through a status
/// operation.
#[deriving(Copy)]
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
/// Each instances appears as a if it were a collection, having a length and
/// allowing indexing as well as provding an iterator.
pub struct Statuses<'repo> {
    raw: *mut raw::git_status_list,
    marker1: marker::ContravariantLifetime<'repo>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

/// An iterator over the statuses in a `Statuses` instance.
pub struct StatusIter<'statuses> {
    statuses: &'statuses Statuses<'statuses>,
    range: Range<uint>,
}

/// A structure representing an entry in the `Statuses` structure.
///
/// Instances are created through the `.iter()` method or the `.get()` method.
pub struct StatusEntry<'statuses> {
    raw: *const raw::git_status_entry,
    marker1: marker::ContravariantLifetime<'statuses>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl StatusOptions {
    /// Creates a new blank set of status options.
    pub fn new() -> StatusOptions {
        unsafe {
            let mut raw = mem::zeroed();
            let r = raw::git_status_init_options(&mut raw,
                                raw::GIT_STATUS_OPTIONS_VERSION);
            assert_eq!(r, 0);
            StatusOptions {
                raw: raw,
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
    pub fn pathspec<T: ToCStr>(&mut self, pathspec: T) -> &mut StatusOptions {
        let s = pathspec.to_c_str();
        self.ptrs.push(s.as_ptr());
        self.pathspec.push(s);
        self
    }

    fn flag(&mut self, flag: raw::git_status_opt_t, val: bool)
            -> &mut StatusOptions {
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
    pub fn recurse_untracked_dirs(&mut self, include: bool)
                                  -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RECURSE_UNTRACKED_DIRS, include)
    }

    /// Indicates that the given paths should be treated as literals paths, note
    /// patterns.
    pub fn disable_pathspec_match(&mut self, include: bool)
                                  -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_DISABLE_PATHSPEC_MATCH, include)
    }

    /// Indicates that the contents of ignored directories should be included in
    /// the status.
    pub fn recurse_ignored_dirs(&mut self, include: bool)
                                -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RECURSE_IGNORED_DIRS, include)
    }

    /// Indicates that rename detection should be processed between the head.
    pub fn renames_head_to_index(&mut self, include: bool)
                                 -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RENAMES_HEAD_TO_INDEX, include)
    }

    /// Indicates that rename detection should be run between the index and the
    /// working directory.
    pub fn renames_index_to_workdir(&mut self, include: bool)
                                    -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_RENAMES_INDEX_TO_WORKDIR, include)
    }

    /// Override the native case sensitivity for the file system and force the
    /// output to be in case sensitive order.
    pub fn sort_case_sensitively(&mut self, include: bool)
                                 -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_SORT_CASE_SENSITIVELY, include)
    }

    /// Override the native case sensitivity for the file system and force the
    /// output to be in case-insensitive order.
    pub fn sort_case_insensitively(&mut self, include: bool)
                                   -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_SORT_CASE_INSENSITIVELY, include)
    }

    /// Indicates that rename detection should include rewritten files.
    pub fn renames_from_rewrites(&mut self, include: bool)
                                 -> &mut StatusOptions {
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
    pub fn include_unreadable_as_untracked(&mut self, include: bool)
                                           -> &mut StatusOptions {
        self.flag(raw::GIT_STATUS_OPT_INCLUDE_UNREADABLE_AS_UNTRACKED, include)
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
    /// Create a new statuses iterator from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_status_list) -> Statuses<'repo> {
        Statuses {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Gets a status entry from this list at the specified index.
    ///
    /// Returns `None` if the index is out of bounds.
    pub fn get(&self, index: uint) -> Option<StatusEntry> {
        unsafe {
            let p = raw::git_status_byindex(self.raw, index as size_t);
            if p.is_null() {
                None
            } else {
                Some(StatusEntry::from_raw(p))
            }
        }
    }

    /// Gets the count of status entries in this list.
    ///
    /// If there are no changes in status (at least according the options given
    /// when the status list was created), this can return 0.
    pub fn len(&self) -> uint {
        unsafe { raw::git_status_list_entrycount(self.raw) as uint }
    }

    /// Returns an iterator over the statuses in this list.
    pub fn iter(&self) -> StatusIter {
        StatusIter {
            statuses: self,
            range: range(0, self.len()),
        }
    }
}

#[unsafe_destructor]
impl<'repo> Drop for Statuses<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_status_list_free(self.raw); }
    }
}

impl<'a> Iterator<StatusEntry<'a>> for StatusIter<'a> {
    fn next(&mut self) -> Option<StatusEntry<'a>> {
        self.range.next().and_then(|i| self.statuses.get(i))
    }
    fn size_hint(&self) -> (uint, Option<uint>) { self.range.size_hint() }
}

impl<'a> DoubleEndedIterator<StatusEntry<'a>> for StatusIter<'a> {
    fn next_back(&mut self) -> Option<StatusEntry<'a>> {
        self.range.next_back().and_then(|i| self.statuses.get(i))
    }
}

impl<'a> ExactSizeIterator<StatusEntry<'a>> for StatusIter<'a> {}

impl<'statuses> StatusEntry<'statuses> {
    /// Create a new status entry from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *const raw::git_status_entry)
                           -> StatusEntry<'statuses> {
        StatusEntry {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Access the bytes for this entry's corresponding pathname
    pub fn path_bytes(&self) -> &[u8] {
        unsafe {
            if (*self.raw).head_to_index.is_null() {
                ::opt_bytes(self, (*(*self.raw).index_to_workdir).old_file.path)
            } else {
                ::opt_bytes(self, (*(*self.raw).head_to_index).old_file.path)
            }.unwrap()
        }
    }

    /// Access this entry's path name as a string.
    ///
    /// Returns `None` if the path is not valid utf-8.
    pub fn path(&self) -> Option<&str> { str::from_utf8(self.path_bytes()).ok() }

    /// Access the status flags for this file
    pub fn status(&self) -> Status {
        Status::from_bits_truncate(unsafe { (*self.raw).status as u32 })
    }

    /// Access detailed information about the differences between the file in
    /// HEAD and the file in the index.
    pub fn head_to_index(&self) -> Option<DiffDelta<'statuses>> {
        unsafe {
            let p = (*self.raw).head_to_index;
            if p.is_null() {
                None
            } else {
                Some(DiffDelta::from_raw(p))
            }
        }
    }

    /// Access detailed information about the differences between the file in
    /// the index and the file in the working directory.
    pub fn index_to_workdir(&self) -> Option<DiffDelta<'statuses>> {
        unsafe {
            let p = (*self.raw).index_to_workdir;
            if p.is_null() {
                None
            } else {
                Some(DiffDelta::from_raw(p))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::File;

    #[test]
    fn smoke() {
        let (td, repo) = ::test::repo_init();
        assert_eq!(repo.statuses(None).unwrap().len(), 0);
        File::create(&td.path().join("foo")).unwrap();
        let statuses = repo.statuses(None).unwrap();
        assert_eq!(statuses.iter().count(), 1);
        let status = statuses.iter().next().unwrap();
        assert_eq!(status.path(), Some("foo"));
        assert!(status.status().contains(::STATUS_WT_NEW));
        assert!(!status.status().contains(::STATUS_INDEX_NEW));
        assert!(status.head_to_index().is_none());
        let diff = status.index_to_workdir().unwrap();
        assert_eq!(diff.old_file().path_bytes().unwrap(), b"foo");
        assert_eq!(diff.new_file().path_bytes().unwrap(), b"foo");
    }
}
