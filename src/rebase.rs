use std::ffi::CString;
use std::{marker, mem, ptr, str};

use crate::build::CheckoutBuilder;
use crate::util::Binding;
use crate::{raw, Error, Index, MergeOptions, Oid, Signature};

/// Rebase options
///
/// Use to tell the rebase machinery how to operate.
pub struct RebaseOptions<'cb> {
    raw: raw::git_rebase_options,
    rewrite_notes_ref: Option<CString>,
    merge_options: Option<MergeOptions>,
    checkout_options: Option<CheckoutBuilder<'cb>>,
}

impl<'cb> Default for RebaseOptions<'cb> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'cb> RebaseOptions<'cb> {
    /// Creates a new default set of rebase options.
    pub fn new() -> RebaseOptions<'cb> {
        let mut opts = RebaseOptions {
            raw: unsafe { mem::zeroed() },
            rewrite_notes_ref: None,
            merge_options: None,
            checkout_options: None,
        };
        assert_eq!(unsafe { raw::git_rebase_init_options(&mut opts.raw, 1) }, 0);
        opts
    }

    /// Used by `Repository::rebase`, this will instruct other clients working on this
    /// rebase that you want a quiet rebase experience, which they may choose to
    /// provide in an application-specific manner. This has no effect upon
    /// libgit2 directly, but is provided for interoperability between Git
    /// tools.
    pub fn quiet(&mut self, quiet: bool) -> &mut RebaseOptions<'cb> {
        self.raw.quiet = quiet as i32;
        self
    }

    /// Used by `Repository::rebase`, this will begin an in-memory rebase,
    /// which will allow callers to step through the rebase operations and
    /// commit the rebased changes, but will not rewind HEAD or update the
    /// repository to be in a rebasing state.  This will not interfere with
    /// the working directory (if there is one).
    pub fn inmemory(&mut self, inmemory: bool) -> &mut RebaseOptions<'cb> {
        self.raw.inmemory = inmemory as i32;
        self
    }

    /// Used by `finish()`, this is the name of the notes reference
    /// used to rewrite notes for rebased commits when finishing the rebase;
    /// if NULL, the contents of the configuration option `notes.rewriteRef`
    /// is examined, unless the configuration option `notes.rewrite.rebase`
    /// is set to false.  If `notes.rewriteRef` is also NULL, notes will
    /// not be rewritten.
    pub fn rewrite_notes_ref(&mut self, rewrite_notes_ref: &str) -> &mut RebaseOptions<'cb> {
        self.rewrite_notes_ref = Some(CString::new(rewrite_notes_ref).unwrap());
        self
    }

    /// Options to control how trees are merged during `next()`.
    pub fn merge_options(&mut self, opts: MergeOptions) -> &mut RebaseOptions<'cb> {
        self.merge_options = Some(opts);
        self
    }

    /// Options to control how files are written during `Repository::rebase`,
    /// `next()` and `abort()`. Note that a minimum strategy of
    /// `GIT_CHECKOUT_SAFE` is defaulted in `init` and `next`, and a minimum
    /// strategy of `GIT_CHECKOUT_FORCE` is defaulted in `abort` to match git
    /// semantics.
    pub fn checkout_options(&mut self, opts: CheckoutBuilder<'cb>) -> &mut RebaseOptions<'cb> {
        self.checkout_options = Some(opts);
        self
    }

    /// Acquire a pointer to the underlying raw options.
    pub fn raw(&mut self) -> *const raw::git_rebase_options {
        unsafe {
            if let Some(opts) = self.merge_options.as_mut().take() {
                ptr::copy_nonoverlapping(opts.raw(), &mut self.raw.merge_options, 1);
            }
            if let Some(opts) = self.checkout_options.as_mut() {
                opts.configure(&mut self.raw.checkout_options);
            }
            self.raw.rewrite_notes_ref = self
                .rewrite_notes_ref
                .as_ref()
                .map(|s| s.as_ptr())
                .unwrap_or(ptr::null());
        }
        &self.raw
    }
}

