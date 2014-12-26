use std::c_str::CString;
use std::iter::Range;
use std::kinds::marker;
use std::mem;
use std::path::PosixPath;

use libc;

use {raw, Repository, Error, Tree, Oid, IndexAddOption, IndexTime};

/// A structure to represent a git [index][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Index {
    raw: *mut raw::git_index,
    marker: marker::NoSync,
}

/// An iterator over the entries in an index
pub struct IndexEntries<'index> {
    range: Range<uint>,
    index: &'index Index,
}

/// A callback function to filter index matches.
///
/// Used by `Index::{add_all,remove_all,update_all}`.  The first argument is the
/// path, and the second is the patchspec that matched it.  Return 0 to confirm
/// the operation on the item, > 0 to skip the item, and < 0 to abort the scan.
pub type IndexMatchedPath<'a> = |&[u8], &[u8]|: 'a -> int;

/// A structure to represent an entry or a file inside of an index.
///
/// All fields of an entry are public for modification and inspection. This is
/// also how a new index entry is created.
#[allow(missing_docs)]
pub struct IndexEntry {
    pub ctime: IndexTime,
    pub mtime: IndexTime,
    pub dev: uint,
    pub ino: uint,
    pub mode: uint,
    pub uid: uint,
    pub gid: uint,
    pub file_size: u64,
    pub id: Oid,
    pub flags: u16,
    pub flags_extended: u16,
    pub path: CString,
}

impl Index {
    /// Creates a new in-memory index.
    ///
    /// This index object cannot be read/written to the filesystem, but may be
    /// used to perform in-memory index operations.
    pub fn new() -> Result<Index, Error> {
        ::init();
        let mut raw = 0 as *mut raw::git_index;
        unsafe {
            try_call!(raw::git_index_new(&mut raw));
            Ok(Index::from_raw(raw))
        }
    }

    /// Create a new bare Git index object as a memory representation of the Git
    /// index file in 'index_path', without a repository to back it.
    ///
    /// Since there is no ODB or working directory behind this index, any Index
    /// methods which rely on these (e.g. add_path) will fail.
    ///
    /// If you need an index attached to a repository, use the `index()` method
    /// on `Repository`.
    pub fn open(index_path: &Path) -> Result<Index, Error> {
        ::init();
        let mut raw = 0 as *mut raw::git_index;
        unsafe {
            try_call!(raw::git_index_open(&mut raw, index_path.to_c_str()));
            Ok(Index::from_raw(raw))
        }
    }

    /// Creates a new index from a raw pointer.
    ///
    /// This function is unsafe as it cannot guarantee the validity of `raw`.
    pub unsafe fn from_raw(raw: *mut raw::git_index) -> Index {
        Index { raw: raw, marker: marker::NoSync }
    }

    /// Add or update an index entry from an in-memory struct
    ///
    /// If a previous index entry exists that has the same path and stage as the
    /// given 'source_entry', it will be replaced. Otherwise, the 'source_entry'
    /// will be added.
    pub fn add(&mut self, source_entry: &IndexEntry) -> Result<(), Error> {
        let mut entry: raw::git_index_entry = unsafe { mem::zeroed() };
        source_entry.configure(&mut entry);
        unsafe {
            try_call!(raw::git_index_add(self.raw, &entry));
            Ok(())
        }
    }

    /// Add or update an index entry from a file on disk
    ///
    /// The file path must be relative to the repository's working folder and
    /// must be readable.
    ///
    /// This method will fail in bare index instances.
    ///
    /// This forces the file to be added to the index, not looking at gitignore
    /// rules.
    ///
    /// If this file currently is the result of a merge conflict, this file will
    /// no longer be marked as conflicting. The data about the conflict will be
    /// moved to the "resolve undo" (REUC) section.
    pub fn add_path(&mut self, path: &Path) -> Result<(), Error> {
        // Git apparently expects '/' to be separators for paths
        let mut posix_path = PosixPath::new(".");
        for comp in path.components() {
            posix_path.push(comp);
        }
        unsafe {
            try_call!(raw::git_index_add_bypath(self.raw, posix_path.to_c_str()));
            Ok(())
        }
    }

