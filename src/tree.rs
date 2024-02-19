use libc::{c_char, c_int, c_void};
use std::cmp::Ordering;
use std::ffi::{CStr, CString};
use std::iter::FusedIterator;
use std::marker;
use std::mem;
use std::ops::Range;
use std::path::Path;
use std::ptr;
use std::str;

use crate::util::{c_cmp_to_ordering, path_to_repo_path, Binding};
use crate::{panic, raw, Error, Object, ObjectType, Oid, Repository};

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

/// A binary indicator of whether a tree walk should be performed in pre-order
/// or post-order.
pub enum TreeWalkMode {
    /// Runs the traversal in pre-order.
    PreOrder = 0,
    /// Runs the traversal in post-order.
    PostOrder = 1,
}

/// Possible return codes for tree walking callback functions.
#[repr(i32)]
pub enum TreeWalkResult {
    /// Continue with the traversal as normal.
    Ok = 0,
    /// Skip the current node (in pre-order mode).
    Skip = 1,
    /// Completely stop the traversal.
    Abort = raw::GIT_EUSER,
}

impl Into<i32> for TreeWalkResult {
    fn into(self) -> i32 {
        self as i32
    }
}

impl Into<raw::git_treewalk_mode> for TreeWalkMode {
    #[cfg(target_env = "msvc")]
    fn into(self) -> raw::git_treewalk_mode {
        self as i32
    }
    #[cfg(not(target_env = "msvc"))]
    fn into(self) -> raw::git_treewalk_mode {
        self as u32
    }
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
    pub fn iter(&self) -> TreeIter<'_> {
        TreeIter {
            range: 0..self.len(),
            tree: self,
        }
    }

    /// Traverse the entries in a tree and its subtrees in post or pre-order.
    /// The callback function will be run on each node of the tree that's
    /// walked. The return code of this function will determine how the walk
    /// continues.
    ///
    /// libgit2 requires that the callback be an integer, where 0 indicates a
    /// successful visit, 1 skips the node, and -1 aborts the traversal completely.
    /// You may opt to use the enum [`TreeWalkResult`] instead.
    ///
    /// ```ignore
    /// let mut ct = 0;
    /// tree.walk(TreeWalkMode::PreOrder, |_, entry| {
    ///     assert_eq!(entry.name(), Some("foo"));
    ///     ct += 1;
    ///     TreeWalkResult::Ok
    /// }).unwrap();
    /// assert_eq!(ct, 1);
    /// ```
    ///
    /// See [libgit2 documentation][1] for more information.
    ///
    /// [1]: https://libgit2.org/libgit2/#HEAD/group/tree/git_tree_walk
    pub fn walk<C, T>(&self, mode: TreeWalkMode, mut callback: C) -> Result<(), Error>
    where
        C: FnMut(&str, &TreeEntry<'_>) -> T,
        T: Into<i32>,
    {
        unsafe {
            let mut data = TreeWalkCbData {
                callback: &mut callback,
            };
            raw::git_tree_walk(
                self.raw(),
                mode.into(),
                Some(treewalk_cb::<T>),
                &mut data as *mut _ as *mut c_void,
            );
            Ok(())
        }
    }

    /// Lookup a tree entry by SHA value.
    pub fn get_id(&self, id: Oid) -> Option<TreeEntry<'_>> {
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
    pub fn get(&self, n: usize) -> Option<TreeEntry<'_>> {
        unsafe {
            let ptr = raw::git_tree_entry_byindex(&*self.raw(), n as libc::size_t);
            if ptr.is_null() {
                None
            } else {
                Some(entry_from_raw_const(ptr))
            }
        }
    }

    /// Lookup a tree entry by its filename
    pub fn get_name(&self, filename: &str) -> Option<TreeEntry<'_>> {
        self.get_name_bytes(filename.as_bytes())
    }

    /// Lookup a tree entry by its filename, specified as bytes.
    ///
    /// This allows for non-UTF-8 filenames.
    pub fn get_name_bytes(&self, filename: &[u8]) -> Option<TreeEntry<'_>> {
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
        let path = path_to_repo_path(path)?;
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tree_entry_bypath(&mut ret, &*self.raw(), path));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Casts this Tree to be usable as an `Object`
    pub fn as_object(&self) -> &Object<'repo> {
        unsafe { &*(self as *const _ as *const Object<'repo>) }
    }

    /// Consumes this Tree to be returned as an `Object`
    pub fn into_object(self) -> Object<'repo> {
        assert_eq!(mem::size_of_val(&self), mem::size_of::<Object<'_>>());
        unsafe { mem::transmute(self) }
    }
}