/// Representation of a rebase
pub struct Rebase<'repo> {
    raw: *mut raw::git_rebase,
    _marker: marker::PhantomData<&'repo raw::git_rebase>,
}

impl<'repo> Rebase<'repo> {
    /// Gets the count of rebase operations that are to be applied.
    pub fn len(&self) -> usize {
        unsafe { raw::git_rebase_operation_entrycount(self.raw) }
    }

    /// Gets the original `HEAD` ref name for merge rebases.
    pub fn orig_head_name(&self) -> Option<&str> {
        let name_bytes =
            unsafe { crate::opt_bytes(self, raw::git_rebase_orig_head_name(self.raw)) };
        name_bytes.and_then(|s| str::from_utf8(s).ok())
    }

    /// Gets the original HEAD id for merge rebases.
    pub fn orig_head_id(&self) -> Option<Oid> {
        unsafe { Oid::from_raw_opt(raw::git_rebase_orig_head_id(self.raw)) }
    }

    ///  Gets the rebase operation specified by the given index.
    pub fn nth(&mut self, n: usize) -> Option<RebaseOperation<'_>> {
        unsafe {
            let op = raw::git_rebase_operation_byindex(self.raw, n);
            if op.is_null() {
                None
            } else {
                Some(RebaseOperation::from_raw(op))
            }
        }
    }

    /// Gets the index of the rebase operation that is currently being applied.
    /// If the first operation has not yet been applied (because you have called
    /// `init` but not yet `next`) then this returns None.
    pub fn operation_current(&mut self) -> Option<usize> {
        let cur = unsafe { raw::git_rebase_operation_current(self.raw) };
        if cur == raw::GIT_REBASE_NO_OPERATION {
            None
        } else {
            Some(cur)
        }
    }

    /// Gets the index produced by the last operation, which is the result of
    /// `next()` and which will be committed by the next invocation of
    /// `commit()`. This is useful for resolving conflicts in an in-memory
    /// rebase before committing them.
    ///
    /// This is only applicable for in-memory rebases; for rebases within a
    /// working directory, the changes were applied to the repository's index.
    pub fn inmemory_index(&mut self) -> Result<Index, Error> {
        let mut idx = ptr::null_mut();
        unsafe {
            try_call!(raw::git_rebase_inmemory_index(&mut idx, self.raw));
            Ok(Binding::from_raw(idx))
        }
    }

    /// Commits the current patch.  You must have resolved any conflicts that
    /// were introduced during the patch application from the `git_rebase_next`
    /// invocation. To keep the author and message from the original commit leave
    /// them as None
    pub fn commit(
        &mut self,
        author: Option<&Signature<'_>>,
        committer: &Signature<'_>,
        message: Option<&str>,
    ) -> Result<Oid, Error> {
        let mut id: raw::git_oid = unsafe { mem::zeroed() };
        let message = crate::opt_cstr(message)?;
        unsafe {
            try_call!(raw::git_rebase_commit(
                &mut id,
                self.raw,
                author.map(|a| a.raw()),
                committer.raw(),
                ptr::null(),
                message
            ));
            Ok(Binding::from_raw(&id as *const _))
        }
    }

    /// Aborts a rebase that is currently in progress, resetting the repository
    /// and working directory to their state before rebase began.
    pub fn abort(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_rebase_abort(self.raw));
        }

        Ok(())
    }

    /// Finishes a rebase that is currently in progress once all patches have
    /// been applied.
    pub fn finish(&mut self, signature: Option<&Signature<'_>>) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_rebase_finish(self.raw, signature.map(|s| s.raw())));
        }

        Ok(())
    }
}

impl<'rebase> Iterator for Rebase<'rebase> {
    type Item = Result<RebaseOperation<'rebase>, Error>;

