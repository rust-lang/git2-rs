use std::kinds::marker;
use std::str;
use std::io;
use libc;

use {raw, Oid, Repository, Error, Object, ObjectType};

/// A structure to represent a git [tree][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Tree<'a> {
    raw: *mut raw::git_tree,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

/// A structure representing an entry inside of a tree. An entry is borrowed
/// from a tree.
pub struct TreeEntry<'a> {
    raw: *mut raw::git_tree_entry,
    owned: bool,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'a> Tree<'a> {
    /// Create a new object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_tree) -> Tree {
        Tree {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Get the id (SHA1) of a repository object
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_tree_id(&*self.raw)) }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_tree { self.raw }

    /// Get the number of entries listed in this tree.
    pub fn len(&self) -> uint {
        unsafe { raw::git_tree_entrycount(&*self.raw) as uint }
    }

    /// Lookup a tree entry by SHA value.
    pub fn get_id(&self, id: Oid) -> Option<TreeEntry> {
        unsafe {
            let ptr = raw::git_tree_entry_byid(&*self.raw(), &*id.raw());
            if ptr.is_null() {
                None
            } else {
                Some(TreeEntry::from_raw_const(self, ptr))
            }
        }
    }

    /// Lookup a tree entry by its position in the tree
    pub fn get(&self, n: uint) -> Option<TreeEntry> {
        unsafe {
            let ptr = raw::git_tree_entry_byindex(&*self.raw(),
                                                  n as libc::size_t);
            if ptr.is_null() {
                None
            } else {
                Some(TreeEntry::from_raw_const(self, ptr))
            }
        }
    }

    /// Lookup a tree entry by its filename
    pub fn get_name(&self, filename: &str) -> Option<TreeEntry> {
        unsafe {
            let ptr = call!(raw::git_tree_entry_byname(&*self.raw(),
                                                       filename.to_c_str()));
            if ptr.is_null() {
                None
            } else {
                Some(TreeEntry::from_raw_const(self, ptr))
            }
        }
    }

    /// Retrieve a tree entry contained in a tree or in any of its subtrees,
    /// given its relative path.
    pub fn get_path(&self, path: &Path) -> Result<TreeEntry<'static>, Error> {
        let mut ret = 0 as *mut raw::git_tree_entry;
        unsafe {
            try_call!(raw::git_tree_entry_bypath(&mut ret,
                                                 &*self.raw(),
                                                 path.to_c_str()));
            Ok(TreeEntry::from_raw(ret))
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Tree<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_tree_free(self.raw) }
    }
}

impl<'a> TreeEntry<'a> {
    /// Create a new tree entry from the raw pointer provided.
    ///
    /// The lifetime of the entry is tied to the tree provided and the function
    /// is unsafe because the validity of the pointer cannot be guaranteed.
    pub unsafe fn from_raw_const<'a>(_tree: &'a Tree,
                                     raw: *const raw::git_tree_entry)
                                     -> TreeEntry<'a> {
        TreeEntry {
            raw: raw as *mut raw::git_tree_entry,
            owned: false,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Create a new tree entry from the raw pointer provided.
    ///
    /// This will consume ownership of the pointer and free it when the entry is
    /// dropped.
    pub unsafe fn from_raw(raw: *mut raw::git_tree_entry) -> TreeEntry<'static> {
        TreeEntry {
            raw: raw,
            owned: true,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Gain access to the underlying raw pointer for this tree entry.
    pub fn raw(&self) -> *mut raw::git_tree_entry { self.raw }

    /// Get the id of the object pointed by the entry
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_tree_entry_id(&*self.raw)) }
    }

    /// Get the filename of a tree entry
    ///
    /// Returns `None` if the name is not valid utf-8
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes())
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
        let mut ret = 0 as *mut raw::git_object;
        unsafe {
            try_call!(raw::git_tree_entry_to_object(&mut ret, repo.raw(),
                                                    &*self.raw()));
            Ok(Object::from_raw(repo, ret))
        }
    }

    /// Get the type of the object pointed by the entry
    pub fn kind(&self) -> Option<ObjectType> {
        ObjectType::from_raw(unsafe { raw::git_tree_entry_type(&*self.raw) })
    }

    /// Get the UNIX file attributes of a tree entry
    pub fn filemode(&self) -> io::FilePermission {
        io::FilePermission::from_bits_truncate(unsafe {
            raw::git_tree_entry_filemode(&*self.raw) as u32
        })
    }

    /// Get the raw UNIX file attributes of a tree entry
    pub fn filemode_raw(&self) -> i32 {
        unsafe { raw::git_tree_entry_filemode_raw(&*self.raw) as i32 }
    }
}

impl<'a> Clone for TreeEntry<'a> {
    fn clone(&self) -> TreeEntry<'a> {
        let mut ret = 0 as *mut raw::git_tree_entry;
        unsafe {
            assert_eq!(raw::git_tree_entry_dup(&mut ret, &*self.raw()), 0);
            TreeEntry::from_raw(ret)
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
            0 => Equal,
            n if n < 0 => Less,
            _ => Greater,
        }
    }
}

impl<'a> PartialEq for TreeEntry<'a> {
    fn eq(&self, other: &TreeEntry<'a>) -> bool { self.cmp(other) == Equal }
}

impl<'a> Eq for TreeEntry<'a> {}

#[unsafe_destructor]
impl<'a> Drop for TreeEntry<'a> {
    fn drop(&mut self) {
        if self.owned {
            unsafe { raw::git_tree_entry_free(self.raw) }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::File;

    #[test]
    fn smoke() {
        let (td, repo) = ::test::repo_init();
        {
            let mut index = repo.index().unwrap();
            File::create(&td.path().join("foo")).write_str("foo").unwrap();
            index.add_path(&Path::new("foo")).unwrap();
            let id = index.write_tree().unwrap();
            let sig = repo.signature().unwrap();
            let tree = repo.find_tree(id).unwrap();
            let parent = repo.find_commit(repo.head().unwrap().target()
                                              .unwrap()).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "another commit",
                        &tree, [&parent]).unwrap();
        }
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();

        let tree = repo.find_tree(commit.tree_id()).unwrap();
        assert_eq!(tree.id(), commit.tree_id());
        assert_eq!(tree.len(), 1);
        let e1 = tree.get(0).unwrap();
        assert!(e1 == tree.get_id(e1.id()).unwrap());
        assert!(e1 == tree.get_name("foo").unwrap());
        assert!(e1 == tree.get_path(&Path::new("foo")).unwrap());
        assert_eq!(e1.name(), Some("foo"));
        e1.to_object(&repo).unwrap();
    }
}
