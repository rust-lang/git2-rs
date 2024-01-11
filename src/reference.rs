use std::cmp::Ordering;
use std::ffi::CString;
use std::marker;
use std::mem;
use std::ptr;
use std::str;

use crate::object::CastOrPanic;
use crate::util::{c_cmp_to_ordering, Binding};
use crate::{
    call, raw, Blob, Commit, Error, Object, ObjectType, Oid, ReferenceFormat, ReferenceType,
    Repository, Tag, Tree,
};

// Not in the public header files (yet?), but a hard limit used by libgit2
// internally
const GIT_REFNAME_MAX: usize = 1024;

/// This is used to logically indicate that a [`raw::git_reference`] or
/// [`raw::git_reference_iterator`] holds a reference to [`raw::git_refdb`].
/// It is not necessary to have a wrapper like this in the
/// [`marker::PhantomData`], since all that matters is that it is tied to the
/// lifetime of the [`Repository`], but this helps distinguish the actual
/// references involved.
struct Refdb<'repo>(#[allow(dead_code)] &'repo Repository);

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
pub struct ReferenceNames<'repo, 'references> {
    inner: &'references mut References<'repo>,
}

impl<'repo> Reference<'repo> {
    /// Ensure the reference name is well-formed.
    ///
    /// Validation is performed as if [`ReferenceFormat::ALLOW_ONELEVEL`]
    /// was given to [`Reference::normalize_name`]. No normalization is
    /// performed, however.
    ///
    /// ```rust
    /// use git2::Reference;
    ///
    /// assert!(Reference::is_valid_name("HEAD"));
    /// assert!(Reference::is_valid_name("refs/heads/main"));
    ///
    /// // But:
    /// assert!(!Reference::is_valid_name("main"));
    /// assert!(!Reference::is_valid_name("refs/heads/*"));
    /// assert!(!Reference::is_valid_name("foo//bar"));
    /// ```
    ///
    /// [`ReferenceFormat::ALLOW_ONELEVEL`]:
    ///     struct.ReferenceFormat#associatedconstant.ALLOW_ONELEVEL
    /// [`Reference::normalize_name`]: struct.Reference#method.normalize_name
    pub fn is_valid_name(refname: &str) -> bool {
        crate::init();
        let refname = CString::new(refname).unwrap();
        let mut valid: libc::c_int = 0;
        unsafe {
            call::c_try(raw::git_reference_name_is_valid(
                &mut valid,
                refname.as_ptr(),
            ))
            .unwrap();
        }
        valid == 1
    }

