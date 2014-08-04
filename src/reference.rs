use std::kinds::marker;
use std::str;
use std::mem;
use libc;

use {raw, Repository, Error, Oid, Signature};

pub struct Reference<'a> {
    raw: *mut raw::git_reference,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoShare,
}

pub struct References<'a> {
    repo: &'a Repository,
    raw: *mut raw::git_reference_iterator,
}

pub struct ReferenceNames<'a> {
    inner: References<'a>,
}

impl<'a> Reference<'a> {
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_reference) -> Reference {
        Reference {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }

    /// Create a new direct reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn new<'a>(repo: &'a Repository, name: &str, id: Oid, force: bool,
                   sig: &Signature,
                   log_message: &str) -> Result<Reference<'a>, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        let name = name.to_c_str();
        let log_message = log_message.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_reference_create(&mut raw, repo.raw(), name.as_ptr(),
                                      &*id.raw(), force as libc::c_int,
                                      &*sig.raw(), log_message.as_ptr())
        }));
        Ok(unsafe { Reference::from_raw(repo, raw) })
    }

    /// Create a new symbolic reference.
    ///
    /// This function will return an error if a reference already exists with
    /// the given name unless force is true, in which case it will be
    /// overwritten.
    pub fn new_symbolic<'a>(repo: &'a Repository, name: &str, target: &str,
                            force: bool, sig: &Signature,
                            log_message: &str) -> Result<Reference<'a>, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        let name = name.to_c_str();
        let target = target.to_c_str();
        let log_message = log_message.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_reference_symbolic_create(&mut raw, repo.raw(), name.as_ptr(),
                                               target.as_ptr(),
                                               force as libc::c_int,
                                               &*sig.raw(), log_message.as_ptr())
        }));
        Ok(unsafe { Reference::from_raw(repo, raw) })
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn lookup<'a>(repo: &'a Repository, name: &str)
                      -> Result<Reference<'a>, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        let name = name.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_reference_lookup(&mut raw, repo.raw(), name.as_ptr())
        }));
        Ok(unsafe { Reference::from_raw(repo, raw) })
    }

    pub fn name_to_id(repo: &Repository, name: &str) -> Result<Oid, Error> {
        let mut ret: raw::git_oid = unsafe { mem::zeroed() };
        let name = name.to_c_str();
        try!(::doit(|| unsafe {
            raw::git_reference_name_to_id(&mut ret, repo.raw(), name.as_ptr())
        }));
        Ok(unsafe { Oid::from_raw(&ret) })
    }

    /// Ensure the reference name is well-formed.
    pub fn is_valid_name(refname: &str) -> bool {
        ::init();
        let refname = refname.to_c_str();
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
        try!(::doit(|| unsafe {
            raw::git_reference_delete(self.raw)
        }));
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
    pub fn name(&self) -> Option<&str> { str::from_utf8(self.name_bytes()) }

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
        str::from_utf8(self.shorthand_bytes())
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
        let ptr = unsafe { raw::git_reference_target(&*self.raw) };
        if ptr.is_null() {None} else {Some(unsafe { Oid::from_raw(ptr) })}
    }

    /// Return the peeled OID target of this reference.
    ///
    /// This peeled OID only applies to direct references that point to a hard
    /// Tag object: it is the result of peeling such Tag.
    pub fn target_peel(&self) -> Option<Oid> {
        let ptr = unsafe { raw::git_reference_target_peel(&*self.raw) };
        if ptr.is_null() {None} else {Some(unsafe { Oid::from_raw(ptr) })}
    }

    /// Get full name to the reference pointed to by a symbolic reference.
    ///
    /// May return `None` if the reference is either not symbolic or not a
    /// valid utf-8 string.
    pub fn symbolic_target(&self) -> Option<&str> {
        self.symbolic_target_bytes().and_then(str::from_utf8)
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
    pub fn resolve(&self) -> Result<Reference<'a>, Error> {
        let mut raw = 0 as *mut raw::git_reference;
        try!(::doit(|| unsafe {
            raw::git_reference_resolve(&mut raw, &*self.raw)
        }));
        Ok(Reference {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        })
    }

    /// Rename an existing reference.
    ///
    /// This method works for both direct and symbolic references.
    ///
    /// If the force flag is not enabled, and there's already a reference with
    /// the given name, the renaming will fail.
    pub fn rename(&mut self, new_name: &str, force: bool, sig: &Signature,
                  msg: &str) -> Result<Reference<'a>, Error> {
        let new_name = new_name.to_c_str();
        let msg = msg.to_c_str();
        let mut raw = 0 as *mut raw::git_reference;
        try!(::doit(|| unsafe {
            raw::git_reference_rename(&mut raw, self.raw, new_name.as_ptr(),
                                      force as libc::c_int,
                                      &*sig.raw(), msg.as_ptr())
        }));
        Ok(Reference {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        })
    }

}

