use std::cmp::Ordering;
use std::ffi::CString;
use std::marker;
use std::mem;
use std::ptr;
use std::str;

use {raw, Error, Oid, Repository, Object, ObjectType, Blob, Commit, Tree, Tag};
use object::CastOrPanic;
use util::Binding;

struct Refdb<'repo>(&'repo Repository);

/// A structure to represent a git [reference][1].
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-References
pub struct Reference<'repo> {
    raw: *mut raw::git_reference,
    _marker: marker::PhantomData<Refdb<'repo>>,
}

/// An iterator over the references in a repository.
pub struct References<'repo> {
    raw: *mut raw::git_reference_iterator,
    _marker: marker::PhantomData<Refdb<'repo>>,
}

/// An iterator over the names of references in a repository.
pub struct ReferenceNames<'repo: 'references, 'references> {
    inner: &'references mut References<'repo>,
}

impl<'repo> Reference<'repo> {
    /// Ensure the reference name is well-formed.
    pub fn is_valid_name(refname: &str) -> bool {
        ::init();
        let refname = CString::new(refname).unwrap();
        unsafe { raw::git_reference_is_valid_name(refname.as_ptr()) == 1 }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_reference { self.raw }

    /// Delete an existing reference.
    ///
    /// This method works for both direct and symbolic references. The reference
    /// will be immediately removed on disk.
    ///
    /// This function will return an error if the reference has changed from the
    /// time it was looked up.
    pub fn delete(&mut self) -> Result<(), Error> {
        unsafe { try_call!(raw::git_reference_delete(self.raw)); }
        Ok(())
    }

    /// Check if a reference is a local branch.
    pub fn is_branch(&self) -> bool {
        unsafe { raw::git_reference_is_branch(&*self.raw) == 1 }
    }

    /// Check if a reference is a note.
    pub fn is_note(&self) -> bool {
        unsafe { raw::git_reference_is_note(&*self.raw) == 1 }
    }

    /// Check if a reference is a remote tracking branch
    pub fn is_remote(&self) -> bool {
        unsafe { raw::git_reference_is_remote(&*self.raw) == 1 }
    }

    /// Check if a reference is a tag
    pub fn is_tag(&self) -> bool {
        unsafe { raw::git_reference_is_tag(&*self.raw) == 1 }
    }

    /// Get the full name of a reference.
    ///
    /// Returns `None` if the name is not valid utf-8.
    pub fn name(&self) -> Option<&str> { str::from_utf8(self.name_bytes()).ok() }

    /// Get the full name of a reference.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_reference_name(&*self.raw)).unwrap() }
    }

    /// Get the full shorthand of a reference.
    ///
    /// This will transform the reference name into a name "human-readable"
    /// version. If no shortname is appropriate, it will return the full name.
    ///
    /// Returns `None` if the shorthand is not valid utf-8.
    pub fn shorthand(&self) -> Option<&str> {
        str::from_utf8(self.shorthand_bytes()).ok()
    }

