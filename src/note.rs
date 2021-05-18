use std::marker;
use std::str;

use crate::util::Binding;
use crate::{raw, signature, Error, Oid, Repository, Signature};

/// A structure representing a [note][note] in git.
///
/// [note]: http://alblue.bandlem.com/2011/11/git-tip-of-week-git-notes.html
pub struct Note<'repo> {
    raw: *mut raw::git_note,

    // Hmm, the current libgit2 version does not have this inside of it, but
    // perhaps it's a good idea to keep it around? Can always remove it later I
    // suppose...
    _marker: marker::PhantomData<&'repo Repository>,
}

/// An iterator over all of the notes within a repository.
pub struct Notes<'repo> {
    raw: *mut raw::git_note_iterator,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> Note<'repo> {
    /// Get the note author
    pub fn author(&self) -> Signature<'_> {
        unsafe { signature::from_raw_const(self, raw::git_note_author(&*self.raw)) }
    }

    /// Get the note committer
    pub fn committer(&self) -> Signature<'_> {
        unsafe { signature::from_raw_const(self, raw::git_note_committer(&*self.raw)) }
    }

    /// Get the note message, in bytes.
    pub fn message_bytes(&self) -> &[u8] {
        unsafe { crate::opt_bytes(self, raw::git_note_message(&*self.raw)).unwrap() }
    }

    /// Get the note message as a string, returning `None` if it is not UTF-8.
    pub fn message(&self) -> Option<&str> {
        str::from_utf8(self.message_bytes()).ok()
    }

    /// Get the note object's id
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_note_id(&*self.raw)) }
    }
}

impl<'repo> Binding for Note<'repo> {
    type Raw = *mut raw::git_note;
    unsafe fn from_raw(raw: *mut raw::git_note) -> Note<'repo> {
        Note {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_note {
        self.raw
    }
}

impl<'repo> std::fmt::Debug for Note<'repo> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("Note").field("id", &self.id()).finish()
    }
}

impl<'repo> Drop for Note<'repo> {
    fn drop(&mut self) {
        unsafe {
            raw::git_note_free(self.raw);
        }
    }
}

impl<'repo> Binding for Notes<'repo> {
    type Raw = *mut raw::git_note_iterator;
    unsafe fn from_raw(raw: *mut raw::git_note_iterator) -> Notes<'repo> {
        Notes {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_note_iterator {
        self.raw
    }
}

impl<'repo> Iterator for Notes<'repo> {
    type Item = Result<(Oid, Oid), Error>;
    fn next(&mut self) -> Option<Result<(Oid, Oid), Error>> {
        let mut note_id = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        let mut annotated_id = note_id;
        unsafe {
            try_call_iter!(raw::git_note_next(
                &mut note_id,
                &mut annotated_id,
                self.raw
            ));
            Some(Ok((
                Binding::from_raw(&note_id as *const _),
                Binding::from_raw(&annotated_id as *const _),
            )))
        }
    }
}

impl<'repo> Drop for Notes<'repo> {
    fn drop(&mut self) {
        unsafe {
            raw::git_note_iterator_free(self.raw);
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        assert!(repo.notes(None).is_err());

        let sig = repo.signature().unwrap();
        let head = repo.head().unwrap().target().unwrap();
        let note = repo.note(&sig, &sig, None, head, "foo", false).unwrap();
        assert_eq!(repo.notes(None).unwrap().count(), 1);

        let note_obj = repo.find_note(None, head).unwrap();
        assert_eq!(note_obj.id(), note);
        assert_eq!(note_obj.message(), Some("foo"));

        let (a, b) = repo.notes(None).unwrap().next().unwrap().unwrap();
        assert_eq!(a, note);
        assert_eq!(b, head);

        assert_eq!(repo.note_default_ref().unwrap(), "refs/notes/commits");

        assert_eq!(sig.name(), note_obj.author().name());
        assert_eq!(sig.name(), note_obj.committer().name());
        assert!(sig.when() == note_obj.committer().when());
    }
}