impl<'a> PartialOrd for Reference<'a> {
    fn partial_cmp(&self, other: &Reference<'a>) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for Reference<'a> {
    fn cmp(&self, other: &Reference<'a>) -> Ordering {
        match unsafe { raw::git_reference_cmp(&*self.raw, &*other.raw) } {
            0 => Equal,
            n if n < 0 => Less,
            _ => Greater,
        }
    }
}

impl<'a> PartialEq for Reference<'a> {
    fn eq(&self, other: &Reference<'a>) -> bool { self.cmp(other) == Equal }
}

impl<'a> Eq for Reference<'a> {}

#[unsafe_destructor]
impl<'a> Drop for Reference<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_reference_free(self.raw) }
    }
}

impl<'a> References<'a> {
    pub unsafe fn from_raw(repo: &Repository,
                           raw: *mut raw::git_reference_iterator)
                           -> References {
        References {
            raw: raw,
            repo: repo,
        }
    }
}

impl<'a> Iterator<Reference<'a>> for References<'a> {
    fn next(&mut self) -> Option<Reference<'a>> {
        let mut out = 0 as *mut raw::git_reference;
        if unsafe { raw::git_reference_next(&mut out, self.raw) == 0 } {
            Some(unsafe { Reference::from_raw(self.repo, out) })
        } else {
            None
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for References<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_reference_iterator_free(self.raw) }
    }
}

impl<'a> ReferenceNames<'a> {
    pub fn new(refs: References) -> ReferenceNames {
        ReferenceNames { inner: refs }
    }
}

impl<'a> Iterator<&'a str> for ReferenceNames<'a> {
    fn next(&mut self) -> Option<&'a str> {
        let mut out = 0 as *const libc::c_char;
        if unsafe { raw::git_reference_next_name(&mut out, self.inner.raw) == 0 } {
            Some(unsafe {
                let bytes = ::opt_bytes(self.inner.repo, out).unwrap();
                str::from_utf8(bytes).unwrap()
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{TempDir, File};
    use {Reference, Repository, Signature};

    #[test]
    fn smoke() {
        assert!(Reference::is_valid_name("refs/foo"));
        assert!(!Reference::is_valid_name("foo"));
    }

    #[test]
    fn smoke2() {
        let td = TempDir::new("test").unwrap();
        git!(td.path(), "init");
        assert!(Repository::init(td.path(), false).unwrap().head().is_err());

        git!(td.path(), "config", "user.name", "foo");
        git!(td.path(), "config", "user.email", "foo");
        File::create(&td.path().join("foo")).write_str("foobar").unwrap();
        git!(td.path(), "add", ".");
        git!(td.path(), "commit", "-m", "foo");

        let repo = Repository::init(td.path(), false).unwrap();
        let mut head = repo.head().unwrap();
        assert!(head.is_branch());
        assert!(!head.is_remote());
        assert!(!head.is_tag());
        assert!(!head.is_note());

        assert!(head == repo.head().unwrap());
        assert_eq!(head.name(), Some("refs/heads/master"));

        assert!(head == Reference::lookup(&repo, "refs/heads/master").unwrap());
        assert_eq!(Reference::name_to_id(&repo, "refs/heads/master").unwrap(),
                   head.target().unwrap());

        assert!(head.symbolic_target().is_none());
        assert!(head.target_peel().is_none());

        assert_eq!(head.shorthand(), Some("master"));
        assert!(head.resolve().unwrap() == head);

        let sig = Signature::default(&repo).unwrap();
        let mut tag1 = Reference::new(&repo, "refs/tags/tag1",
                                      head.target().unwrap(),
                                      false,
                                      &sig, "test").unwrap();
        assert!(tag1.is_tag());
        tag1.delete().unwrap();

        let mut sym1 = Reference::new_symbolic(&repo, "refs/tags/tag1",
                                               "refs/heads/master", false,
                                               &sig, "test").unwrap();
        sym1.delete().unwrap();

        {
            assert!(repo.references().unwrap().count() == 1);
            assert!(repo.references().unwrap().next().unwrap() == head);
            let mut names = ::ReferenceNames::new(repo.references().unwrap());
            assert_eq!(names.next(), Some("refs/heads/master"));
            assert_eq!(names.next(), None);
            assert!(repo.references_glob("foo").unwrap().count() == 0);
            assert!(repo.references_glob("refs/heads/*").unwrap().count() == 1);
        }

        let mut head = head.rename("refs/foo", true, &sig, "test").unwrap();
        head.delete().unwrap();

    }
}
