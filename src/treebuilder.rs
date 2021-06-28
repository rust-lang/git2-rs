use std::marker;
use std::ptr;

use libc::{c_int, c_void};

use crate::util::{Binding, IntoCString};
use crate::{panic, raw, tree, Error, Oid, Repository, TreeEntry};

/// Constructor for in-memory trees (low-level)
///
/// You probably want to use [`build::TreeUpdateBuilder`] instead.
///
/// This is the more raw of the two tree update facilities.  It
/// handles only one level of a nested tree structure at a time.  Each
/// path passed to `insert` etc. must be a single component.
///
/// [`build::TreeUpdateBuilder`]: crate::build::TreeUpdateBuilder
pub struct TreeBuilder<'repo> {
    raw: *mut raw::git_treebuilder,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> TreeBuilder<'repo> {
    /// Clear all the entries in the builder
    pub fn clear(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_treebuilder_clear(self.raw));
        }
        Ok(())
    }

    /// Get the number of entries
    pub fn len(&self) -> usize {
        unsafe { raw::git_treebuilder_entrycount(self.raw) as usize }
    }

    /// Return `true` if there is no entry
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get en entry from the builder from its filename
    pub fn get<P>(&self, filename: P) -> Result<Option<TreeEntry<'_>>, Error>
    where
        P: IntoCString,
    {
        let filename = filename.into_c_string()?;
        unsafe {
            let ret = raw::git_treebuilder_get(self.raw, filename.as_ptr());
            if ret.is_null() {
                Ok(None)
            } else {
                Ok(Some(tree::entry_from_raw_const(ret)))
            }
        }
    }

    /// Add or update an entry in the builder
    ///
    /// No attempt is made to ensure that the provided Oid points to
    /// an object of a reasonable type (or any object at all).
    ///
    /// The mode given must be one of 0o040000, 0o100644, 0o100755, 0o120000 or
    /// 0o160000 currently.
    pub fn insert<P: IntoCString>(
        &mut self,
        filename: P,
        oid: Oid,
        filemode: i32,
    ) -> Result<TreeEntry<'_>, Error> {
        let filename = filename.into_c_string()?;
        let filemode = filemode as raw::git_filemode_t;

        let mut ret = ptr::null();
        unsafe {
            try_call!(raw::git_treebuilder_insert(
                &mut ret,
                self.raw,
                filename,
                oid.raw(),
                filemode
            ));
            Ok(tree::entry_from_raw_const(ret))
        }
    }

    /// Remove an entry from the builder by its filename
    pub fn remove<P: IntoCString>(&mut self, filename: P) -> Result<(), Error> {
        let filename = filename.into_c_string()?;
        unsafe {
            try_call!(raw::git_treebuilder_remove(self.raw, filename));
        }
        Ok(())
    }

    /// Selectively remove entries from the tree
    ///
    /// Values for which the filter returns `true` will be kept.  Note
    /// that this behavior is different from the libgit2 C interface.
    pub fn filter<F>(&mut self, mut filter: F) -> Result<(), Error>
    where
        F: FnMut(&TreeEntry<'_>) -> bool,
    {
        let mut cb: &mut FilterCb<'_> = &mut filter;
        let ptr = &mut cb as *mut _;
        let cb: raw::git_treebuilder_filter_cb = Some(filter_cb);
        unsafe {
            try_call!(raw::git_treebuilder_filter(self.raw, cb, ptr as *mut _));
            panic::check();
        }
        Ok(())
    }

    /// Write the contents of the TreeBuilder as a Tree object and
    /// return its Oid
    pub fn write(&self) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_treebuilder_write(&mut raw, self.raw()));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }
}

type FilterCb<'a> = dyn FnMut(&TreeEntry<'_>) -> bool + 'a;

extern "C" fn filter_cb(entry: *const raw::git_tree_entry, payload: *mut c_void) -> c_int {
    let ret = panic::wrap(|| unsafe {
        // There's no way to return early from git_treebuilder_filter.
        if panic::panicked() {
            true
        } else {
            let entry = tree::entry_from_raw_const(entry);
            let payload = payload as *mut &mut FilterCb<'_>;
            (*payload)(&entry)
        }
    });
    if ret == Some(false) {
        1
    } else {
        0
    }
}

impl<'repo> Binding for TreeBuilder<'repo> {
    type Raw = *mut raw::git_treebuilder;

    unsafe fn from_raw(raw: *mut raw::git_treebuilder) -> TreeBuilder<'repo> {
        TreeBuilder {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_treebuilder {
        self.raw
    }
}

impl<'repo> Drop for TreeBuilder<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_treebuilder_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use crate::ObjectType;

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();

        let mut builder = repo.treebuilder(None).unwrap();
        assert_eq!(builder.len(), 0);
        let blob = repo.blob(b"data").unwrap();
        {
            let entry = builder.insert("a", blob, 0o100644).unwrap();
            assert_eq!(entry.kind(), Some(ObjectType::Blob));
        }
        builder.insert("b", blob, 0o100644).unwrap();
        assert_eq!(builder.len(), 2);
        builder.remove("a").unwrap();
        assert_eq!(builder.len(), 1);
        assert_eq!(builder.get("b").unwrap().unwrap().id(), blob);
        builder.clear().unwrap();
        assert_eq!(builder.len(), 0);
    }

    #[test]
    fn write() {
        let (_td, repo) = crate::test::repo_init();

        let mut builder = repo.treebuilder(None).unwrap();
        let data = repo.blob(b"data").unwrap();
        builder.insert("name", data, 0o100644).unwrap();
        let tree = builder.write().unwrap();
        let tree = repo.find_tree(tree).unwrap();
        let entry = tree.get(0).unwrap();
        assert_eq!(entry.name(), Some("name"));
        let blob = entry.to_object(&repo).unwrap();
        let blob = blob.as_blob().unwrap();
        assert_eq!(blob.content(), b"data");

        let builder = repo.treebuilder(Some(&tree)).unwrap();
        assert_eq!(builder.len(), 1);
    }

    #[test]
    fn filter() {
        let (_td, repo) = crate::test::repo_init();

        let mut builder = repo.treebuilder(None).unwrap();
        let blob = repo.blob(b"data").unwrap();
        let tree = {
            let head = repo.head().unwrap().peel(ObjectType::Commit).unwrap();
            let head = head.as_commit().unwrap();
            head.tree_id()
        };
        builder.insert("blob", blob, 0o100644).unwrap();
        builder.insert("dir", tree, 0o040000).unwrap();
        builder.insert("dir2", tree, 0o040000).unwrap();

        builder.filter(|_| true).unwrap();
        assert_eq!(builder.len(), 3);
        builder
            .filter(|e| e.kind().unwrap() != ObjectType::Blob)
            .unwrap();
        assert_eq!(builder.len(), 2);
        builder.filter(|_| false).unwrap();
        assert_eq!(builder.len(), 0);
    }
}