    /// Performs the next rebase operation and returns the information about it.
    /// If the operation is one that applies a patch (which is any operation except
    /// GitRebaseOperation::Exec) then the patch will be applied and the index and
    /// working directory will be updated with the changes.  If there are conflicts,
    /// you will need to address those before committing the changes.
    fn next(&mut self) -> Option<Result<RebaseOperation<'rebase>, Error>> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call_iter!(raw::git_rebase_next(&mut out, self.raw));
            Some(Ok(RebaseOperation::from_raw(out)))
        }
    }
}

impl<'repo> Binding for Rebase<'repo> {
    type Raw = *mut raw::git_rebase;
    unsafe fn from_raw(raw: *mut raw::git_rebase) -> Rebase<'repo> {
        Rebase {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_rebase {
        self.raw
    }
}

impl<'repo> Drop for Rebase<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_rebase_free(self.raw) }
    }
}

/// A rebase operation
///
/// Describes a single instruction/operation to be performed during the
/// rebase.
#[derive(Debug, PartialEq)]
pub enum RebaseOperationType {
    /// The given commit is to be cherry-picked. The client should commit the
    /// changes and continue if there are no conflicts.
    Pick,

    /// The given commit is to be cherry-picked, but the client should prompt
    /// the user to provide an updated commit message.
    Reword,

    /// The given commit is to be cherry-picked, but the client should stop to
    /// allow the user to edit the changes before committing them.
    Edit,

    /// The given commit is to be squashed into the previous commit. The commit
    /// message will be merged with the previous message.
    Squash,

    /// The given commit is to be squashed into the previous commit. The commit
    /// message from this commit will be discarded.
    Fixup,

    /// No commit will be cherry-picked. The client should run the given command
    /// and (if successful) continue.
    Exec,
}

impl RebaseOperationType {
    /// Convert from the int into an enum. Returns None if invalid.
    pub fn from_raw(raw: raw::git_rebase_operation_t) -> Option<RebaseOperationType> {
        match raw {
            raw::GIT_REBASE_OPERATION_PICK => Some(RebaseOperationType::Pick),
            raw::GIT_REBASE_OPERATION_REWORD => Some(RebaseOperationType::Reword),
            raw::GIT_REBASE_OPERATION_EDIT => Some(RebaseOperationType::Edit),
            raw::GIT_REBASE_OPERATION_SQUASH => Some(RebaseOperationType::Squash),
            raw::GIT_REBASE_OPERATION_FIXUP => Some(RebaseOperationType::Fixup),
            raw::GIT_REBASE_OPERATION_EXEC => Some(RebaseOperationType::Exec),
            _ => None,
        }
    }
}

/// A rebase operation
///
/// Describes a single instruction/operation to be performed during the
/// rebase.
#[derive(Debug)]
pub struct RebaseOperation<'rebase> {
    raw: *const raw::git_rebase_operation,
    _marker: marker::PhantomData<Rebase<'rebase>>,
}

impl<'rebase> RebaseOperation<'rebase> {
    /// The type of rebase operation
    pub fn kind(&self) -> Option<RebaseOperationType> {
        unsafe { RebaseOperationType::from_raw((*self.raw).kind) }
    }

    /// The commit ID being cherry-picked. This will be populated for all
    /// operations except those of type `GIT_REBASE_OPERATION_EXEC`.
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).id as *const _) }
    }

    ///The executable the user has requested be run.  This will only
    /// be populated for operations of type RebaseOperationType::Exec
    pub fn exec(&self) -> Option<&str> {
        unsafe { str::from_utf8(crate::opt_bytes(self, (*self.raw).exec).unwrap()).ok() }
    }
}

impl<'rebase> Binding for RebaseOperation<'rebase> {
    type Raw = *const raw::git_rebase_operation;
    unsafe fn from_raw(raw: *const raw::git_rebase_operation) -> RebaseOperation<'rebase> {
        RebaseOperation {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_rebase_operation {
        self.raw
    }
}

#[cfg(test)]
mod tests {
    use crate::{RebaseOperationType, RebaseOptions, Signature};
    use std::{fs, path};

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head_target = repo.head().unwrap().target().unwrap();
        let tip = repo.find_commit(head_target).unwrap();
        let sig = tip.author();
        let tree = tip.tree().unwrap();

