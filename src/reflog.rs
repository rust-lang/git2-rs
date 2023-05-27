use libc::size_t;
use std::iter::FusedIterator;
use std::marker;
use std::ops::Range;
use std::str;

use crate::util::Binding;
use crate::{raw, signature, Error, Oid, Signature};

/// A reference log of a git repository.
pub struct Reflog {
    raw: *mut raw::git_reflog,
}

/// An entry inside the reflog of a repository
pub struct ReflogEntry<'reflog> {
    raw: *const raw::git_reflog_entry,
    _marker: marker::PhantomData<&'reflog Reflog>,
}

/// An iterator over the entries inside of a reflog.
pub struct ReflogIter<'reflog> {
    range: Range<usize>,
    reflog: &'reflog Reflog,
}

impl Reflog {
    /// Add a new entry to the in-memory reflog.
    pub fn append(
        &mut self,
        new_oid: Oid,
        committer: &Signature<'_>,
        msg: Option<&str>,
    ) -> Result<(), Error> {
        let msg = crate::opt_cstr(msg)?;
        unsafe {
            try_call!(raw::git_reflog_append(
                self.raw,
                new_oid.raw(),
                committer.raw(),
                msg
            ));
        }
        Ok(())
    }

    /// Remove an entry from the reflog by its index
    ///
    /// To ensure there's no gap in the log history, set rewrite_previous_entry
    /// param value to `true`. When deleting entry n, member old_oid of entry
    /// n-1 (if any) will be updated with the value of member new_oid of entry
    /// n+1.
    pub fn remove(&mut self, i: usize, rewrite_previous_entry: bool) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_reflog_drop(
                self.raw,
                i as size_t,
                rewrite_previous_entry
            ));
        }
        Ok(())
    }

    /// Lookup an entry by its index
    ///
    /// Requesting the reflog entry with an index of 0 (zero) will return the
    /// most recently created entry.
    pub fn get(&self, i: usize) -> Option<ReflogEntry<'_>> {
        unsafe {
            let ptr = raw::git_reflog_entry_byindex(self.raw, i as size_t);
            Binding::from_raw_opt(ptr)
        }
    }

    /// Get the number of log entries in a reflog
    pub fn len(&self) -> usize {
        unsafe { raw::git_reflog_entrycount(self.raw) as usize }
    }

    /// Return `true ` is there is no log entry in a reflog
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Get an iterator to all entries inside of this reflog
    pub fn iter(&self) -> ReflogIter<'_> {
        ReflogIter {
            range: 0..self.len(),
            reflog: self,
        }
    }

    /// Write an existing in-memory reflog object back to disk using an atomic
    /// file lock.
    pub fn write(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_reflog_write(self.raw));
        }
        Ok(())
    }
}

impl Binding for Reflog {
    type Raw = *mut raw::git_reflog;

    unsafe fn from_raw(raw: *mut raw::git_reflog) -> Reflog {
        Reflog { raw }
    }
    fn raw(&self) -> *mut raw::git_reflog {
        self.raw
    }
}

impl Drop for Reflog {
    fn drop(&mut self) {
        unsafe { raw::git_reflog_free(self.raw) }
    }
}

impl<'reflog> ReflogEntry<'reflog> {
    /// Get the committer of this entry
    pub fn committer(&self) -> Signature<'_> {
        unsafe {
            let ptr = raw::git_reflog_entry_committer(self.raw);
            signature::from_raw_const(self, ptr)
        }
    }

    /// Get the new oid
    pub fn id_new(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_reflog_entry_id_new(self.raw)) }
    }

    /// Get the old oid
    pub fn id_old(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_reflog_entry_id_old(self.raw)) }
    }

    /// Get the log message, returning `None` on invalid UTF-8.
    pub fn message(&self) -> Option<&str> {
        self.message_bytes().and_then(|s| str::from_utf8(s).ok())
    }

    /// Get the log message as a byte array.
    pub fn message_bytes(&self) -> Option<&[u8]> {
        unsafe { crate::opt_bytes(self, raw::git_reflog_entry_message(self.raw)) }
    }
}

impl<'reflog> Binding for ReflogEntry<'reflog> {
    type Raw = *const raw::git_reflog_entry;

    unsafe fn from_raw(raw: *const raw::git_reflog_entry) -> ReflogEntry<'reflog> {
        ReflogEntry {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_reflog_entry {
        self.raw
    }
}

impl<'reflog> Iterator for ReflogIter<'reflog> {
    type Item = ReflogEntry<'reflog>;
    fn next(&mut self) -> Option<ReflogEntry<'reflog>> {
        self.range.next().and_then(|i| self.reflog.get(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}
impl<'reflog> DoubleEndedIterator for ReflogIter<'reflog> {
    fn next_back(&mut self) -> Option<ReflogEntry<'reflog>> {
        self.range.next_back().and_then(|i| self.reflog.get(i))
    }
}
impl<'reflog> FusedIterator for ReflogIter<'reflog> {}
impl<'reflog> ExactSizeIterator for ReflogIter<'reflog> {}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let mut reflog = repo.reflog("HEAD").unwrap();
        assert_eq!(reflog.iter().len(), 1);
        reflog.write().unwrap();

        let entry = reflog.iter().next().unwrap();
        assert!(entry.message().is_some());

        repo.reflog_rename("HEAD", "refs/heads/foo").unwrap();
        repo.reflog_delete("refs/heads/foo").unwrap();
    }
}
