use std::marker;
use std::ffi::CString;
use std::mem;
use std::ptr;

use libc::{c_char, size_t, c_uint, c_int};

use {raw, Error, Repository, Oid, Signature, Index, MergeOptions};
use build::CheckoutBuilder;
use util::Binding;

/// A structure representing a [rebase][1]
///
/// [1]: https://libgit2.github.com/libgit2/#HEAD/type/git_rebase
///
/// It has a lifetime of the repository it is attached to.
pub struct Rebase<'repo> {
    raw: *mut raw::git_rebase,
    _marker: marker::PhantomData<&'repo Repository>,
}

/// A structure representing a single [operation][2] to be performed during the rebase.
///
/// [2]: https://libgit2.github.com/libgit2/#HEAD/type/git_rebase_operation
///
/// It has a lifetime of the rebase it belongs to.
pub struct RebaseOperation<'rebase, 'repo: 'rebase> {
    raw: *mut raw::git_rebase_operation,
    _marker: marker::PhantomData<&'rebase Rebase<'repo>>,
}

/// Type of rebase operation
#[derive(PartialEq, Eq, Debug, Copy, Clone)]
pub enum RebaseOperationType {
    /// The given commit is to be cherry-picked.
    /// The client should commit the changes and continue if there are no conflicts.
    Pick,
    /// The given commit is to be cherry-picked, but the client should prompt
    /// the user to provide an updated commit message.
    Reword,
    /// The given commit is to be cherry-picked, but the client should stop
    /// to allow the user to edit the changes before committing them.
    Edit,
    /// The given commit is to be squashed into the previous commit.
    /// The commit message will be merged with the previous message.
    Squash,
    /// The given commit is to be squashed into the previous commit.
    /// The commit message from this commit will be discarded.
    FixUp,
    /// No commit will be cherry-picked.  The client should run the given
    /// command and (if successful) continue
    Exec,
}

/// A structure representing options for a `rebase`
pub struct RebaseOptions<'a> {
    /// Version of rebase options, defined by `GIT_REBASE_OPTIONS_VERSION`
    pub version: usize,

    /// this will instruct other clients working
    /// on this `rebase` that you want a quiet rebase experience.
    /// This is provided for interoperability between Git tools
    pub quiet: bool,

    /// Perform an in-memory rebase, will not updated the repository to be in a rebasing-state
    /// or modify the working directory.
    pub in_memory: bool,

    /// The name of the notes reference used to rewrite notes
    /// for rebased commits when finishing the rebase
    pub rewrite_notes_ref: Option<CString>,

    /// Options to control how trees are merged during a `rebase`.
    pub merge_options: Option<MergeOptions>,

    /// Options to control how files are written during a `rebase`.
    pub checkout_options: Option<CheckoutBuilder<'a>>,
}

impl<'repo> Rebase<'repo> {
    /// Returns the number of applied operations.
    pub fn operation_count(&self) -> usize {
        unsafe { raw::git_rebase_operation_entrycount(self.raw) }
    }

