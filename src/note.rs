use std::kinds::marker;
use std::str;

use {raw, Signature, Oid};

/// A structure representing a [note][note] in git.
///
/// [note]: http://git-scm.com/blog/2010/08/25/notes.html
pub struct Note<'repo> {
    raw: *mut raw::git_note,
    marker: marker::ContravariantLifetime<'repo>,
}

/// An iterator over all of the notes within a repository.
pub struct Notes<'repo> {
    raw: *mut raw::git_note_iterator,
    marker: marker::ContravariantLifetime<'repo>,
}

impl<'repo> Note<'repo> {
    /// Create a new note from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_note) -> Note<'repo> {
        Note {
            raw: raw,
            marker: marker::ContravariantLifetime,
        }
    }

    /// Get the note author
    pub fn author(&self) -> Signature {
        unsafe {
            Signature::from_raw_const(self, raw::git_note_author(&*self.raw))
        }
    }

    /// Get the note committer
    pub fn committer(&self) -> Signature {
        unsafe {
            Signature::from_raw_const(self, raw::git_note_committer(&*self.raw))
        }
    }

    /// Get the note message, in bytes.
    pub fn message_bytes(&self) -> &[u8] {
        unsafe { ::opt_bytes(self, raw::git_note_message(&*self.raw)).unwrap() }
    }

    /// Get the note message as a string, returning `None` if it is not UTF-8.
    pub fn message(&self) -> Option<&str> {
        str::from_utf8(self.message_bytes()).ok()
    }

    /// Get the note object's id
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_note_id(&*self.raw)) }
    }
}

#[unsafe_destructor]
impl<'repo> Drop for Note<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_note_free(self.raw); }
    }
}

impl<'repo> Notes<'repo> {
    /// Create a new note iterator from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_note_iterator) -> Notes<'repo> {
        Notes {
            raw: raw,
            marker: marker::ContravariantLifetime,
        }
    }
}

impl<'repo> Iterator for Notes<'repo> {
    type Item = (Oid, Oid);
    fn next(&mut self) -> Option<(Oid, Oid)> {
        let mut note_id = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        let mut annotated_id = note_id;
        unsafe {
            match raw::git_note_next(&mut note_id, &mut annotated_id, self.raw) {
                0 => Some((Oid::from_raw(&note_id), Oid::from_raw(&annotated_id))),
                _ => None,
            }
        }
    }
}

#[unsafe_destructor]
impl<'repo> Drop for Notes<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_note_iterator_free(self.raw); }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        assert!(repo.notes(None).is_err());

        let sig = repo.signature().unwrap();
        let head = repo.head().unwrap().target().unwrap();
        let note = repo.note(&sig, &sig, None, head, "foo", false).unwrap();
        assert_eq!(repo.notes(None).unwrap().count(), 1);

        let note_obj = repo.find_note(None, head).unwrap();
        assert_eq!(note_obj.id(), note);
        assert_eq!(note_obj.message(), Some("foo"));

        let (a, b) = repo.notes(None).unwrap().next().unwrap();
        assert_eq!(a, note);
        assert_eq!(b, head);

        assert_eq!(repo.note_default_ref().unwrap(), "refs/notes/commits");
    }
}
