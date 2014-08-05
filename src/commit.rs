use std::kinds::marker;
use std::str;
use libc;

use {raw, Oid, Repository, Error, Signature};

/// A structure to represent a git [commit][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Commit<'a> {
    raw: *mut raw::git_commit,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoShare,
}

/// An iterator over the parent commits of a commit.
pub struct Parents<'a, 'b> {
    cur: uint,
    max: uint,
    commit: &'b Commit<'a>,
}

impl<'a> Commit<'a> {
    /// Create a new object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_commit) -> Commit {
        Commit {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        }
    }

    /// Lookup a reference to one of the commits in a repository.
    pub fn lookup(repo: &Repository, oid: Oid) -> Result<Commit, Error> {
        let mut raw = 0 as *mut raw::git_commit;
        unsafe {
            try_call!(raw::git_commit_lookup(&mut raw, repo.raw(), oid.raw()));
            Ok(Commit::from_raw(repo, raw))
        }
    }

    /// Get the id (SHA1) of a repository commit
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_commit_id(&*self.raw)) }
    }

    /// Get the id of the tree pointed to by this commit.
    ///
    /// No attempts are made to fetch an object from the
    pub fn tree_id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_commit_tree_id(&*self.raw)) }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_commit { self.raw }

    /// Get the full message of a commit.
    ///
    /// The returned message will be slightly prettified by removing any
    /// potential leading newlines.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn message(&self) -> Option<&str> {
        str::from_utf8(self.message_bytes())
    }

    /// Get the full message of a commit as a byte slice.
    ///
    /// The returned message will be slightly prettified by removing any
    /// potential leading newlines.
    pub fn message_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_commit_message(&*self.raw)).unwrap()
        }
    }

    /// Get the encoding for the message of a commit, as a string representing a
    /// standard encoding name.
    ///
    /// `None` will be returned if the encoding is not known
    pub fn message_encoding(&self) -> Option<&str> {
        let bytes = unsafe {
            ::opt_bytes(self, raw::git_commit_message(&*self.raw))
        };
        bytes.map(|b| str::from_utf8(b).unwrap())
    }

    /// Get the full raw message of a commit.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn message_raw(&self) -> Option<&str> {
        str::from_utf8(self.message_raw_bytes())
    }

    /// Get the full raw message of a commit.
    pub fn message_raw_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_commit_message_raw(&*self.raw)).unwrap()
        }
    }

    /// Get the full raw text of the commit header.
    ///
    /// `None` will be returned if the message is not valid utf-8
    pub fn raw_header(&self) -> Option<&str> {
        str::from_utf8(self.raw_header_bytes())
    }

    /// Get the full raw text of the commit header.
    pub fn raw_header_bytes(&self) -> &[u8] {
        unsafe {
            ::opt_bytes(self, raw::git_commit_raw_header(&*self.raw)).unwrap()
        }
    }

    /// Get the short "summary" of the git commit message.
    ///
    /// The returned message is the summary of the commit, comprising the first
    /// paragraph of the message with whitespace trimmed and squashed.
    ///
    /// `None` may be returned if an error occurs or if the summary is not valid
    /// utf-8.
    pub fn summary(&mut self) -> Option<&str> {
        self.summary_bytes().and_then(str::from_utf8)
    }

    /// Get the short "summary" of the git commit message.
    ///
    /// The returned message is the summary of the commit, comprising the first
    /// paragraph of the message with whitespace trimmed and squashed.
    ///
    /// `None` may be returned if an error occurs
    pub fn summary_bytes(&mut self) -> Option<&[u8]> {
        unsafe { ::opt_bytes(self, raw::git_commit_summary(self.raw)) }
    }

    /// Get the commit time (i.e. committer time) of a commit.
    ///
    /// The first element of the tuple is the time, in seconds, since the epoch.
    /// The second element is the offset, in minutes, of the time zone of the
    /// committer's preferred time zone.
    pub fn time(&self) -> (u64, int) {
        unsafe {
            (raw::git_commit_time(&*self.raw) as u64,
             raw::git_commit_time_offset(&*self.raw) as int)
        }
    }

    /// Creates a new iterator over the parents of this commit.
    pub fn parents<'b>(&'b self) -> Parents<'a, 'b> {
        Parents {
            cur: 0,
            max: unsafe { raw::git_commit_parentcount(&*self.raw) as uint },
            commit: self,
        }
    }

    /// Get the author of this commit.
    pub fn author(&self) -> Signature {
        unsafe {
            let ptr = raw::git_commit_author(&*self.raw);
            Signature::from_raw_const(self, ptr)
        }
    }

    /// Get the committer of this commit.
    pub fn committer(&self) -> Signature {
        unsafe {
            let ptr = raw::git_commit_committer(&*self.raw);
            Signature::from_raw_const(self, ptr)
        }
    }
}

impl<'a, 'b> Iterator<Commit<'a>> for Parents<'a, 'b> {
    fn next(&mut self) -> Option<Commit<'a>> {
        if self.cur == self.max { return None }
        self.cur += 1;
        let mut raw = 0 as *mut raw::git_commit;
        assert_eq!(unsafe {
            raw::git_commit_parent(&mut raw, &*self.commit.raw,
                                   (self.cur - 1) as libc::c_uint)
        }, 0);
        Some(Commit {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoShare,
        })
    }
}

#[unsafe_destructor]
impl<'a> Drop for Commit<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_commit_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{TempDir, File};
    use {Repository, Commit};

    #[test]
    fn smoke_revparse() {
        let td = TempDir::new("test").unwrap();
        git!(td.path(), "init");
        git!(td.path(), "config", "user.name", "foo");
        git!(td.path(), "config", "user.email", "bar");
        File::create(&td.path().join("foo")).write_str("foobar").unwrap();
        git!(td.path(), "add", ".");
        git!(td.path(), "commit", "-m", "foo");

        let repo = Repository::open(td.path()).unwrap();
        let head = repo.head().unwrap();
        let target = head.target().unwrap();
        let mut commit = Commit::lookup(&repo, target).unwrap();
        assert_eq!(commit.message(), Some("foo\n"));
        assert_eq!(commit.id(), target);
        commit.message_raw().unwrap();
        commit.raw_header().unwrap();
        commit.message_encoding();
        commit.summary().unwrap();
        commit.tree_id();
        assert_eq!(commit.parents().count(), 0);

        assert_eq!(commit.author().name(), Some("foo"));
        assert_eq!(commit.author().email(), Some("bar"));
        assert_eq!(commit.committer().name(), Some("foo"));
        assert_eq!(commit.committer().email(), Some("bar"));
    }
}