type TreeWalkCb<'a, T> = dyn FnMut(&str, &TreeEntry<'_>) -> T + 'a;

struct TreeWalkCbData<'a, T> {
    callback: &'a mut TreeWalkCb<'a, T>,
}

extern "C" fn treewalk_cb<T: Into<i32>>(
    root: *const c_char,
    entry: *const raw::git_tree_entry,
    payload: *mut c_void,
) -> c_int {
    match panic::wrap(|| unsafe {
        let root = match CStr::from_ptr(root).to_str() {
            Ok(value) => value,
            _ => return -1,
        };
        let entry = entry_from_raw_const(entry);
        let payload = &mut *(payload as *mut TreeWalkCbData<'_, T>);
        let callback = &mut payload.callback;
        callback(root, &entry).into()
    }) {
        Some(value) => value,
        None => -1,
    }
}

impl<'repo> Binding for Tree<'repo> {
    type Raw = *mut raw::git_tree;

    unsafe fn from_raw(raw: *mut raw::git_tree) -> Tree<'repo> {
        Tree {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_tree {
        self.raw
    }
}

impl<'repo> std::fmt::Debug for Tree<'repo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Tree").field("id", &self.id()).finish()
    }
}

impl<'repo> Clone for Tree<'repo> {
    fn clone(&self) -> Self {
        self.as_object().clone().into_tree().ok().unwrap()
    }
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
pub unsafe fn entry_from_raw_const<'tree>(raw: *const raw::git_tree_entry) -> TreeEntry<'tree> {
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
        unsafe { crate::opt_bytes(self, raw::git_tree_entry_name(&*self.raw())).unwrap() }
    }

    /// Convert a tree entry to the object it points to.
    pub fn to_object<'a>(&self, repo: &'a Repository) -> Result<Object<'a>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_tree_entry_to_object(
                &mut ret,
                repo.raw(),
                &*self.raw()
            ));
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
            raw,
            owned: true,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_tree_entry {
        self.raw
    }
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
        c_cmp_to_ordering(unsafe { raw::git_tree_entry_cmp(&*self.raw(), &*other.raw()) })
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
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
    fn nth(&mut self, n: usize) -> Option<TreeEntry<'tree>> {
        self.range.nth(n).and_then(|i| self.tree.get(i))
    }
}
impl<'tree> DoubleEndedIterator for TreeIter<'tree> {
    fn next_back(&mut self) -> Option<TreeEntry<'tree>> {
        self.range.next_back().and_then(|i| self.tree.get(i))
    }
}
impl<'tree> FusedIterator for TreeIter<'tree> {}
impl<'tree> ExactSizeIterator for TreeIter<'tree> {}

#[cfg(test)]
mod tests {
    use super::{TreeWalkMode, TreeWalkResult};
    use crate::{Object, ObjectType, Repository, Tree, TreeEntry};
    use std::fs::File;
    use std::io::prelude::*;
    use std::path::Path;
    use tempfile::TempDir;

