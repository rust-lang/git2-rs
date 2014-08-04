use std::kinds::marker;

use {raw, Oid, Repository, Error};

/// A structure to represent a git [object][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Tree<'a> {
    raw: *mut raw::git_tree,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoShare,
}

/// dox
pub struct TreeEntry;

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
            marker3: marker::NoShare,
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn lookup(repo: &Repository, oid: Oid) -> Result<Tree, Error> {
        let mut raw = 0 as *mut raw::git_tree;
        unsafe {
            try_call!(raw::git_tree_lookup(&mut raw, repo.raw(), oid.raw()));
            Ok(Tree::from_raw(repo, raw))
        }
    }

    /// Get the id (SHA1) of a repository object
    pub fn id(&self) -> Oid {
        unsafe {
            Oid::from_raw(raw::git_tree_id(&*self.raw))
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_tree { self.raw }
}

#[unsafe_destructor]
impl<'a> Drop for Tree<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_tree_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
    }
}