        // We just want to see the iteration work so we can create commits with
        // no changes
        let c1 = repo
            .commit(Some("refs/heads/main"), &sig, &sig, "foo", &tree, &[&tip])
            .unwrap();
        let c1 = repo.find_commit(c1).unwrap();
        let c2 = repo
            .commit(Some("refs/heads/main"), &sig, &sig, "foo", &tree, &[&c1])
            .unwrap();

        let head = repo.find_reference("refs/heads/main").unwrap();
        let branch = repo.reference_to_annotated_commit(&head).unwrap();
        let upstream = repo.find_annotated_commit(tip.id()).unwrap();
        let mut rebase = repo
            .rebase(Some(&branch), Some(&upstream), None, None)
            .unwrap();

        assert_eq!(Some("refs/heads/main"), rebase.orig_head_name());
        assert_eq!(Some(c2), rebase.orig_head_id());

        assert_eq!(rebase.len(), 2);
        {
            let op = rebase.next().unwrap().unwrap();
            assert_eq!(op.kind(), Some(RebaseOperationType::Pick));
            assert_eq!(op.id(), c1.id());
        }
        {
            let op = rebase.next().unwrap().unwrap();
            assert_eq!(op.kind(), Some(RebaseOperationType::Pick));
            assert_eq!(op.id(), c2);
        }
        {
            let op = rebase.next();
            assert!(op.is_none());
        }
    }

    #[test]
    fn keeping_original_author_msg() {
        let (td, repo) = crate::test::repo_init();
        let head_target = repo.head().unwrap().target().unwrap();
        let tip = repo.find_commit(head_target).unwrap();
        let sig = Signature::now("testname", "testemail").unwrap();
        let mut index = repo.index().unwrap();

        fs::File::create(td.path().join("file_a")).unwrap();
        index.add_path(path::Path::new("file_a")).unwrap();
        index.write().unwrap();
        let tree_id_a = index.write_tree().unwrap();
        let tree_a = repo.find_tree(tree_id_a).unwrap();
        let c1 = repo
            .commit(Some("refs/heads/main"), &sig, &sig, "A", &tree_a, &[&tip])
            .unwrap();
        let c1 = repo.find_commit(c1).unwrap();

        fs::File::create(td.path().join("file_b")).unwrap();
        index.add_path(path::Path::new("file_b")).unwrap();
        index.write().unwrap();
        let tree_id_b = index.write_tree().unwrap();
        let tree_b = repo.find_tree(tree_id_b).unwrap();
        let c2 = repo
            .commit(Some("refs/heads/main"), &sig, &sig, "B", &tree_b, &[&c1])
            .unwrap();

        let branch = repo.find_annotated_commit(c2).unwrap();
        let upstream = repo.find_annotated_commit(tip.id()).unwrap();
        let mut opts: RebaseOptions<'_> = Default::default();
        let mut rebase = repo
            .rebase(Some(&branch), Some(&upstream), None, Some(&mut opts))
            .unwrap();

        assert_eq!(rebase.len(), 2);

        {
            rebase.next().unwrap().unwrap();
            let id = rebase.commit(None, &sig, None).unwrap();
            let commit = repo.find_commit(id).unwrap();
            assert_eq!(commit.message(), Some("A"));
            assert_eq!(commit.author().name(), Some("testname"));
            assert_eq!(commit.author().email(), Some("testemail"));
        }

        {
            rebase.next().unwrap().unwrap();
            let id = rebase.commit(None, &sig, None).unwrap();
            let commit = repo.find_commit(id).unwrap();
            assert_eq!(commit.message(), Some("B"));
            assert_eq!(commit.author().name(), Some("testname"));
            assert_eq!(commit.author().email(), Some("testemail"));
        }
        rebase.finish(None).unwrap();
    }
}
