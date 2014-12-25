use std::kinds::marker;

use {raw, StatusEntry, Delta, Oid};

/// Description of changes to one entry.
pub struct DiffDelta<'a> {
    raw: *mut raw::git_diff_delta,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

/// Description of one side of a delta.
///
/// Although this is called a "file" it could represent a file, a symbolic
/// link, a submodule commit id, or even a tree (although that only happens if
/// you are tracking type changes or ignored/untracked directories).
pub struct DiffFile<'a> {
    raw: *const raw::git_diff_file,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'a> DiffDelta<'a> {
    /// Create a new diff delta from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_entry: &StatusEntry<'a>,
                           raw: *mut raw::git_diff_delta) -> DiffDelta<'a> {
        DiffDelta {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    // TODO: expose when diffs are more exposed
    // pub fn similarity(&self) -> u16 {
    //     unsafe { (*self.raw).similarity }
    // }

    /// Returns the number of files in this delta.
    pub fn nfiles(&self) -> u16 {
        unsafe { (*self.raw).nfiles }
    }

    /// Returns the status of this entry
    ///
    /// For more information, see `Delta`'s documentation
    pub fn status(&self) -> Delta {
        match unsafe { (*self.raw).status } {
            raw::GIT_DELTA_UNMODIFIED => Delta::Unmodified,
            raw::GIT_DELTA_ADDED => Delta::Added,
            raw::GIT_DELTA_DELETED => Delta::Deleted,
            raw::GIT_DELTA_MODIFIED => Delta::Modified,
            raw::GIT_DELTA_RENAMED => Delta::Renamed,
            raw::GIT_DELTA_COPIED => Delta::Copied,
            raw::GIT_DELTA_IGNORED => Delta::Ignored,
            raw::GIT_DELTA_UNTRACKED => Delta::Untracked,
            raw::GIT_DELTA_TYPECHANGE => Delta::Typechange,
            raw::GIT_DELTA_UNREADABLE => Delta::Unreadable,
        }
    }

    /// Return the file which represents the "from" side of the diff.
    ///
    /// What side this means depends on the function that was used to generate
    /// the diff and will be documented on the function itself.
    pub fn old_file(&self) -> DiffFile<'a> {
        unsafe { DiffFile::from_raw(self, &(*self.raw).old_file) }
    }

    /// Return the file which represents the "to" side of the diff.
    ///
    /// What side this means depends on the function that was used to generate
    /// the diff and will be documented on the function itself.
    pub fn new_file(&self) -> DiffFile<'a> {
        unsafe { DiffFile::from_raw(self, &(*self.raw).new_file) }
    }
}

impl<'a> DiffFile<'a> {
    /// Create a new diff delta from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_entry: &DiffDelta<'a>,
                           raw: *const raw::git_diff_file) -> DiffFile<'a> {
        DiffFile {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Returns the Oid of this item.
    ///
    /// If this entry represents an absent side of a diff (e.g. the `old_file`
    /// of a `Added` delta), then the oid returned will be zeroes.
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(&(*self.raw).id) }
    }

    /// Returns the path, in bytes, of the entry relative to the working
    /// directory of the repository.
    pub fn path_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, (*self.raw).path) }
    }

    /// Returns the path of the entry relative to the working directory of the
    /// repository.
    pub fn path(&self) -> Option<Path> {
        self.path_bytes().map(Path::new)
    }

    /// Returns the size of this entry, in bytes
    pub fn size(&self) -> u64 { unsafe { (*self.raw).size as u64 } }

    // TODO: expose flags/mode
}