    /// Add or update index entries matching files in the working directory.
    ///
    /// This method will fail in bare index instances.
    ///
    /// The `pathspecs` are a list of file names or shell glob patterns that
    /// will matched against files in the repository's working directory. Each
    /// file that matches will be added to the index (either updating an
    /// existing entry or adding a new entry). You can disable glob expansion
    /// and force exact matching with the `AddDisablePathspecMatch` flag.
    ///
    /// Files that are ignored will be skipped (unlike `add_path`). If a file is
    /// already tracked in the index, then it will be updated even if it is
    /// ignored. Pass the `AddForce` flag to skip the checking of ignore rules.
    ///
    /// To emulate `git add -A` and generate an error if the pathspec contains
    /// the exact path of an ignored file (when not using `AddForce`), add the
    /// `AddCheckPathspec` flag. This checks that each entry in `pathspecs`
    /// that is an exact match to a filename on disk is either not ignored or
    /// already in the index. If this check fails, the function will return
    /// an error.
    ///
    /// To emulate `git add -A` with the "dry-run" option, just use a callback
    /// function that always returns a positive value. See below for details.
    ///
    /// If any files are currently the result of a merge conflict, those files
    /// will no longer be marked as conflicting. The data about the conflicts
    /// will be moved to the "resolve undo" (REUC) section.
    ///
    /// If you provide a callback function, it will be invoked on each matching
    /// item in the working directory immediately before it is added to /
    /// updated in the index. Returning zero will add the item to the index,
    /// greater than zero will skip the item, and less than zero will abort the
    /// scan an return an error to the caller.
    pub fn add_all<T: ToCStr>(&mut self,
                              pathspecs: &[T],
                              flag: IndexAddOption,
                              mut cb: Option<IndexMatchedPath>)
                              -> Result<(), Error> {
        let arr = pathspecs.iter().map(|t| t.to_c_str()).collect::<Vec<CString>>();
        let strarray = arr.iter().map(|c| c.as_ptr())
                          .collect::<Vec<*const libc::c_char>>();
        let ptr = cb.as_mut();
        let raw_strarray = raw::git_strarray {
            strings: strarray.as_ptr() as *mut _,
            count: strarray.len() as libc::size_t,
        };
        let callback = ptr.as_ref().map(|_| {
            index_matched_path_cb as raw::git_index_matched_path_cb
        });
        unsafe {
            try_call!(raw::git_index_add_all(self.raw,
                                             &raw_strarray,
                                             flag.bits() as libc::c_uint,
                                             callback,
                                             ptr.map(|p| p as *mut _)
                                                .unwrap_or(0 as *mut _)
                                                    as *mut libc::c_void));
        }
        return Ok(());
    }

    /// Get access to the underlying raw index pointer.
    pub fn raw(&self) -> *mut raw::git_index { self.raw }