    /// Get the full shorthand of a reference.
    pub fn shorthand_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_reference_shorthand(&*self.raw)).unwrap()
        }
    }

    /// Get the OID pointed to by a direct reference.
    ///
    /// Only available if the reference is direct (i.e. an object id reference,
    /// not a symbolic one).
    pub fn target(&self) -> Option<Oid> {
        unsafe {
            Binding::from_raw_opt(raw::git_reference_target(&*self.raw))
        }
    }

    /// Return the peeled OID target of this reference.
    ///
    /// This peeled OID only applies to direct references that point to a hard
    /// Tag object: it is the result of peeling such Tag.
    pub fn target_peel(&self) -> Option<Oid> {
        unsafe {
            Binding::from_raw_opt(raw::git_reference_target_peel(&*self.raw))
        }
    }

    /// Get full name to the reference pointed to by a symbolic reference.
    ///
    /// May return `None` if the reference is either not symbolic or not a
    /// valid utf-8 string.
    pub fn symbolic_target(&self) -> Option<&str> {
        self.symbolic_target_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get full name to the reference pointed to by a symbolic reference.
    ///
    /// Only available if the reference is symbolic.
    pub fn symbolic_target_bytes(&self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_reference_symbolic_target(&*self.raw)) }
    }

    /// Resolve a symbolic reference to a direct reference.
    ///
    /// This method iteratively peels a symbolic reference until it resolves to
    /// a direct reference to an OID.
    ///
    /// If a direct reference is passed as an argument, a copy of that
    /// reference is returned.
    pub fn resolve(&self) -> Result<Reference<'repo>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_resolve(&mut raw, &*self.raw));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Peel a reference to an object
    ///
    /// This method recursively peels the reference until it reaches
    /// an object of the specified type.
    pub fn peel(&self, kind: ObjectType) -> Result<Object<'repo>, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_reference_peel(&mut raw, self.raw, kind));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Peel a reference to a blob
    ///
    /// This method recursively peels the reference until it reaches
    /// a blob.
    pub fn peel_to_blob(&self) -> Result<Blob<'repo>, Error> {
        Ok(try!(self.peel(ObjectType::Blob)).cast_or_panic(ObjectType::Blob))
    }

    /// Peel a reference to a commit
    ///
    /// This method recursively peels the reference until it reaches
    /// a blob.
    pub fn peel_to_commit(&self) -> Result<Commit<'repo>, Error> {
        Ok(try!(self.peel(ObjectType::Commit)).cast_or_panic(ObjectType::Commit))
    }

    /// Peel a reference to a tree
    ///
    /// This method recursively peels the reference until it reaches
    /// a blob.
    pub fn peel_to_tree(&self) -> Result<Tree<'repo>, Error> {
        Ok(try!(self.peel(ObjectType::Tree)).cast_or_panic(ObjectType::Tree))
    }

    /// Peel a reference to a tag
    ///
    /// This method recursively peels the reference until it reaches
    /// a tag.
    pub fn peel_to_tag(&self) -> Result<Tag<'repo>, Error> {
        Ok(try!(self.peel(ObjectType::Tag)).cast_or_panic(ObjectType::Tag))
    }

    /// Rename an existing reference.
    ///
    /// This method works for both direct and symbolic references.
    ///
    /// If the force flag is not enabled, and there's already a reference with
    /// the given name, the renaming will fail.
    pub fn rename(&mut self, new_name: &str, force: bool,
                  msg: &str) -> Result<Reference<'repo>, Error> {
        let mut raw = ptr::null_mut();
        let new_name = try!(CString::new(new_name));
        let msg = try!(CString::new(msg));
        unsafe {
            try_call!(raw::git_reference_rename(&mut raw, self.raw, new_name,
                                                force, msg));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Conditionally create a new reference with the same name as the given
    /// reference but a different OID target. The reference must be a direct
    /// reference, otherwise this will fail.
    ///
    /// The new reference will be written to disk, overwriting the given
    /// reference.
    pub fn set_target(&mut self, id: Oid, reflog_msg: &str)
                      -> Result<Reference<'repo>, Error> {
        let mut raw = ptr::null_mut();
        let msg = try!(CString::new(reflog_msg));
        unsafe {
            try_call!(raw::git_reference_set_target(&mut raw, self.raw,
                                                    id.raw(), msg));
            Ok(Binding::from_raw(raw))
        }
    }

}

impl<'repo> PartialOrd for Reference<'repo> {
    fn partial_cmp(&self, other: &Reference<'repo>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'repo> Ord for Reference<'repo> {
    fn cmp(&self, other: &Reference<'repo>) -> Ordering {
        match unsafe { raw::git_reference_cmp(&*self.raw, &*other.raw) } {
            0 => Ordering::Equal,
            n if n < 0 => Ordering::Less,
            _ => Ordering::Greater,
        }
    }
}

impl<'repo> PartialEq for Reference<'repo> {
    fn eq(&self, other: &Reference<'repo>) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<'repo> Eq for Reference<'repo> {}

impl<'repo> Binding for Reference<'repo> {
    type Raw = *mut raw::git_reference;
    unsafe fn from_raw(raw: *mut raw::git_reference) -> Reference<'repo> {
        Reference { raw: raw, _marker: marker::PhantomData }
    }
    fn raw(&self) -> *mut raw::git_reference { self.raw }
}

impl<'repo> Drop for Reference<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_reference_free(self.raw) }
    }
}

