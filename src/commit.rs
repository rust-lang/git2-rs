use std::iter::FusedIterator;
use std::marker;
use std::mem;
use std::ops::Range;
use std::ptr;
use std::str;

use crate::util::Binding;
use crate::{raw, signature, Buf, Error, IntoCString, Mailmap, Object, Oid, Signature, Time, Tree};

/// A structure to represent a git [commit][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Commit<'repo> {
    raw: *mut raw::git_commit,
    _marker: marker::PhantomData<Object<'repo>>,
}

/// An iterator over the parent commits of a commit.
///
/// Aborts iteration when a commit cannot be found
pub struct Parents<'commit, 'repo> {
    range: Range<usize>,
    commit: &'commit Commit<'repo>,
}

/// An iterator over the parent commits' ids of a commit.
///
/// Aborts iteration when a commit cannot be found
pub struct ParentIds<'commit> {
    range: Range<usize>,
    commit: &'commit Commit<'commit>,
}

impl<'repo> Commit<'repo> {
    /// Get the id (SHA1) of a repository commit
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_commit_id(&*self.raw)) }
    }

    /// Get the id of the tree pointed to by this commit.
    ///
    /// No attempts are made to fetch an object from the ODB.
    pub fn tree_id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_commit_tree_id(&*self.raw)) }
    }

    /// Get the tree pointed to by a commit.
    pub fn tree(&self) -> Result<Tree<'repo>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_commit_tree(&mut ret, &*self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_commit {
        self.raw
    }

    /// Get the full message of a commit.
    ///
    /// The returned message will be slightly prettified by removing any
    /// potential leading newlines.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn message(&self) -> Option<&str> {
        str::from_utf8(self.message_bytes()).ok()
    }

    /// Get the full message of a commit as a byte slice.
    ///
    /// The returned message will be slightly prettified by removing any
    /// potential leading newlines.
    pub fn message_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_commit_message(&*self.raw)).unwrap() }
    }

    /// Get the encoding for the message of a commit, as a string representing a
    /// standard encoding name.
    ///
    /// `None` will be returned if the encoding is not known
    pub fn message_encoding(&self) -> Option<&str> {
        let bytes = unsafe { crate::opt_bytes(self, raw::git_commit_message_encoding(&*self.raw)) };
        bytes.and_then(|b| str::from_utf8(b).ok())
    }

    /// Get the full raw message of a commit.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn message_raw(&self) -> Option<&str> {
        str::from_utf8(self.message_raw_bytes()).ok()
    }

    /// Get the full raw message of a commit.
    pub fn message_raw_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_commit_message_raw(&*self.raw)).unwrap() }
    }

    /// Get the full raw text of the commit header.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn raw_header(&self) -> Option<&str> {
        str::from_utf8(self.raw_header_bytes()).ok()
    }

    /// Get an arbitrary header field.
    pub fn header_field_bytes<T: IntoCString>(&self, field: T) -> Result<Buf, Error> {
        let buf = Buf::new();
        let raw_field = field.into_c_string()?;
        unsafe {
            try_call!(raw::git_commit_header_field(
                buf.raw(),
                &*self.raw,
                raw_field
            ));
        }
        Ok(buf)
    }

    /// Get the full raw text of the commit header.
    pub fn raw_header_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_commit_raw_header(&*self.raw)).unwrap() }
    }

    /// Get the short "summary" of the git commit message.
    ///
    /// The returned message is the summary of the commit, comprising the first
    /// paragraph of the message with whitespace trimmed and squashed.
    ///
    /// `None` may be returned if an error occurs or if the summary is not valid
    /// utf-8.
    pub fn summary(&self) -> Option<&str> {
        self.summary_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the short "summary" of the git commit message.
    ///
    /// The returned message is the summary of the commit, comprising the first
    /// paragraph of the message with whitespace trimmed and squashed.
    ///
    /// `None` may be returned if an error occurs
    pub fn summary_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, raw::git_commit_summary(self.raw)) }
    }

    /// Get the long "body" of the git commit message.
    ///
    /// The returned message is the body of the commit, comprising everything
    /// but the first paragraph of the message. Leading and trailing whitespaces
    /// are trimmed.
    ///
    /// `None` may be returned if an error occurs or if the summary is not valid
    /// utf-8.
    pub fn body(&self) -> Option<&str> {
        self.body_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the long "body" of the git commit message.
    ///
    /// The returned message is the body of the commit, comprising everything
    /// but the first paragraph of the message. Leading and trailing whitespaces
    /// are trimmed.
    ///
    /// `None` may be returned if an error occurs.
    pub fn body_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, raw::git_commit_body(self.raw)) }
    }

    /// Get the commit time (i.e. committer time) of a commit.
    ///
    /// The first element of the tuple is the time, in seconds, since the epoch.
    /// The second element is the offset, in minutes, of the time zone of the
    /// committer's preferred time zone.
    pub fn time(&self) -> Time {
        unsafe {
            Time::new(
                raw::git_commit_time(&*self.raw) as i64,
                raw::git_commit_time_offset(&*self.raw) as i32,
            )
        }
    }

    /// Creates a new iterator over the parents of this commit.
    pub fn parents<'a>(&'a self) -> Parents<'a, 'repo> {
        Parents {
            range: 0..self.parent_count(),
            commit: self,
        }
    }

    /// Creates a new iterator over the parents of this commit.
    pub fn parent_ids(&self) -> ParentIds<'_> {
        ParentIds {
            range: 0..self.parent_count(),
            commit: self,
        }
    }

    /// Get the author of this commit.
    pub fn author(&self) -> Signature<'_> {
        unsafe {
            let ptr = raw::git_commit_author(&*self.raw);
            signature::from_raw_const(self, ptr)
        }
    }

    /// Get the author of this commit, using the mailmap to map names and email
    /// addresses to canonical real names and email addresses.
    pub fn author_with_mailmap(&self, mailmap: &Mailmap) -> Result<Signature<'static>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_commit_author_with_mailmap(
                &mut ret,
                &*self.raw,
                &*mailmap.raw()
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Get the committer of this commit.
    pub fn committer(&self) -> Signature<'_> {
        unsafe {
            let ptr = raw::git_commit_committer(&*self.raw);
            signature::from_raw_const(self, ptr)
        }
    }

    /// Get the committer of this commit, using the mailmap to map names and email
    /// addresses to canonical real names and email addresses.
    pub fn committer_with_mailmap(&self, mailmap: &Mailmap) -> Result<Signature<'static>, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_commit_committer_with_mailmap(
                &mut ret,
                &*self.raw,
                &*mailmap.raw()
            ));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Amend this existing commit with all non-`None` values
    ///
    /// This creates a new commit that is exactly the same as the old commit,
    /// except that any non-`None` values will be updated. The new commit has
    /// the same parents as the old commit.
    ///
    /// For information about `update_ref`, see [`Repository::commit`].
    ///
    /// [`Repository::commit`]: struct.Repository.html#method.commit
    pub fn amend(
        &self,
        update_ref: Option<&str>,
        author: Option<&Signature<'_>>,
        committer: Option<&Signature<'_>>,
        message_encoding: Option<&str>,
        message: Option<&str>,
        tree: Option<&Tree<'repo>>,
    ) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        let update_ref = crate::opt_cstr(update_ref)?;
        let encoding = crate::opt_cstr(message_encoding)?;
        let message = crate::opt_cstr(message)?;
        unsafe {
            try_call!(raw::git_commit_amend(
                &mut raw,
                self.raw(),
                update_ref,
                author.map(|s| s.raw()),
                committer.map(|s| s.raw()),
                encoding,
                message,
                tree.map(|t| t.raw())
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Get the number of parents of this commit.
    ///
    /// Use the `parents` iterator to return an iterator over all parents.
    pub fn parent_count(&self) -> usize {
        unsafe { raw::git_commit_parentcount(&*self.raw) as usize }
    }

    /// Get the specified parent of the commit.
    ///
    /// Use the `parents` iterator to return an iterator over all parents.
    pub fn parent(&self, i: usize) -> Result<Commit<'repo>, Error> {
        unsafe {
            let mut raw = ptr::null_mut();
            try_call!(raw::git_commit_parent(
                &mut raw,
                &*self.raw,
                i as libc::c_uint
            ));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Get the specified parent id of the commit.
    ///
    /// This is different from `parent`, which will attempt to load the
    /// parent commit from the ODB.
    ///
    /// Use the `parent_ids` iterator to return an iterator over all parents.
    pub fn parent_id(&self, i: usize) -> Result<Oid, Error> {
        unsafe {
            let id = raw::git_commit_parent_id(self.raw, i as libc::c_uint);
            if id.is_null() {
                Err(Error::from_str("parent index out of bounds"))
            } else {
                Ok(Binding::from_raw(id))
            }
        }
    }

    /// Casts this Commit to be usable as an `Object`
    pub fn as_object(&self) -> &Object<'repo> {
        unsafe { &*(self as *const _ as *const Object<'repo>) }
    }

    /// Consumes Commit to be returned as an `Object`
    pub fn into_object(self) -> Object<'repo> {
        assert_eq!(mem::size_of_val(&self), mem::size_of::<Object<'_>>());
        unsafe { mem::transmute(self) }
    }
}

impl<'repo> Binding for Commit<'repo> {
    type Raw = *mut raw::git_commit;
    unsafe fn from_raw(raw: *mut raw::git_commit) -> Commit<'repo> {
        Commit {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_commit {
        self.raw
    }
}

impl<'repo> std::fmt::Debug for Commit<'repo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut ds = f.debug_struct("Commit");
        ds.field("id", &self.id());
        if let Some(summary) = self.summary() {
            ds.field("summary", &summary);
        }
        ds.finish()
    }
}

/// Aborts iteration when a commit cannot be found
impl<'repo, 'commit> Iterator for Parents<'commit, 'repo> {
    type Item = Commit<'repo>;
    fn next(&mut self) -> Option<Commit<'repo>> {
        self.range.next().and_then(|i| self.commit.parent(i).ok())
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

/// Aborts iteration when a commit cannot be found
impl<'repo, 'commit> DoubleEndedIterator for Parents<'commit, 'repo> {
    fn next_back(&mut self) -> Option<Commit<'repo>> {
        self.range
            .next_back()
            .and_then(|i| self.commit.parent(i).ok())
    }
}

impl<'repo, 'commit> FusedIterator for Parents<'commit, 'repo> {}

impl<'repo, 'commit> ExactSizeIterator for Parents<'commit, 'repo> {}

/// Aborts iteration when a commit cannot be found
impl<'commit> Iterator for ParentIds<'commit> {
    type Item = Oid;
    fn next(&mut self) -> Option<Oid> {
        self.range
            .next()
            .and_then(|i| self.commit.parent_id(i).ok())
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}

/// Aborts iteration when a commit cannot be found
impl<'commit> DoubleEndedIterator for ParentIds<'commit> {
    fn next_back(&mut self) -> Option<Oid> {
        self.range
            .next_back()
            .and_then(|i| self.commit.parent_id(i).ok())
    }
}

impl<'commit> FusedIterator for ParentIds<'commit> {}

impl<'commit> ExactSizeIterator for ParentIds<'commit> {}

impl<'repo> Clone for Commit<'repo> {
    fn clone(&self) -> Self {
        self.as_object().clone().into_commit().ok().unwrap()
    }
}

impl<'repo> Drop for Commit<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_commit_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let commit = repo.find_commit(target).unwrap();
        assert_eq!(commit.message(), Some("initial\n\nbody"));
        assert_eq!(commit.body(), Some("body"));
        assert_eq!(commit.id(), target);
        commit.message_raw().unwrap();
        commit.raw_header().unwrap();
        commit.message_encoding();
        commit.summary().unwrap();
        commit.body().unwrap();
        commit.tree_id();
        commit.tree().unwrap();
        assert_eq!(commit.parents().count(), 0);

        let tree_header_bytes = commit.header_field_bytes("tree").unwrap();
        assert_eq!(
            crate::Oid::from_str(tree_header_bytes.as_str().unwrap()).unwrap(),
            commit.tree_id()
        );
        assert_eq!(commit.author().name(), Some("name"));
        assert_eq!(commit.author().email(), Some("email"));
        assert_eq!(commit.committer().name(), Some("name"));
        assert_eq!(commit.committer().email(), Some("email"));

        let sig = repo.signature().unwrap();
        let tree = repo.find_tree(commit.tree_id()).unwrap();
        let id = repo
            .commit(Some("HEAD"), &sig, &sig, "bar", &tree, &[&commit])
            .unwrap();
        let head = repo.find_commit(id).unwrap();

        let new_head = head
            .amend(Some("HEAD"), None, None, None, Some("new message"), None)
            .unwrap();
        let new_head = repo.find_commit(new_head).unwrap();
        assert_eq!(new_head.message(), Some("new message"));
        new_head.into_object();

        repo.find_object(target, None).unwrap().as_commit().unwrap();
        repo.find_object(target, None)
            .unwrap()
            .into_commit()
            .ok()
            .unwrap();
    }
}
