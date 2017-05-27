use std::ffi::{CStr, OsString, CString};
use std::ops::Range;
use std::path::Path;
use std::ptr;
use std::slice;

use libc::{c_int, c_uint, size_t, c_void, c_char};

use {raw, panic, Repository, Error, Tree, Oid, IndexAddOption, IndexTime};
use IntoCString;
use util::{self, Binding};

/// A structure to represent a git [index][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Index {
    raw: *mut raw::git_index,
}

/// An iterator over the entries in an index
pub struct IndexEntries<'index> {
    range: Range<usize>,
    index: &'index Index,
}

/// A callback function to filter index matches.
///
/// Used by `Index::{add_all,remove_all,update_all}`.  The first argument is the
/// path, and the second is the patchspec that matched it.  Return 0 to confirm
/// the operation on the item, > 0 to skip the item, and < 0 to abort the scan.
pub type IndexMatchedPath<'a> = FnMut(&Path, &[u8]) -> i32 + 'a;

/// A structure to represent an entry or a file inside of an index.
///
/// All fields of an entry are public for modification and inspection. This is
/// also how a new index entry is created.
#[allow(missing_docs)]
pub struct IndexEntry {
    pub ctime: IndexTime,
    pub mtime: IndexTime,
    pub dev: u32,
    pub ino: u32,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub file_size: u32,
    pub id: Oid,
    pub flags: u16,
    pub flags_extended: u16,
    pub path: Vec<u8>,
}