impl<'repo> References<'repo> {
    /// Consumes a `References` iterator to create an iterator over just the
    /// name of some references.
    ///
    /// This is more efficient if only the names are desired of references as
    /// the references themselves don't have to be allocated and deallocated.
    ///
    /// The returned iterator will yield strings as opposed to a `Reference`.
    pub fn names<'a>(&'a mut self) -> ReferenceNames<'repo, 'a> {
        ReferenceNames { inner: self }
    }
}

impl<'repo> Binding for References<'repo> {
    type Raw = *mut raw::git_reference_iterator;
    unsafe fn from_raw(raw: *mut raw::git_reference_iterator)
                       -> References<'repo> {
        References { raw: raw, _marker: marker::PhantomData }
    }
    fn raw(&self) -> *mut raw::git_reference_iterator { self.raw }
}

impl<'repo> Iterator for References<'repo> {
    type Item = Result<Reference<'repo>, Error>;
    fn next(&mut self) -> Option<Result<Reference<'repo>, Error>> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call_iter!(raw::git_reference_next(&mut out, self.raw));
            Some(Ok(Binding::from_raw(out)))
        }
    }
}

impl<'repo> Drop for References<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_reference_iterator_free(self.raw) }
    }
}

impl<'repo, 'references> Iterator for ReferenceNames<'repo, 'references> {
    type Item = Result<&'references str, Error>;
    fn next(&mut self) -> Option<Result<&'references str, Error>> {
        let mut out = ptr::null();
        unsafe {
            try_call_iter!(raw::git_reference_next_name(&mut out,
                                                        self.inner.raw));
            let bytes = ::opt_bytes(self, out).unwrap();
            let s = str::from_utf8(bytes).unwrap();
            Some(Ok(mem::transmute::<&str, &'references str>(s)))
        }
    }
}

#[cfg(test)]
mod tests {
    use {Reference, ObjectType};

    #[test]
    fn smoke() {
        assert!(Reference::is_valid_name("refs/foo"));
        assert!(!Reference::is_valid_name("foo"));
    }

    #[test]
    fn smoke2() {
        let (_td, repo) = ::test::repo_init();
        let mut head = repo.head().unwrap();
        assert!(head.is_branch());
        assert!(!head.is_remote());
        assert!(!head.is_tag());
        assert!(!head.is_note());

        assert!(head == repo.head().unwrap());
        assert_eq!(head.name(), Some("refs/heads/master"));

        assert!(head == repo.find_reference("refs/heads/master").unwrap());
        assert_eq!(repo.refname_to_id("refs/heads/master").unwrap(),
                   head.target().unwrap());

        assert!(head.symbolic_target().is_none());
        assert!(head.target_peel().is_none());

        assert_eq!(head.shorthand(), Some("master"));
        assert!(head.resolve().unwrap() == head);

        let mut tag1 = repo.reference("refs/tags/tag1",
                                      head.target().unwrap(),
                                      false, "test").unwrap();
        assert!(tag1.is_tag());

        let peeled_commit = tag1.peel(ObjectType::Commit).unwrap();
        assert_eq!(ObjectType::Commit, peeled_commit.kind().unwrap());
        assert_eq!(tag1.target().unwrap(), peeled_commit.id());

        tag1.delete().unwrap();

        let mut sym1 = repo.reference_symbolic("refs/tags/tag1",
                                               "refs/heads/master", false,
                                               "test").unwrap();
        sym1.delete().unwrap();

        {
            assert!(repo.references().unwrap().count() == 1);
            assert!(repo.references().unwrap().next().unwrap().unwrap() == head);
            let mut names = repo.references().unwrap();
            let mut names = names.names();
            assert_eq!(names.next().unwrap().unwrap(), "refs/heads/master");
            assert!(names.next().is_none());
            assert!(repo.references_glob("foo").unwrap().count() == 0);
            assert!(repo.references_glob("refs/heads/*").unwrap().count() == 1);
        }

        let mut head = head.rename("refs/foo", true, "test").unwrap();
        head.delete().unwrap();

    }
}
