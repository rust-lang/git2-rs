use std::mem;
use std::cmp::Ordering;
use std::ffi::CString;
use std::ops::Range;
use std::marker;
use std::path::Path;
use std::ptr;
use std::str;
use libc;

use {raw, Oid, Repository, Error, Object, ObjectType};
use util::{Binding, IntoCString};

/// A structure to represent a git [tree][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Tree<'repo> {
    raw: *mut raw::git_tree,
    _marker: marker::PhantomData<Object<'repo>>,
}

/// A structure representing an entry inside of a tree. An entry is borrowed
/// from a tree.
pub struct TreeEntry<'tree> {
    raw: *mut raw::git_tree_entry,
    owned: bool,
    _marker: marker::PhantomData<&'tree raw::git_tree_entry>,
}

/// An iterator over the entries in a tree.
pub struct TreeIter<'tree> {
    range: Range<usize>,
    tree: &'tree Tree<'tree>,
}

impl<'repo> Tree<'repo> {
    /// Get the id (SHA1) of a repository object
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_tree_id(&*self.raw)) }
    }

    /// Get the number of entries listed in this tree.
    pub fn len(&self) -> usize {
        unsafe { raw::git_tree_entrycount(&*self.raw) as usize }
    }

    /// Return `true` if there is not entry
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Returns an iterator over the entries in this tree.
    pub fn iter(&self) -> TreeIter {
        TreeIter { range: 0..self.len(), tree: self }
    }

    /// Lookup a tree entry by SHA value.
    pub fn get_id(&self, id: Oid) -> Option<TreeEntry> {
        unsafe {
            let ptr = raw::git_tree_entry_byid(&*self.raw(), &*id.raw());
            if ptr.is_null() {
                None
            } else {
                Some(entry_from_raw_const(ptr))
            }
        }
    }

    /// Lookup a tree entry by its position in the tree
    pub fn get(&self, n: usize) -> Option<TreeEntry> {
        unsafe {
            let ptr = raw::git_tree_entry_byindex(&*self.raw(),
                                                  n as libc::size_t);
            if ptr.is_null() {
                None
            } else {
                Some(entry_from_raw_const(ptr))
            }
        }
    }

    /// Lookup a tree entry by its filename
    pub fn get_name(&self, filename: &str) -> Option<TreeEntry> {
        let filename = CString::new(filename).unwrap();
        unsafe {
            let ptr = call!(raw::git_tree_entry_byname(&*self.raw(), filename));
            if ptr.is_null() {
                None
            } else {
                Some(entry_from_raw_const(ptr))
            }
        }
    }

    /// Retrieve a tree entry contained in a tree or in any of its subtrees,
    /// given its relative path.
    pub fn get_path(&self, path: &Path) -> Result<TreeEntry<'static>, Error> {
        let path = try!(path.into_c_string());
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tree_entry_bypath(&mut ret, &*self.raw(), path));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Casts this Tree to be usable as an `Object`
    pub fn as_object(&self) -> &Object<'repo> {
        unsafe {
            &*(self as *const _ as *const Object<'repo>)
        }
    }

    /// Consumes Commit to be returned as an `Object`
    pub fn into_object(self) -> Object<'repo> {
        assert_eq!(mem::size_of_val(&self), mem::size_of::<Object>());
        unsafe {
            mem::transmute(self)
        }
    }
}

impl<'repo> Binding for Tree<'repo> {
    type Raw = *mut raw::git_tree;

    unsafe fn from_raw(raw: *mut raw::git_tree) -> Tree<'repo> {
        Tree { raw: raw, _marker: marker::PhantomData }
    }
    fn raw(&self) -> *mut raw::git_tree { self.raw }
}

impl<'repo> Drop for Tree<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_tree_free(self.raw) }
    }
}

impl<'repo, 'iter> IntoIterator for &'iter Tree<'repo> {
    type Item = TreeEntry<'iter>;
    type IntoIter = TreeIter<'iter>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// Create a new tree entry from the raw pointer provided.
///
/// The lifetime of the entry is tied to the tree provided and the function
/// is unsafe because the validity of the pointer cannot be guaranteed.
pub unsafe fn entry_from_raw_const<'tree>(raw: *const raw::git_tree_entry)
                                          -> TreeEntry<'tree> {
    TreeEntry {
        raw: raw as *mut raw::git_tree_entry,
        owned: false,
        _marker: marker::PhantomData,
    }
}

impl<'tree> TreeEntry<'tree> {
    /// Get the id of the object pointed by the entry
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_tree_entry_id(&*self.raw)) }
    }

    /// Get the filename of a tree entry
    ///
    /// Returns `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes()).ok()
    }

    /// Get the filename of a tree entry
    pub fn name_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_tree_entry_name(&*self.raw())).unwrap()
        }
    }

    /// Convert a tree entry to the object it points to.
    pub fn to_object<'a>(&self, repo: &'a Repository)
                         -> Result<Object<'a>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tree_entry_to_object(&mut ret, repo.raw(),
                                                    &*self.raw()));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get the type of the object pointed by the entry
    pub fn kind(&self) -> Option<ObjectType> {
        ObjectType::from_raw(unsafe { raw::git_tree_entry_type(&*self.raw) })
    }

    /// Get the UNIX file attributes of a tree entry
    pub fn filemode(&self) -> i32 {
        unsafe { raw::git_tree_entry_filemode(&*self.raw) as i32 }
    }

    /// Get the raw UNIX file attributes of a tree entry
    pub fn filemode_raw(&self) -> i32 {
        unsafe { raw::git_tree_entry_filemode_raw(&*self.raw) as i32 }
    }

    /// Convert this entry of any lifetime into an owned signature with a static
    /// lifetime.
    ///
    /// This will use the `Clone::clone` implementation under the hood.
    pub fn to_owned(&self) -> TreeEntry<'static> {
        unsafe {
            let me = mem::transmute::<&TreeEntry<'tree>, &TreeEntry<'static>>(self);
            me.clone()
        }
    }
}