    pub struct TestTreeIter<'a> {
        entries: Vec<TreeEntry<'a>>,
        repo: &'a Repository,
    }

    impl<'a> Iterator for TestTreeIter<'a> {
        type Item = TreeEntry<'a>;

        fn next(&mut self) -> Option<TreeEntry<'a>> {
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

    fn tree_iter<'repo>(tree: &Tree<'repo>, repo: &'repo Repository) -> TestTreeIter<'repo> {
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
        let (td, repo) = crate::test::repo_init();

        setup_repo(&td, &repo);

        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let tree = repo.find_tree(commit.tree_id()).unwrap();
        assert_eq!(tree.id(), commit.tree_id());
        assert_eq!(tree.len(), 8);

        for entry in tree_iter(&tree, &repo) {
            println!("iter entry {:?}", entry.name());
        }
    }

    #[test]
    fn smoke_tree_nth() {
        let (td, repo) = crate::test::repo_init();

        setup_repo(&td, &repo);

        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let tree = repo.find_tree(commit.tree_id()).unwrap();
        assert_eq!(tree.id(), commit.tree_id());
        assert_eq!(tree.len(), 8);
        let mut it = tree.iter();
        let e = it.nth(4).unwrap();
        assert_eq!(e.name(), Some("f4"));
    }

    fn setup_repo(td: &TempDir, repo: &Repository) {
        let mut index = repo.index().unwrap();
        for n in 0..8 {
            let name = format!("f{n}");
            File::create(&td.path().join(&name))
                .unwrap()
                .write_all(name.as_bytes())
                .unwrap();
            index.add_path(Path::new(&name)).unwrap();
        }
        let id = index.write_tree().unwrap();
        let sig = repo.signature().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let parent = repo
            .find_commit(repo.head().unwrap().target().unwrap())
            .unwrap();
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "another commit",
            &tree,
            &[&parent],
        )
        .unwrap();
    }

    #[test]
    fn smoke() {
        let (td, repo) = crate::test::repo_init();

        setup_repo(&td, &repo);

        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let tree = repo.find_tree(commit.tree_id()).unwrap();
        assert_eq!(tree.id(), commit.tree_id());
        assert_eq!(tree.len(), 8);
        {
            let e0 = tree.get(0).unwrap();
            assert!(e0 == tree.get_id(e0.id()).unwrap());
            assert!(e0 == tree.get_name("f0").unwrap());
            assert!(e0 == tree.get_name_bytes(b"f0").unwrap());
            assert!(e0 == tree.get_path(Path::new("f0")).unwrap());
            assert_eq!(e0.name(), Some("f0"));
            e0.to_object(&repo).unwrap();

            let e1 = tree.get(1).unwrap();
            assert!(e1 == tree.get_id(e1.id()).unwrap());
            assert!(e1 == tree.get_name("f1").unwrap());
            assert!(e1 == tree.get_name_bytes(b"f1").unwrap());
            assert!(e1 == tree.get_path(Path::new("f1")).unwrap());
            assert_eq!(e1.name(), Some("f1"));
            e1.to_object(&repo).unwrap();
        }
        tree.into_object();

        repo.find_object(commit.tree_id(), None)
            .unwrap()
            .as_tree()
            .unwrap();
        repo.find_object(commit.tree_id(), None)
            .unwrap()
            .into_tree()
            .ok()
            .unwrap();
    }

    #[test]
    fn tree_walk() {
        let (td, repo) = crate::test::repo_init();

        setup_repo(&td, &repo);

        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();
        let tree = repo.find_tree(commit.tree_id()).unwrap();

        let mut ct = 0;
        tree.walk(TreeWalkMode::PreOrder, |_, entry| {
            assert_eq!(entry.name(), Some(format!("f{ct}").as_str()));
            ct += 1;
            0
        })
        .unwrap();
        assert_eq!(ct, 8);

        let mut ct = 0;
        tree.walk(TreeWalkMode::PreOrder, |_, entry| {
            assert_eq!(entry.name(), Some(format!("f{ct}").as_str()));
            ct += 1;
            TreeWalkResult::Ok
        })
        .unwrap();
        assert_eq!(ct, 8);
    }
}