    /// Normalize reference name and check validity.
    ///
    /// This will normalize the reference name by collapsing runs of adjacent
    /// slashes between name components into a single slash. It also validates
    /// the name according to the following rules:
    ///
    /// 1. If [`ReferenceFormat::ALLOW_ONELEVEL`] is given, the name may
    ///    contain only capital letters and underscores, and must begin and end
    ///    with a letter. (e.g. "HEAD", "ORIG_HEAD").
    /// 2. The flag [`ReferenceFormat::REFSPEC_SHORTHAND`] has an effect
    ///    only when combined with [`ReferenceFormat::ALLOW_ONELEVEL`]. If
    ///    it is given, "shorthand" branch names (i.e. those not prefixed by
    ///    `refs/`, but consisting of a single word without `/` separators)
    ///    become valid. For example, "main" would be accepted.
    /// 3. If [`ReferenceFormat::REFSPEC_PATTERN`] is given, the name may
    ///    contain a single `*` in place of a full pathname component (e.g.
    ///    `foo/*/bar`, `foo/bar*`).
    /// 4. Names prefixed with "refs/" can be almost anything. You must avoid
    ///    the characters '~', '^', ':', '\\', '?', '[', and '*', and the
    ///    sequences ".." and "@{" which have special meaning to revparse.
    ///
    /// If the reference passes validation, it is returned in normalized form,
    /// otherwise an [`Error`] with [`ErrorCode::InvalidSpec`] is returned.
    ///
    /// ```rust
    /// use git2::{Reference, ReferenceFormat};
    ///
    /// assert_eq!(
    ///     Reference::normalize_name(
    ///         "foo//bar",
    ///         ReferenceFormat::NORMAL
    ///     )
    ///     .unwrap(),
    ///     "foo/bar".to_owned()
    /// );
    ///
    /// assert_eq!(
    ///     Reference::normalize_name(
    ///         "HEAD",
    ///         ReferenceFormat::ALLOW_ONELEVEL
    ///     )
    ///     .unwrap(),
    ///     "HEAD".to_owned()
    /// );
    ///
    /// assert_eq!(
    ///     Reference::normalize_name(
    ///         "refs/heads/*",
    ///         ReferenceFormat::REFSPEC_PATTERN
    ///     )
    ///     .unwrap(),
    ///     "refs/heads/*".to_owned()
    /// );
    ///
    /// assert_eq!(
    ///     Reference::normalize_name(
    ///         "main",
    ///         ReferenceFormat::ALLOW_ONELEVEL | ReferenceFormat::REFSPEC_SHORTHAND
    ///     )
    ///     .unwrap(),
    ///     "main".to_owned()
    /// );
    /// ```
    ///
    /// [`ReferenceFormat::ALLOW_ONELEVEL`]:
    ///     struct.ReferenceFormat#associatedconstant.ALLOW_ONELEVEL
    /// [`ReferenceFormat::REFSPEC_SHORTHAND`]:
    ///     struct.ReferenceFormat#associatedconstant.REFSPEC_SHORTHAND
    /// [`ReferenceFormat::REFSPEC_PATTERN`]:
    ///     struct.ReferenceFormat#associatedconstant.REFSPEC_PATTERN
    /// [`Error`]: struct.Error
    /// [`ErrorCode::InvalidSpec`]: enum.ErrorCode#variant.InvalidSpec
    pub fn normalize_name(refname: &str, flags: ReferenceFormat) -> Result<String, Error> {
        crate::init();
        let mut dst = [0u8; GIT_REFNAME_MAX];
        let refname = CString::new(refname)?;
        unsafe {
            try_call!(raw::git_reference_normalize_name(
                dst.as_mut_ptr() as *mut libc::c_char,
                dst.len() as libc::size_t,
                refname,
                flags.bits()
            ));
            let s = &dst[..dst.iter().position(|&a| a == 0).unwrap()];
            Ok(str::from_utf8(s).unwrap().to_owned())
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_reference {
        self.raw
    }

    /// Delete an existing reference.
    ///
    /// This method works for both direct and symbolic references. The reference
    /// will be immediately removed on disk.
    ///
    /// This function will return an error if the reference has changed from the
    /// time it was looked up.
    pub fn delete(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_reference_delete(self.raw));
        }
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

    /// Get the reference type of a reference.
    ///
    /// If the type is unknown, then `None` is returned.
    pub fn kind(&self) -> Option<ReferenceType> {
        ReferenceType::from_raw(unsafe { raw::git_reference_type(&*self.raw) })
    }

    /// Get the full name of a reference.
    ///
    /// Returns `None` if the name is not valid utf-8.
    pub fn name(&self) -> Option<&str> {
        str::from_utf8(self.name_bytes()).ok()
    }

    /// Get the full name of a reference.
    pub fn name_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_reference_name(&*self.raw)).unwrap() }
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
        unsafe { crate::opt_bytes(self, raw::git_reference_shorthand(&*self.raw)).unwrap() }
    }

    /// Get the OID pointed to by a direct reference.
    ///
    /// Only available if the reference is direct (i.e. an object id reference,
    /// not a symbolic one).
    pub fn target(&self) -> Option<Oid> {
        unsafe { Binding::from_raw_opt(raw::git_reference_target(&*self.raw)) }
    }

    /// Return the peeled OID target of this reference.
    ///
    /// This peeled OID only applies to direct references that point to a hard
    /// Tag object: it is the result of peeling such Tag.
    pub fn target_peel(&self) -> Option<Oid> {
        unsafe { Binding::from_raw_opt(raw::git_reference_target_peel(&*self.raw)) }
    }

    /// Get full name to the reference pointed to by a symbolic reference.
    ///
    /// May return `None` if the reference is either not symbolic or not a
    /// valid utf-8 string.
    pub fn symbolic_target(&self) -> Option<&str> {
        self.symbolic_target_bytes()
            .and_then(|s| str::from_utf8(s).ok())
    }

    /// Get full name to the reference pointed to by a symbolic reference.
    ///
    /// Only available if the reference is symbolic.
    pub fn symbolic_target_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, raw::git_reference_symbolic_target(&*self.raw)) }
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
        Ok(self.peel(ObjectType::Blob)?.cast_or_panic(ObjectType::Blob))
    }

    /// Peel a reference to a commit
    ///
    /// This method recursively peels the reference until it reaches
    /// a commit.
    pub fn peel_to_commit(&self) -> Result<Commit<'repo>, Error> {
        Ok(self
            .peel(ObjectType::Commit)?
            .cast_or_panic(ObjectType::Commit))
    }

    /// Peel a reference to a tree
    ///
    /// This method recursively peels the reference until it reaches
    /// a tree.
    pub fn peel_to_tree(&self) -> Result<Tree<'repo>, Error> {
        Ok(self.peel(ObjectType::Tree)?.cast_or_panic(ObjectType::Tree))
    }

    /// Peel a reference to a tag
    ///
    /// This method recursively peels the reference until it reaches
    /// a tag.
    pub fn peel_to_tag(&self) -> Result<Tag<'repo>, Error> {
        Ok(self.peel(ObjectType::Tag)?.cast_or_panic(ObjectType::Tag))
    }

    /// Rename an existing reference.
    ///
    /// This method works for both direct and symbolic references.
    ///
    /// If the force flag is not enabled, and there's already a reference with
    /// the given name, the renaming will fail.
    pub fn rename(
        &mut self,
        new_name: &str,
        force: bool,
        msg: &str,
    ) -> Result<Reference<'repo>, Error> {
        let mut raw = ptr::null_mut();
        let new_name = CString::new(new_name)?;
        let msg = CString::new(msg)?;
        unsafe {
            try_call!(raw::git_reference_rename(
                &mut raw, self.raw, new_name, force, msg
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Conditionally create a new reference with the same name as the given
    /// reference but a different OID target. The reference must be a direct
    /// reference, otherwise this will fail.
    ///
    /// The new reference will be written to disk, overwriting the given
    /// reference.
    pub fn set_target(&mut self, id: Oid, reflog_msg: &str) -> Result<Reference<'repo>, Error> {
        let mut raw = ptr::null_mut();
        let msg = CString::new(reflog_msg)?;
        unsafe {
            try_call!(raw::git_reference_set_target(
                &mut raw,
                self.raw,
                id.raw(),
                msg
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Create a new reference with the same name as the given reference but a
    /// different symbolic target. The reference must be a symbolic reference,
    /// otherwise this will fail.
    ///
    /// The new reference will be written to disk, overwriting the given
    /// reference.
    ///
    /// The target name will be checked for validity. See
    /// [`Repository::reference_symbolic`] for rules about valid names.
    ///
    /// The message for the reflog will be ignored if the reference does not
    /// belong in the standard set (HEAD, branches and remote-tracking
    /// branches) and it does not have a reflog.
    pub fn symbolic_set_target(
        &mut self,
        target: &str,
        reflog_msg: &str,
    ) -> Result<Reference<'repo>, Error> {
        let mut raw = ptr::null_mut();
        let target = CString::new(target)?;
        let msg = CString::new(reflog_msg)?;
        unsafe {
            try_call!(raw::git_reference_symbolic_set_target(
                &mut raw, self.raw, target, msg
            ));
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
        c_cmp_to_ordering(unsafe { raw::git_reference_cmp(&*self.raw, &*other.raw) })
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
        Reference {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_reference {
        self.raw
    }
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
    unsafe fn from_raw(raw: *mut raw::git_reference_iterator) -> References<'repo> {
        References {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_reference_iterator {
        self.raw
    }
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
            try_call_iter!(raw::git_reference_next_name(&mut out, self.inner.raw));
            let bytes = crate::opt_bytes(self, out).unwrap();
            let s = str::from_utf8(bytes).unwrap();
            Some(Ok(mem::transmute::<&str, &'references str>(s)))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ObjectType, Reference, ReferenceType};

    #[test]
    fn is_valid_name() {
        assert!(Reference::is_valid_name("refs/foo"));
        assert!(!Reference::is_valid_name("foo"));
        assert!(Reference::is_valid_name("FOO_BAR"));

        assert!(!Reference::is_valid_name("foo"));
        assert!(!Reference::is_valid_name("_FOO_BAR"));
    }

    #[test]
    #[should_panic]
    fn is_valid_name_for_invalid_ref() {
        Reference::is_valid_name("ab\012");
    }

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let mut head = repo.head().unwrap();
        assert!(head.is_branch());
        assert!(!head.is_remote());
        assert!(!head.is_tag());
        assert!(!head.is_note());

        // HEAD is a symbolic reference but git_repository_head resolves it
        // so it is a GIT_REFERENCE_DIRECT.
        assert_eq!(head.kind().unwrap(), ReferenceType::Direct);

        assert!(head == repo.head().unwrap());
        assert_eq!(head.name(), Some("refs/heads/main"));

        assert!(head == repo.find_reference("refs/heads/main").unwrap());
        assert_eq!(
            repo.refname_to_id("refs/heads/main").unwrap(),
            head.target().unwrap()
        );

        assert!(head.symbolic_target().is_none());
        assert!(head.target_peel().is_none());

        assert_eq!(head.shorthand(), Some("main"));
        assert!(head.resolve().unwrap() == head);

        let mut tag1 = repo
            .reference("refs/tags/tag1", head.target().unwrap(), false, "test")
            .unwrap();
        assert!(tag1.is_tag());
        assert_eq!(tag1.kind().unwrap(), ReferenceType::Direct);

        let peeled_commit = tag1.peel(ObjectType::Commit).unwrap();
        assert_eq!(ObjectType::Commit, peeled_commit.kind().unwrap());
        assert_eq!(tag1.target().unwrap(), peeled_commit.id());

        tag1.delete().unwrap();

        let mut sym1 = repo
            .reference_symbolic("refs/tags/tag1", "refs/heads/main", false, "test")
            .unwrap();
        assert_eq!(sym1.kind().unwrap(), ReferenceType::Symbolic);
        let mut sym2 = repo
            .reference_symbolic("refs/tags/tag2", "refs/heads/main", false, "test")
            .unwrap()
            .symbolic_set_target("refs/tags/tag1", "test")
            .unwrap();
        assert_eq!(sym2.kind().unwrap(), ReferenceType::Symbolic);
        assert_eq!(sym2.symbolic_target().unwrap(), "refs/tags/tag1");
        sym2.delete().unwrap();
        sym1.delete().unwrap();

        {
            assert!(repo.references().unwrap().count() == 1);
            assert!(repo.references().unwrap().next().unwrap().unwrap() == head);
            let mut names = repo.references().unwrap();
            let mut names = names.names();
            assert_eq!(names.next().unwrap().unwrap(), "refs/heads/main");
            assert!(names.next().is_none());
            assert!(repo.references_glob("foo").unwrap().count() == 0);
            assert!(repo.references_glob("refs/heads/*").unwrap().count() == 1);
        }

        let mut head = head.rename("refs/foo", true, "test").unwrap();
        head.delete().unwrap();
    }
}
