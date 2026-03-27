use std::marker;

use crate::util::Binding;
use crate::{raw, Error, Repository};

/// A structure to represent a git reference database.
pub struct Refdb<'repo> {
    raw: *mut raw::git_refdb,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl Drop for Refdb<'_> {
    fn drop(&mut self) {
        unsafe { raw::git_refdb_free(self.raw) }
    }
}

impl<'repo> Binding for Refdb<'repo> {
    type Raw = *mut raw::git_refdb;

    unsafe fn from_raw(raw: *mut raw::git_refdb) -> Refdb<'repo> {
        Refdb {
            raw,
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *mut raw::git_refdb {
        self.raw
    }
}

impl<'repo> Refdb<'repo> {
    /// Suggests that the reference database compress or optimize its
    /// references. This mechanism is implementation specific. For on-disk
    /// reference databases, for example, this may pack all loose references.
    pub fn compress(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_refdb_compress(self.raw));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let refdb = repo.refdb().unwrap();
        refdb.compress().unwrap();
    }

    #[test]
    fn compress_with_loose_refs() {
        let (_td, repo) = crate::test::repo_init();
        let head_id = repo.refname_to_id("HEAD").unwrap();
        for i in 0..10 {
            repo.reference(
                &format!("refs/tags/refdb-test-{}", i),
                head_id,
                false,
                "test",
            )
            .unwrap();
        }
        let refdb = repo.refdb().unwrap();
        refdb.compress().unwrap();
        assert_eq!(
            repo.references_glob("refs/tags/refdb-test-*")
                .unwrap()
                .count(),
            10
        );
    }
}