impl<'a> Binding for TreeEntry<'a> {
    type Raw = *mut raw::git_tree_entry;
    unsafe fn from_raw(raw: *mut raw::git_tree_entry) -> TreeEntry<'a> {
        TreeEntry {
            raw: raw,
            owned: true,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_tree_entry { self.raw }
}

impl<'a> Clone for TreeEntry<'a> {
    fn clone(&self) -> TreeEntry<'a> {
        let mut ret = ptr::null_mut();
        unsafe {
            assert_eq!(raw::git_tree_entry_dup(&mut ret, &*self.raw()), 0);
            Binding::from_raw(ret)
        }
    }
}

impl<'a> PartialOrd for TreeEntry<'a> {
    fn partial_cmp(&self, other: &TreeEntry<'a>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a> Ord for TreeEntry<'a> {
    fn cmp(&self, other: &TreeEntry<'a>) -> Ordering {
        match unsafe { raw::git_tree_entry_cmp(&*self.raw(), &*other.raw()) } {
            0 => Ordering::Equal,
            n if n < 0 => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl<'a> PartialEq for TreeEntry<'a> {
    fn eq(&self, other: &TreeEntry<'a>) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}
impl<'a> Eq for TreeEntry<'a> {}

impl<'a> Drop for TreeEntry<'a> {
    fn drop(&mut self) {
        if self.owned {
            unsafe { raw::git_tree_entry_free(self.raw) }
        }
    }
}

impl<'tree> Iterator for TreeIter<'tree> {
    type Item = TreeEntry<'tree>;
    fn next(&mut self) -> Option<TreeEntry<'tree>> {
        self.range.next().and_then(|i| self.tree.get(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}
impl<'tree> DoubleEndedIterator for TreeIter<'tree> {
    fn next_back(&mut self) -> Option<TreeEntry<'tree>> {
        self.range.next_back().and_then(|i| self.tree.get(i))
    }
}
impl<'tree> ExactSizeIterator for TreeIter<'tree> {}

#[cfg(test)]
mod tests {
    use {Repository,Tree,TreeEntry,ObjectType,Object};
    use tempdir::TempDir;
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;

    pub struct TestTreeIter<'a> {
        entries: Vec<TreeEntry<'a>>,
        repo: &'a Repository,
    }

    impl<'a> Iterator for TestTreeIter<'a> {
        type Item = TreeEntry<'a>;

        fn next(&mut self) -> Option<TreeEntry<'a> > {
            if self.entries.is_empty() {
                None
            } else {
                let entry = self.entries.remove(0);

                match entry.kind() {
                    Some(ObjectType::Tree) => {
                        let obj: Object<'a> = entry.to_object(self.repo).unwrap();

                        let tree: &Tree<'a> = obj.as_tree().unwrap();

                        for entry in tree.iter() {
                            self.entries.push(entry.to_owned());
                        }
                    }
                    _ => {}
                }

                Some(entry)
            }
        }
    }

    fn tree_iter<'repo>(tree: &Tree<'repo>, repo: &'repo Repository)
                        -> TestTreeIter<'repo> {
        let mut initial = vec![];

        for entry in tree.iter() {
            initial.push(entry.to_owned());
        }

        TestTreeIter {
            entries: initial,
            repo: repo,
        }
    }

    #[test]
    fn smoke_tree_iter() {
        let (td, repo) = ::test::repo_init();

        setup_repo(&td, &repo);

        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let tree = repo.find_tree(commit.tree_id()).unwrap();
        assert_eq!(tree.id(), commit.tree_id());
        assert_eq!(tree.len(), 1);

        for entry in tree_iter(&tree, &repo) {
            println!("iter entry {:?}", entry.name());
        }
    }

    fn setup_repo(td: &TempDir, repo: &Repository) {
        let mut index = repo.index().unwrap();
        File::create(&td.path().join("foo")).unwrap().write_all(b"foo").unwrap();
        index.add_path(Path::new("foo")).unwrap();
        let id = index.write_tree().unwrap();
        let sig = repo.signature().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let parent = repo.find_commit(repo.head().unwrap().target()
                                      .unwrap()).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "another commit",
                    &tree, &[&parent]).unwrap();
    }

    #[test]
    fn smoke() {
        let (td, repo) = ::test::repo_init();

        setup_repo(&td, &repo);

        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let tree = repo.find_tree(commit.tree_id()).unwrap();
        assert_eq!(tree.id(), commit.tree_id());
        assert_eq!(tree.len(), 1);
        {
            let e1 = tree.get(0).unwrap();
            assert!(e1 == tree.get_id(e1.id()).unwrap());
            assert!(e1 == tree.get_name("foo").unwrap());
            assert!(e1 == tree.get_path(Path::new("foo")).unwrap());
            assert_eq!(e1.name(), Some("foo"));
            e1.to_object(&repo).unwrap();
        }
        tree.into_object();

        repo.find_object(commit.tree_id(), None).unwrap().as_tree().unwrap();
        repo.find_object(commit.tree_id(), None).unwrap().into_tree().ok().unwrap();
    }
}