    /// Clear the contents (all the entries) of an index object.
    ///
    /// This clears the index object in memory; changes must be explicitly
    /// written to disk for them to take effect persistently via `write_*`.
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_index_clear(self.raw)); }
        Ok(())
    }

    /// Get the count of entries currently in the index
    pub fn len(&self) -> uint {
        unsafe { raw::git_index_entrycount(&*self.raw) as uint }
    }

    /// Get one of the entries in the index by its position.
    pub fn get(&self, n: uint) -> Option<IndexEntry> {
        unsafe {
            let ptr = raw::git_index_get_byindex(self.raw, n as libc::size_t);
            if ptr.is_null() {None} else {Some(IndexEntry::from_raw(ptr))}
        }
    }

    /// Get an iterator over the entries in this index.
    pub fn iter(&self) -> IndexEntries {
        IndexEntries { range: range(0, self.len()), index: self }
    }

    /// Get one of the entries in the index by its path.
    pub fn get_path(&self, path: &Path, stage: int) -> Option<IndexEntry> {
        unsafe {
            let ptr = call!(raw::git_index_get_bypath(self.raw, path.to_c_str(),
                                                      stage as libc::c_int));
            if ptr.is_null() {None} else {Some(IndexEntry::from_raw(ptr))}
        }
    }

    /// Get the full path to the index file on disk.
    ///
    /// Returns `None` if this is an in-memory index.
    pub fn path(&self) -> Option<Path> {
        unsafe {
            ::opt_bytes(self, raw::git_index_path(&*self.raw)).map(Path::new)
        }
    }

    /// Update the contents of an existing index object in memory by reading
    /// from the hard disk.
    ///
    /// If force is true, this performs a "hard" read that discards in-memory
    /// changes and always reloads the on-disk index data. If there is no
    /// on-disk version, the index will be cleared.
    ///
    /// If force is false, this does a "soft" read that reloads the index data
    /// from disk only if it has changed since the last time it was loaded.
    /// Purely in-memory index data will be untouched. Be aware: if there are
    /// changes on disk, unwritten in-memory changes are discarded.
    pub fn read(&mut self, force: bool) -> Result<(), Error> {
        unsafe { try_call!(raw::git_index_read(self.raw, force)); }
        Ok(())
    }

    /// Read a tree into the index file with stats
    ///
    /// The current index contents will be replaced by the specified tree.
    pub fn read_tree(&mut self, tree: &Tree) -> Result<(), Error> {
        unsafe { try_call!(raw::git_index_read_tree(self.raw, &*tree.raw())); }
        Ok(())
    }

    /// Remove an entry from the index
    pub fn remove(&mut self, path: &Path, stage: int) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_index_remove(self.raw, path.to_c_str(),
                                            stage as libc::c_int));
        }
        Ok(())
    }

    /// Remove an index entry corresponding to a file on disk.
    ///
    /// The file path must be relative to the repository's working folder. It
    /// may exist.
    ///
    /// If this file currently is the result of a merge conflict, this file will
    /// no longer be marked as conflicting. The data about the conflict will be
    /// moved to the "resolve undo" (REUC) section.
    pub fn remove_path(&mut self, path: &Path) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_index_remove_bypath(self.raw, path.to_c_str()));
        }
        Ok(())
    }

    /// Remove all entries from the index under a given directory.
    pub fn remove_dir(&mut self, path: &Path, stage: int) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_index_remove_directory(self.raw, path.to_c_str(),
                                                      stage as libc::c_int));
        }
        Ok(())
    }

    /// Remove all matching index entries.
    ///
    /// If you provide a callback function, it will be invoked on each matching
    /// item in the index immediately before it is removed. Return 0 to remove
    /// the item, > 0 to skip the item, and < 0 to abort the scan.
    pub fn remove_all<T: ToCStr>(&mut self,
                                 pathspecs: &[T],
                                 mut cb: Option<IndexMatchedPath>)
                                 -> Result<(), Error> {
        let arr = pathspecs.iter().map(|t| t.to_c_str()).collect::<Vec<CString>>();
        let strarray = arr.iter().map(|c| c.as_ptr())
                          .collect::<Vec<*const libc::c_char>>();
        let ptr = cb.as_mut();
        let raw_strarray = raw::git_strarray {
            strings: strarray.as_ptr() as *mut _,
            count: strarray.len() as libc::size_t,
        };
        let callback = ptr.as_ref().map(|_| {
            index_matched_path_cb as raw::git_index_matched_path_cb
        });
        unsafe {
            try_call!(raw::git_index_remove_all(self.raw,
                                                &raw_strarray,
                                                callback,
                                                ptr.map(|p| p as *mut _)
                                                   .unwrap_or(0 as *mut _)
                                                        as *mut libc::c_void));
        }
        return Ok(());
    }

    /// Update all index entries to match the working directory
    ///
    /// This method will fail in bare index instances.
    ///
    /// This scans the existing index entries and synchronizes them with the
    /// working directory, deleting them if the corresponding working directory
    /// file no longer exists otherwise updating the information (including
    /// adding the latest version of file to the ODB if needed).
    ///
    /// If you provide a callback function, it will be invoked on each matching
    /// item in the index immediately before it is updated (either refreshed or
    /// removed depending on working directory state). Return 0 to proceed with
    /// updating the item, > 0 to skip the item, and < 0 to abort the scan.
    pub fn update_all<T: ToCStr>(&mut self,
                                 pathspecs: &[T],
                                 mut cb: Option<IndexMatchedPath>)
                                 -> Result<(), Error> {
        let arr = pathspecs.iter().map(|t| t.to_c_str()).collect::<Vec<CString>>();
        let strarray = arr.iter().map(|c| c.as_ptr())
                          .collect::<Vec<*const libc::c_char>>();
        let ptr = cb.as_mut();
        let raw_strarray = raw::git_strarray {
            strings: strarray.as_ptr() as *mut _,
            count: strarray.len() as libc::size_t,
        };
        let callback = ptr.as_ref().map(|_| {
            index_matched_path_cb as raw::git_index_matched_path_cb
        });
        unsafe {
            try_call!(raw::git_index_update_all(self.raw,
                                                &raw_strarray,
                                                callback,
                                                ptr.map(|p| p as *mut _)
                                                   .unwrap_or(0 as *mut _)
                                                        as *mut libc::c_void));
        }
        return Ok(());
    }

    /// Write an existing index object from memory back to disk using an atomic
    /// file lock.
    pub fn write(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_index_write(self.raw)); }
        Ok(())
    }

    /// Write the index as a tree.
    ///
    /// This method will scan the index and write a representation of its
    /// current state back to disk; it recursively creates tree objects for each
    /// of the subtrees stored in the index, but only returns the OID of the
    /// root tree. This is the OID that can be used e.g. to create a commit.
    ///
    /// The index instance cannot be bare, and needs to be associated to an
    /// existing repository.
    ///
    /// The index must not contain any file in conflict.
    pub fn write_tree(&mut self) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_index_write_tree(&mut raw, self.raw));
            Ok(Oid::from_raw(&raw))
        }
    }

    /// Write the index as a tree to the given repository
    ///
    /// This is the same as `write_tree` except that the destination repository
    /// can be chosen.
    pub fn write_tree_to(&mut self, repo: &Repository) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_index_write_tree_to(&mut raw, self.raw,
                                                   repo.raw()));
            Ok(Oid::from_raw(&raw))
        }
    }
}

