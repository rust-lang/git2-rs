use std::ffi::CString;
use std::marker;

use crate::{raw, util::Binding, Error, Oid, Reflog, Repository, Signature};

/// A structure representing a transactional update of a repository's references.
///
/// Transactions work by locking loose refs for as long as the [`Transaction`]
/// is held, and committing all changes to disk when [`Transaction::commit`] is
/// called. Note that committing is not atomic: if an operation fails, the
/// transaction aborts, but previous successful operations are not rolled back.
pub struct Transaction<'repo> {
    raw: *mut raw::git_transaction,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        unsafe { raw::git_transaction_free(self.raw) }
    }
}

impl<'repo> Binding for Transaction<'repo> {
    type Raw = *mut raw::git_transaction;

    unsafe fn from_raw(ptr: *mut raw::git_transaction) -> Transaction<'repo> {
        Transaction {
            raw: ptr,
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *mut raw::git_transaction {
        self.raw
    }
}

impl<'repo> Transaction<'repo> {
    /// Lock the specified reference by name.
    pub fn lock_ref(&mut self, refname: &str) -> Result<(), Error> {
        let refname = CString::new(refname).unwrap();
        unsafe {
            try_call!(raw::git_transaction_lock_ref(self.raw, refname));
        }

        Ok(())
    }

    /// Set the target of the specified reference.
    ///
    /// The reference must have been locked via `lock_ref`.
    ///
    /// If `reflog_signature` is `None`, the [`Signature`] is read from the
    /// repository config.
    pub fn set_target(
        &mut self,
        refname: &str,
        target: Oid,
        reflog_signature: Option<&Signature<'_>>,
        reflog_message: &str,
    ) -> Result<(), Error> {
        let refname = CString::new(refname).unwrap();
        let reflog_message = CString::new(reflog_message).unwrap();
        unsafe {
            try_call!(raw::git_transaction_set_target(
                self.raw,
                refname,
                target.raw(),
                reflog_signature.map(|s| s.raw()),
                reflog_message
            ));
        }

        Ok(())
    }

    /// Set the target of the specified symbolic reference.
    ///
    /// The reference must have been locked via `lock_ref`.
    ///
    /// If `reflog_signature` is `None`, the [`Signature`] is read from the
    /// repository config.
    pub fn set_symbolic_target(
        &mut self,
        refname: &str,
        target: &str,
        reflog_signature: Option<&Signature<'_>>,
        reflog_message: &str,
    ) -> Result<(), Error> {
        let refname = CString::new(refname).unwrap();
        let target = CString::new(target).unwrap();
        let reflog_message = CString::new(reflog_message).unwrap();
        unsafe {
            try_call!(raw::git_transaction_set_symbolic_target(
                self.raw,
                refname,
                target,
                reflog_signature.map(|s| s.raw()),
                reflog_message
            ));
        }

        Ok(())
    }

    /// Add a [`Reflog`] to the transaction.
    ///
    /// This commit the in-memory [`Reflog`] to disk when the transaction commits.
    /// Note that atomicity is **not* guaranteed: if the transaction fails to
    /// modify `refname`, the reflog may still have been committed to disk.
    ///
    /// If this is combined with setting the target, that update won't be
    /// written to the log (i.e. the `reflog_signature` and `reflog_message`
    /// parameters will be ignored).
    pub fn set_reflog(&mut self, refname: &str, reflog: Reflog) -> Result<(), Error> {
        let refname = CString::new(refname).unwrap();
        unsafe {
            try_call!(raw::git_transaction_set_reflog(
                self.raw,
                refname,
                reflog.raw()
            ));
        }

        Ok(())
    }

    /// Remove a reference.
    ///
    /// The reference must have been locked via `lock_ref`.
    pub fn remove(&mut self, refname: &str) -> Result<(), Error> {
        let refname = CString::new(refname).unwrap();
        unsafe {
            try_call!(raw::git_transaction_remove(self.raw, refname));
        }

        Ok(())
    }