impl Index {
    /// Creates a new in-memory index.
    ///
    /// This index object cannot be read/written to the filesystem, but may be
    /// used to perform in-memory index operations.
    pub fn new() -> Result<Index, Error> {
        ::init();
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_index_new(&mut raw));
            Ok(Binding::from_raw(raw))
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
        let mut raw = ptr::null_mut();
        let index_path = try!(index_path.into_c_string());
        unsafe {
            try_call!(raw::git_index_open(&mut raw, index_path));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Add or update an index entry from an in-memory struct
    ///
    /// If a previous index entry exists that has the same path and stage as the
    /// given 'source_entry', it will be replaced. Otherwise, the 'source_entry'
    /// will be added.
    pub fn add(&mut self, entry: &IndexEntry) -> Result<(), Error> {
        let path = try!(CString::new(&entry.path[..]));

        // libgit2 encodes the length of the path in the lower bits of the
        // `flags` entry, so mask those out and recalculate here to ensure we
        // don't corrupt anything.
        let mut flags = entry.flags & !raw::GIT_IDXENTRY_NAMEMASK;

        if entry.path.len() < raw::GIT_IDXENTRY_NAMEMASK as usize {
            flags |= entry.path.len() as u16;
        } else {
            flags |= raw::GIT_IDXENTRY_NAMEMASK;
        }

        unsafe {
            let raw = raw::git_index_entry {
                dev: entry.dev,
                ino: entry.ino,
                mode: entry.mode,
                uid: entry.uid,
                gid: entry.gid,
                file_size: entry.file_size,
                id: *entry.id.raw(),
                flags: flags,
                flags_extended: entry.flags_extended,
                path: path.as_ptr(),
                mtime: raw::git_index_time {
                    seconds: entry.mtime.seconds(),
                    nanoseconds: entry.mtime.nanoseconds(),
                },
                ctime: raw::git_index_time {
                    seconds: entry.ctime.seconds(),
                    nanoseconds: entry.ctime.nanoseconds(),
                },
            };
            try_call!(raw::git_index_add(self.raw, &raw));
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
        let mut posix_path = OsString::new();
        for (i, comp) in path.components().enumerate() {
            if i != 0 { posix_path.push("/"); }
            posix_path.push(comp.as_os_str());
        }
        let posix_path = try!(posix_path.into_c_string());
        unsafe {
            try_call!(raw::git_index_add_bypath(self.raw, posix_path));
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
    pub fn add_all<T, I>(&mut self,
                         pathspecs: I,
                         flag: IndexAddOption,
                         mut cb: Option<&mut IndexMatchedPath>)
                         -> Result<(), Error>
        where T: IntoCString, I: IntoIterator<Item=T>,
    {
        let (_a, _b, raw_strarray) = try!(::util::iter2cstrs(pathspecs));
        let ptr = cb.as_mut();
        let callback = ptr.as_ref().map(|_| {
            index_matched_path_cb as raw::git_index_matched_path_cb
        });
        unsafe {
            try_call!(raw::git_index_add_all(self.raw,
                                             &raw_strarray,
                                             flag.bits() as c_uint,
                                             callback,
                                             ptr.map(|p| p as *mut _)
                                                .unwrap_or(ptr::null_mut())
                                                    as *mut c_void));
        }
        return Ok(());
    }

    /// Clear the contents (all the entries) of an index object.
    ///
    /// This clears the index object in memory; changes must be explicitly
    /// written to disk for them to take effect persistently via `write_*`.
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_index_clear(self.raw)); }
        Ok(())
    }

    /// Get the count of entries currently in the index
    pub fn len(&self) -> usize {
        unsafe { raw::git_index_entrycount(&*self.raw) as usize }
    }

    /// Get one of the entries in the index by its position.
    pub fn get(&self, n: usize) -> Option<IndexEntry> {
        unsafe {
            let ptr = raw::git_index_get_byindex(self.raw, n as size_t);
            if ptr.is_null() {None} else {Some(Binding::from_raw(*ptr))}
        }
    }

    /// Get an iterator over the entries in this index.
    pub fn iter(&self) -> IndexEntries {
        IndexEntries { range: 0..self.len(), index: self }
    }

    /// Get one of the entries in the index by its path.
    pub fn get_path(&self, path: &Path, stage: i32) -> Option<IndexEntry> {
        let path = path.into_c_string().unwrap();
        unsafe {
            let ptr = call!(raw::git_index_get_bypath(self.raw, path,
                                                      stage as c_int));
            if ptr.is_null() {None} else {Some(Binding::from_raw(*ptr))}
        }
    }

    /// Does this index have conflicts?
    ///
    /// Returns `true` if the index contains conflicts, `false` if it does not.
    pub fn has_conflicts(&self) -> bool {
        unsafe {
            raw::git_index_has_conflicts(self.raw) == 1
        }
    }

    /// Get the full path to the index file on disk.
    ///
    /// Returns `None` if this is an in-memory index.
    pub fn path(&self) -> Option<&Path> {
        unsafe {
            ::opt_bytes(self, raw::git_index_path(&*self.raw)).map(util::bytes2path)
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
    pub fn remove(&mut self, path: &Path, stage: i32) -> Result<(), Error> {
        let path = try!(path.into_c_string());
        unsafe {
            try_call!(raw::git_index_remove(self.raw, path, stage as c_int));
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
        let path = try!(path.into_c_string());
        unsafe {
            try_call!(raw::git_index_remove_bypath(self.raw, path));
        }
        Ok(())
    }

    /// Remove all entries from the index under a given directory.
    pub fn remove_dir(&mut self, path: &Path, stage: i32) -> Result<(), Error> {
        let path = try!(path.into_c_string());
        unsafe {
            try_call!(raw::git_index_remove_directory(self.raw, path,
                                                      stage as c_int));
        }
        Ok(())
    }

    /// Remove all matching index entries.
    ///
    /// If you provide a callback function, it will be invoked on each matching
    /// item in the index immediately before it is removed. Return 0 to remove
    /// the item, > 0 to skip the item, and < 0 to abort the scan.
    pub fn remove_all<T, I>(&mut self,
                            pathspecs: I,
                            mut cb: Option<&mut IndexMatchedPath>)
                            -> Result<(), Error>
        where T: IntoCString, I: IntoIterator<Item=T>,
    {
        let (_a, _b, raw_strarray) = try!(::util::iter2cstrs(pathspecs));
        let ptr = cb.as_mut();
        let callback = ptr.as_ref().map(|_| {
            index_matched_path_cb as raw::git_index_matched_path_cb
        });
        unsafe {
            try_call!(raw::git_index_remove_all(self.raw,
                                                &raw_strarray,
                                                callback,
                                                ptr.map(|p| p as *mut _)
                                                   .unwrap_or(ptr::null_mut())
                                                        as *mut c_void));
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
    pub fn update_all<T, I>(&mut self,
                            pathspecs: I,
                            mut cb: Option<&mut IndexMatchedPath>)
                            -> Result<(), Error>
        where T: IntoCString, I: IntoIterator<Item=T>,
    {
        let (_a, _b, raw_strarray) = try!(::util::iter2cstrs(pathspecs));
        let ptr = cb.as_mut();
        let callback = ptr.as_ref().map(|_| {
            index_matched_path_cb as raw::git_index_matched_path_cb
        });
        unsafe {
            try_call!(raw::git_index_update_all(self.raw,
                                                &raw_strarray,
                                                callback,
                                                ptr.map(|p| p as *mut _)
                                                   .unwrap_or(ptr::null_mut())
                                                        as *mut c_void));
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
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_index_write_tree(&mut raw, self.raw));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Write the index as a tree to the given repository
    ///
    /// This is the same as `write_tree` except that the destination repository
    /// can be chosen.
    pub fn write_tree_to(&mut self, repo: &Repository) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_index_write_tree_to(&mut raw, self.raw,
                                                   repo.raw()));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }
}

impl Binding for Index {
    type Raw = *mut raw::git_index;
    unsafe fn from_raw(raw: *mut raw::git_index) -> Index {
        Index { raw: raw }
    }
    fn raw(&self) -> *mut raw::git_index { self.raw }
}

extern fn index_matched_path_cb(path: *const c_char,
                                matched_pathspec: *const c_char,
                                payload: *mut c_void) -> c_int {
    unsafe {
        let path = CStr::from_ptr(path).to_bytes();
        let matched_pathspec = CStr::from_ptr(matched_pathspec).to_bytes();

        panic::wrap(|| {
            let payload = payload as *mut &mut IndexMatchedPath;
            (*payload)(util::bytes2path(path), matched_pathspec) as c_int
        }).unwrap_or(-1)
    }
}

impl Drop for Index {
    fn drop(&mut self) {
        unsafe { raw::git_index_free(self.raw) }
    }
}

impl<'index> Iterator for IndexEntries<'index> {
    type Item = IndexEntry;
    fn next(&mut self) -> Option<IndexEntry> {
        self.range.next().map(|i| self.index.get(i).unwrap())
    }
}

impl Binding for IndexEntry {
    type Raw = raw::git_index_entry;

    unsafe fn from_raw(raw: raw::git_index_entry) -> IndexEntry {
        let raw::git_index_entry {
            ctime, mtime, dev, ino, mode, uid, gid, file_size, id, flags,
            flags_extended, path
        } = raw;

        // libgit2 encodes the length of the path in the lower bits of `flags`,
        // but if the length exceeds the number of bits then the path is
        // nul-terminated.
        let mut pathlen = (flags & raw::GIT_IDXENTRY_NAMEMASK) as usize;
        if pathlen == raw::GIT_IDXENTRY_NAMEMASK as usize {
            pathlen = CStr::from_ptr(path).to_bytes().len();
        }

        let path = slice::from_raw_parts(path as *const u8, pathlen);

        IndexEntry {
            dev: dev,
            ino: ino,
            mode: mode,
            uid: uid,
            gid: gid,
            file_size: file_size,
            id: Binding::from_raw(&id as *const _),
            flags: flags,
            flags_extended: flags_extended,
            path: path.to_vec(),
            mtime: Binding::from_raw(mtime),
            ctime: Binding::from_raw(ctime),
        }
    }

    fn raw(&self) -> raw::git_index_entry {
        // not implemented, may require a CString in storage
        panic!()
    }
}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::Path;
    use tempdir::TempDir;

    use {Index, IndexEntry, Repository, ResetType, Oid, IndexTime};

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
        assert_eq!(index.path().map(|s| s.to_path_buf()),
                   Some(repo.path().join("index")));
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

        let root = repo.path().parent().unwrap();
        fs::create_dir(&root.join("foo")).unwrap();
        File::create(&root.join("foo/bar")).unwrap();
        let mut called = false;
        index.add_all(["foo"].iter(), ::ADD_DEFAULT,
                      Some(&mut |a: &Path, b: &[u8]| {
            assert!(!called);
            called = true;
            assert_eq!(b, b"foo");
            assert_eq!(a, Path::new("foo/bar"));
            0
        })).unwrap();
        assert!(called);

        called = false;
        index.remove_all(["."].iter(), Some(&mut |a: &Path, b: &[u8]| {
            assert!(!called);
            called = true;
            assert_eq!(b, b".");
            assert_eq!(a, Path::new("foo/bar"));
            0
        })).unwrap();
        assert!(called);
    }

    #[test]
    fn smoke_add() {
        let (_td, repo) = ::test::repo_init();
        let mut index = repo.index().unwrap();

        let root = repo.path().parent().unwrap();
        fs::create_dir(&root.join("foo")).unwrap();
        File::create(&root.join("foo/bar")).unwrap();
        index.add_path(Path::new("foo/bar")).unwrap();
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
        repo.reset(&obj, ResetType::Hard, None).unwrap();

        let td2 = TempDir::new("git").unwrap();
        let url = ::test::path2url(&root);
        let repo = Repository::clone(&url, td2.path()).unwrap();
        let obj = repo.find_object(commit, None).unwrap();
        repo.reset(&obj, ResetType::Hard, None).unwrap();
    }

    #[test]
    fn add_then_read() {
        let mut index = Index::new().unwrap();
        assert!(index.add(&entry()).is_err());

        let mut index = Index::new().unwrap();
        let mut e = entry();
        e.path = b"foobar".to_vec();
        index.add(&e).unwrap();
        let e = index.get(0).unwrap();
        assert_eq!(e.path.len(), 6);
    }

    fn entry() -> IndexEntry {
        IndexEntry {
            ctime: IndexTime::new(0, 0),
            mtime: IndexTime::new(0, 0),
            dev: 0,
            ino: 0,
            mode: 0o100644,
            uid: 0,
            gid: 0,
            file_size: 0,
            id: Oid::from_bytes(&[0; 20]).unwrap(),
            flags: 0,
            flags_extended: 0,
            path: Vec::new(),
        }
    }
}