    /// Aborts a rebase that is currently in progress,
    /// resetting the repository and working directory to their state before rebase began.
    pub fn abort(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_rebase_abort(self.raw));
        }
        Ok(())
    }

    /// Commits the current patch.
    /// You must have resolved any conflicts
    /// that were introduced during the patch application from the `next` invocation.
    pub fn commit(&mut self,
                  author: &Signature,
                  committer: &Signature,
                  message: &str)
                  -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        let message = try!(CString::new(message));
        unsafe {
            try_call!(raw::git_rebase_commit(&mut raw,
                                             self.raw,
                                             author.raw(),
                                             committer.raw(),
                                             0 as *const c_char,
                                             message));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    /// Finishes a rebase that is currently in progress once all patches have been applied.
    pub fn finish(&mut self, signature: Option<&Signature>) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_rebase_finish(self.raw,
                                             signature
                                                 .map(|s| {
                                                          s.raw() as *const raw::git_signature
                                                      })
                                                 .unwrap_or(0 as *const raw::git_signature)));
        }
        Ok(())
    }
    /// Gets the index produced by the last operation.
    /// This is useful for resolving conflicts in an in-memory rebase before committing them.
    pub fn inmemory_index(&self) -> Result<Index, Error> {
        let mut ret = 0 as *mut raw::git_index;
        unsafe {
            try_call!(raw::git_rebase_inmemory_index(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Performs the next rebase operation and returns the `RebaseOperation` about it.
    pub fn next(&mut self) -> Result<RebaseOperation, Error> {
        let mut ret = 0 as *mut raw::git_rebase_operation;
        unsafe {
            try_call!(raw::git_rebase_next(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Gets the rebase operation specified by the given index.
    pub fn operation_at_index(&self, index: usize) -> Option<RebaseOperation> {
        unsafe {
            let ptr = raw::git_rebase_operation_byindex(self.raw, index as size_t);
            if ptr.is_null() {
                None
            } else {
                Some(Binding::from_raw(ptr))
            }
        }
    }

    /// Gets the index of the rebase operation that is currently being applied.
    /// If the first operation has not yet been applied it returns `None`.
    pub fn current_operation_index(&self) -> Option<usize> {
        unsafe {
            let index = raw::git_rebase_operation_current(self.raw);
            if index == raw::GIT_REBASE_NO_OPERATION {
                None
            } else {
                Some(index as usize)
            }
        }
    }

    /// Convenience function to get the operation at the current index.
    /// Returns none if the first operation has not yet been applied.
    pub fn current_operation(&self) -> Option<RebaseOperation> {
        if let Some(index) = self.current_operation_index() {
            self.operation_at_index(index)
        } else {
            None
        }
    }
}

impl<'repo> Binding for Rebase<'repo> {
    type Raw = *mut raw::git_rebase;
    unsafe fn from_raw(raw: *mut raw::git_rebase) -> Rebase<'repo> {
        Rebase {
            raw: raw,
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

impl<'rebase, 'repo: 'rebase> Binding for RebaseOperation<'rebase, 'repo> {
    type Raw = *mut raw::git_rebase_operation;
    unsafe fn from_raw(raw: *mut raw::git_rebase_operation) -> RebaseOperation<'rebase, 'repo> {
        RebaseOperation {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_rebase_operation {
        self.raw
    }
}

impl<'a> RebaseOptions<'a> {
    /// Initiate default set of rebase options.
    pub fn new() -> RebaseOptions<'a> {
        RebaseOptions {
            version: raw::GIT_REBASE_OPTIONS_VERSION as usize,
            quiet: false,
            in_memory: false,
            rewrite_notes_ref: None,
            merge_options: None,
            checkout_options: None,
        }
    }

    /// raw value of options
    pub unsafe fn raw(&mut self) -> Result<raw::git_rebase_options, Error> {
        let mut checkout_options: raw::git_checkout_options = mem::zeroed();
        try_call!(raw::git_checkout_init_options(&mut checkout_options,
                                                 raw::GIT_CHECKOUT_OPTIONS_VERSION));
        if let Some(ref mut opts) = self.checkout_options {
            opts.configure(&mut checkout_options);
        }
        let mut merge_options: raw::git_merge_options = mem::zeroed();
        if let Some(ref opts) = self.merge_options {
            ptr::copy(opts.raw(), &mut merge_options, 1);
        }

        Ok(raw::git_rebase_options {
               version: self.version as c_uint,
               quiet: self.quiet as c_int,
               inmemory: self.in_memory as c_int,
               rewrite_notes_ref: ::call::convert(&self.rewrite_notes_ref),
               merge_options: merge_options,
               checkout_options: checkout_options,
           })
    }
}

impl<'rebase, 'repo: 'rebase> RebaseOperation<'rebase, 'repo> {
    /// The type of `rebase` operation
    pub fn kind(&self) -> RebaseOperationType {
        unsafe { Binding::from_raw((*self.raw).kind) }
    }

    /// The commit ID being cherry-picked.  This will be populated for
    /// all operations except those of type `Exec`
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).id as *const raw::git_oid) }
    }
    /// The executable the user has requested be run.  This will only
    /// be populated for operations of type `Exec`
    pub fn exec(&self) -> Option<CString> {
        unsafe {
            if !(*self.raw).exec.is_null() {
                Some(CString::from_raw((*self.raw).exec as *mut c_char))
            } else {
                None
            }
        }
    }
}

impl Binding for RebaseOperationType {
    type Raw = raw::git_rebase_operation_t;
    unsafe fn from_raw(raw: raw::git_rebase_operation_t) -> RebaseOperationType {
        match raw {
            raw::GIT_REBASE_OPERATION_PICK => RebaseOperationType::Pick,
            raw::GIT_REBASE_OPERATION_REWORD => RebaseOperationType::Reword,
            raw::GIT_REBASE_OPERATION_EDIT => RebaseOperationType::Edit,
            raw::GIT_REBASE_OPERATION_SQUASH => RebaseOperationType::Squash,
            raw::GIT_REBASE_OPERATION_FIXUP => RebaseOperationType::FixUp,
            raw::GIT_REBASE_OPERATION_EXEC => RebaseOperationType::Exec,
            _ => panic!("Unknown rebase operation type: {}", raw),
        }
    }

    fn raw(&self) -> raw::git_rebase_operation_t {
        match *self {
            RebaseOperationType::Pick => raw::GIT_REBASE_OPERATION_PICK,
            RebaseOperationType::Reword => raw::GIT_REBASE_OPERATION_REWORD,
            RebaseOperationType::Edit => raw::GIT_REBASE_OPERATION_EDIT,
            RebaseOperationType::Squash => raw::GIT_REBASE_OPERATION_SQUASH,
            RebaseOperationType::FixUp => raw::GIT_REBASE_OPERATION_FIXUP,
            RebaseOperationType::Exec => raw::GIT_REBASE_OPERATION_EXEC,
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {}
}