extern fn index_matched_path_cb(path: *const libc::c_char,
                                matched_pathspec: *const libc::c_char,
                                payload: *mut libc::c_void) -> libc::c_int {
    unsafe {
        let path = CString::new(path, false);
        let matched_pathspec = CString::new(matched_pathspec, false);
        let payload = payload as *mut IndexMatchedPath;
        (*payload)(path.as_bytes_no_nul(),
                   matched_pathspec.as_bytes_no_nul()) as libc::c_int
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        unsafe { raw::git_index_free(self.raw) }
    }
}

impl<'index> Iterator<IndexEntry> for IndexEntries<'index> {
    fn next(&mut self) -> Option<IndexEntry> {
        self.range.next().map(|i| self.index.get(i).unwrap())
    }
}

impl IndexEntry {
    /// Creates a new entry from its raw pointer.
    pub unsafe fn from_raw(raw: *const raw::git_index_entry) -> IndexEntry {
        let raw::git_index_entry {
            ctime, mtime, dev, ino, mode, uid, gid, file_size, id, flags,
            flags_extended, path
        } = *raw;
        IndexEntry {
            dev: dev as uint,
            ino: ino as uint,
            mode: mode as uint,
            uid: uid as uint,
            gid: gid as uint,
            file_size: file_size as u64,
            id: Oid::from_raw(&id),
            flags: flags as u16,
            flags_extended: flags_extended as u16,
            path: CString::new(path, false).clone(),
            mtime: IndexTime::from_raw(&mtime),
            ctime: IndexTime::from_raw(&ctime),
        }
    }