    /// Commit the changes from the transaction.
    ///
    /// The updates will be made one by one, and the first failure will stop the
    /// processing.
    pub fn commit(self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_transaction_commit(self.raw));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{Error, ErrorClass, ErrorCode, Oid, Repository};

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();

        let mut tx = t!(repo.transaction());

        t!(tx.lock_ref("refs/heads/main"));
        t!(tx.lock_ref("refs/heads/next"));

        t!(tx.set_target("refs/heads/main", Oid::zero(), None, "set main to zero"));
        t!(tx.set_symbolic_target(
            "refs/heads/next",
            "refs/heads/main",
            None,
            "set next to main",
        ));

        t!(tx.commit());

        assert_eq!(repo.refname_to_id("refs/heads/main").unwrap(), Oid::zero());
        assert_eq!(
            repo.find_reference("refs/heads/next")
                .unwrap()
                .symbolic_target()
                .unwrap(),
            "refs/heads/main"
        );
    }

    #[test]
    fn locks_same_repo_handle() {
        let (_td, repo) = crate::test::repo_init();

        let mut tx1 = t!(repo.transaction());
        t!(tx1.lock_ref("refs/heads/seen"));

        let mut tx2 = t!(repo.transaction());
        assert!(matches!(tx2.lock_ref("refs/heads/seen"), Err(e) if e.code() == ErrorCode::Locked))
    }

    #[test]
    fn locks_across_repo_handles() {
        let (td, repo1) = crate::test::repo_init();
        let repo2 = t!(Repository::open(&td));

        let mut tx1 = t!(repo1.transaction());
        t!(tx1.lock_ref("refs/heads/seen"));

        let mut tx2 = t!(repo2.transaction());
        assert!(matches!(tx2.lock_ref("refs/heads/seen"), Err(e) if e.code() == ErrorCode::Locked))
    }

    #[test]
    fn drop_unlocks() {
        let (_td, repo) = crate::test::repo_init();

        let mut tx = t!(repo.transaction());
        t!(tx.lock_ref("refs/heads/seen"));
        drop(tx);

        let mut tx2 = t!(repo.transaction());
        t!(tx2.lock_ref("refs/heads/seen"))
    }

    #[test]
    fn commit_unlocks() {
        let (_td, repo) = crate::test::repo_init();

        let mut tx = t!(repo.transaction());
        t!(tx.lock_ref("refs/heads/seen"));
        t!(tx.commit());

        let mut tx2 = t!(repo.transaction());
        t!(tx2.lock_ref("refs/heads/seen"));
    }

    #[test]
    fn prevents_non_transactional_updates() {
        let (_td, repo) = crate::test::repo_init();
        let head = t!(repo.refname_to_id("HEAD"));

        let mut tx = t!(repo.transaction());
        t!(tx.lock_ref("refs/heads/seen"));

        assert!(matches!(
            repo.reference("refs/heads/seen", head, true, "competing with lock"),
            Err(e) if e.code() == ErrorCode::Locked
        ));
    }

    #[test]
    fn remove() {
        let (_td, repo) = crate::test::repo_init();
        let head = t!(repo.refname_to_id("HEAD"));
        let next = "refs/heads/next";

        t!(repo.reference(
            next,
            head,
            true,
            "refs/heads/next@{0}: branch: Created from HEAD"
        ));

        {
            let mut tx = t!(repo.transaction());
            t!(tx.lock_ref(next));
            t!(tx.remove(next));
            t!(tx.commit());
        }
        assert!(matches!(repo.refname_to_id(next), Err(e) if e.code() == ErrorCode::NotFound))
    }

    #[test]
    fn must_lock_ref() {
        let (_td, repo) = crate::test::repo_init();

        // 🤷
        fn is_not_locked_err(e: &Error) -> bool {
            e.code() == ErrorCode::NotFound
                && e.class() == ErrorClass::Reference
                && e.message() == "the specified reference is not locked"
        }

        let mut tx = t!(repo.transaction());
        assert!(matches!(
            tx.set_target("refs/heads/main", Oid::zero(), None, "set main to zero"),
            Err(e) if is_not_locked_err(&e)
        ))
    }
}