    /// Configures a raw git entry from this entry
    pub fn configure(&self, raw: &mut raw::git_index_entry) {
        *raw = raw::git_index_entry {
            dev: self.dev as libc::c_uint,
            ino: self.ino as libc::c_uint,
            mode: self.mode as libc::c_uint,
            uid: self.uid as libc::c_uint,
            gid: self.gid as libc::c_uint,
            file_size: self.file_size as raw::git_off_t,
            id: unsafe { *self.id.raw() },
            flags: self.flags as libc::c_ushort,
            flags_extended: self.flags_extended as libc::c_ushort,
            path: self.path.as_ptr(),
            mtime: raw::git_index_time {
                seconds: self.mtime.seconds() as raw::git_time_t,
                nanoseconds: self.mtime.nanoseconds() as libc::c_uint,
            },
            ctime: raw::git_index_time {
                seconds: self.ctime.seconds() as raw::git_time_t,
                nanoseconds: self.ctime.nanoseconds() as libc::c_uint,
            },
        };
    }
}

#[cfg(test)]
mod tests {
    use std::io::{mod, fs, File, TempDir};
    use url::Url;

    use {Index, Repository, ResetType};

    #[test]
    fn smoke() {
        let mut index = Index::new().unwrap();
        assert!(index.add_path(&Path::new(".")).is_err());
        index.clear().unwrap();
        assert_eq!(index.len(), 0);
        assert!(index.get(0).is_none());
        assert!(index.path().is_none());
        assert!(index.read(true).is_err());
    }

    #[test]
    fn smoke_from_repo() {
        let (_td, repo) = ::test::repo_init();
        let mut index = repo.index().unwrap();
        assert!(index.path() == Some(repo.path().join("index")));
        Index::open(&repo.path().join("index")).unwrap();

        index.clear().unwrap();
        index.read(true).unwrap();
        index.write().unwrap();
        index.write_tree().unwrap();
        index.write_tree_to(&repo).unwrap();
    }

    #[test]
    fn add_all() {
        let (_td, repo) = ::test::repo_init();
        let mut index = repo.index().unwrap();

        let root = repo.path().dir_path();
        fs::mkdir(&root.join("foo"), io::USER_DIR).unwrap();
        File::create(&root.join("foo/bar")).unwrap();
        let mut called = false;
        index.add_all(&["foo"], ::ADD_DEFAULT, Some(|a: &[u8], b: &[u8]| {
            assert!(!called);
            called = true;
            assert_eq!(b, b"foo");
            assert_eq!(a, b"foo/bar");
            0
        })).unwrap();
        assert!(called);

        called = false;
        index.remove_all(&["."], Some(|a: &[u8], b: &[u8]| {
            assert!(!called);
            called = true;
            assert_eq!(b, b".");
            assert_eq!(a, b"foo/bar");
            0
        })).unwrap();
        assert!(called);
    }

    #[test]
    fn smoke_add() {
        let (_td, repo) = ::test::repo_init();
        let mut index = repo.index().unwrap();

        let root = repo.path().dir_path();
        fs::mkdir(&root.join("foo"), io::USER_DIR).unwrap();
        File::create(&root.join("foo/bar")).unwrap();
        index.add_path(&Path::new("foo/bar")).unwrap();
        index.write().unwrap();
        assert_eq!(index.iter().count(), 1);

        // Make sure we can use this repo somewhere else now.
        let id = index.write_tree().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        let id = repo.refname_to_id("HEAD").unwrap();
        let parent = repo.find_commit(id).unwrap();
        let commit = repo.commit(Some("HEAD"), &sig, &sig, "commit",
                                 &tree, &[&parent]).unwrap();
        let obj = repo.find_object(commit, None).unwrap();
        repo.reset(&obj, ResetType::Hard, None, None).unwrap();

        let td2 = TempDir::new("git").unwrap();
        let url = Url::from_file_path(&root).unwrap();
        let url = url.to_string();
        let repo = Repository::clone(url.as_slice(), td2.path()).unwrap();
        let obj = repo.find_object(commit, None).unwrap();
        repo.reset(&obj, ResetType::Hard, None, None).unwrap();
    }
}

