use std::marker;
use {raw, Repository, Oid, signature, Signature};
use util::{self, Binding};
use std::path::Path;
use std::ops::Range;
use std::mem;

/// Opaque structure to hold blame results.
pub struct Blame<'repo> {
    raw: *mut raw::git_blame,
    _marker: marker::PhantomData<&'repo Repository>,
}

/// Structure that represents a blame hunk.
pub struct BlameHunk<'blame> {
    raw: *mut raw::git_blame_hunk,
    _marker: marker::PhantomData<&'blame raw::git_blame>,
}

/// Blame options
pub struct BlameOptions {
    raw: raw::git_blame_options,
}

/// An iterator over the hunks in a blame.
pub struct BlameIter<'blame> {
    range: Range<usize>,
    blame: &'blame Blame<'blame>,
}

impl<'repo> Blame<'repo> {

    /// Gets the number of hunks that exist in the blame structure.
    pub fn len(&self) -> usize {
        unsafe { raw::git_blame_get_hunk_count(self.raw) as usize }
    }

    /// Gets the blame hunk at the given index.
    pub fn get_index(&self, index: usize) -> Option<BlameHunk> {
        unsafe {
            let ptr = raw::git_blame_get_hunk_byindex(self.raw(), index as u32);
            if ptr.is_null() {
                None
            } else {
                Some(BlameHunk::from_raw_const(ptr))
            }
        }
    }

    /// Gets the hunk that relates to the given line number in the newest
    /// commit.
    pub fn get_line(&self, lineno: usize) -> Option<BlameHunk> {
        unsafe {
            let ptr = raw::git_blame_get_hunk_byline(self.raw(), lineno);
            if ptr.is_null() {
                None
            } else {
                Some(BlameHunk::from_raw_const(ptr))
            }
        }
    }

    /// Returns an iterator over the hunks in this blame.
    pub fn iter(&self) -> BlameIter {
        BlameIter { range: 0..self.len(), blame: self }
    }

}

impl<'blame> BlameHunk<'blame> {

    unsafe fn from_raw_const(raw: *const raw::git_blame_hunk)
                                 -> BlameHunk<'blame> {
        BlameHunk {
            raw: raw as *mut raw::git_blame_hunk,
            _marker: marker::PhantomData,
        }
    }

    /// Returns OID of the commit where this line was last changed
    pub fn final_commit_id(&self) -> Oid {
        unsafe { Oid::from_raw(&(*self.raw).final_commit_id) }
    }

    /// Returns signature of the commit.
    pub fn final_signature(&self) -> Signature {
        unsafe { signature::from_raw_const(self, (*self.raw).final_signature) }
    }

    /// Returns line number where this hunk begins.
    ///
    /// Note that the start line is counting from 1.
    pub fn final_start_line(&self) -> usize {
        unsafe { (*self.raw).final_start_line_number }
    }

    /// Returns the OID of the commit where this hunk was found.
    ///
    /// This will usually be the same as `final_commit_id`,
    /// except when `BlameOptions::track_copies_any_commit_copies` has been
    /// turned on
    pub fn orig_commit_id(&self) -> Oid {
        unsafe { Oid::from_raw(&(*self.raw).orig_commit_id) }
    }

    /// Returns signature of the commit.
    pub fn orig_signature(&self) -> Signature {
        unsafe { signature::from_raw_const(self, (*self.raw).orig_signature) }
    }

    /// Returns line number where this hunk begins.
    ///
    /// Note that the start line is counting from 1.
    pub fn orig_start_line(&self) -> usize {
        unsafe { (*self.raw).orig_start_line_number}
    }

    /// Returns path to the file where this hunk originated.
    ///
    /// Note: `None` could be returned for non-unicode paths on Widnows.
    pub fn path(&self) -> Option<&Path> {
        unsafe {
            if let Some(bytes) = ::opt_bytes(self, (*self.raw).orig_path) {
                Some(util::bytes2path(bytes))
            } else {
                None
            }
        }
    }

    /// Tests whether this hunk has been tracked to a boundary commit
    /// (the root, or the commit specified in git_blame_options.oldest_commit).
    pub fn is_boundary(&self) -> bool {
        unsafe { (*self.raw).boundary == 1 }
    }

    /// Returns number of lines in this hunk.
    pub fn lines_in_hunk(&self) -> usize {
        unsafe { (*self.raw).lines_in_hunk as usize }
    }
}


impl Default for BlameOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl BlameOptions {

    /// Initialize options
    pub fn new() -> BlameOptions {
        unsafe {
            let mut raw: raw::git_blame_options = mem::zeroed();
            assert_eq!(
                raw::git_blame_init_options(&mut raw,
                                            raw::GIT_BLAME_OPTIONS_VERSION)
            , 0);

            Binding::from_raw(&raw as *const _ as *mut _)
        }
    }

    fn flag(&mut self, opt: u32, val: bool) -> &mut BlameOptions {
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Track lines that have moved within a file.
    pub fn track_copies_same_file(&mut self, opt: bool) -> &mut BlameOptions {
        self.flag(raw::GIT_BLAME_TRACK_COPIES_SAME_FILE, opt)
    }

    /// Track lines that have moved across files in the same commit.
    pub fn track_copies_same_commit_moves(&mut self, opt: bool) -> &mut BlameOptions {
        self.flag(raw::GIT_BLAME_TRACK_COPIES_SAME_COMMIT_MOVES, opt)
    }

    /// Track lines that have been copied from another file that exists
    /// in the same commit.
    pub fn track_copies_same_commit_copies(&mut self, opt: bool) -> &mut BlameOptions {
        self.flag(raw::GIT_BLAME_TRACK_COPIES_SAME_COMMIT_COPIES, opt)
    }

    /// Track lines that have been copied from another file that exists
    /// in any commit.
    pub fn track_copies_any_commit_copies(&mut self, opt: bool) -> &mut BlameOptions {
        self.flag(raw::GIT_BLAME_TRACK_COPIES_ANY_COMMIT_COPIES, opt)
    }

    /// Restrict the search of commits to those reachable following only
    /// the first parents.
    pub fn first_parent(&mut self, opt: bool) -> &mut BlameOptions {
        self.flag(raw::GIT_BLAME_FIRST_PARENT, opt)
    }

    /// Setter for the id of the newest commit to consider.
    pub fn newest_commit(&mut self, id: Oid) -> &mut BlameOptions {
        unsafe { self.raw.newest_commit = *id.raw(); }
        self
    }

    /// Setter for the id of the oldest commit to consider.
    pub fn oldest_commit(&mut self, id: Oid) -> &mut BlameOptions {
        unsafe { self.raw.oldest_commit = *id.raw(); }
        self
    }

}

impl<'repo> Binding for Blame<'repo> {
    type Raw = *mut raw::git_blame;

    unsafe fn from_raw(raw: *mut raw::git_blame) -> Blame<'repo> {
        Blame { raw: raw, _marker: marker::PhantomData }
    }

    fn raw(&self) -> *mut raw::git_blame { self.raw }
}

impl<'repo> Drop for Blame<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_blame_free(self.raw) }
    }
}

impl<'blame> Binding for BlameHunk<'blame> {
    type Raw = *mut raw::git_blame_hunk;

    unsafe fn from_raw(raw: *mut raw::git_blame_hunk) -> BlameHunk<'blame> {
        BlameHunk { raw: raw, _marker: marker::PhantomData }
    }

    fn raw(&self) -> *mut raw::git_blame_hunk { self.raw }
}

impl Binding for BlameOptions {
    type Raw = *mut raw::git_blame_options;

    unsafe fn from_raw(opts: *mut raw::git_blame_options) -> BlameOptions {
        BlameOptions { raw: *opts }
    }

    fn raw(&self) -> *mut raw::git_blame_options {
        &self.raw as *const _ as *mut _
    }
}

impl<'blame> Iterator for BlameIter<'blame> {
    type Item = BlameHunk<'blame>;
    fn next(&mut self) -> Option<BlameHunk<'blame>> {
        self.range.next().and_then(|i| self.blame.get_index(i))
    }

    fn size_hint(&self) -> (usize, Option<usize>) { self.range.size_hint() }
}

impl<'blame> DoubleEndedIterator for BlameIter<'blame> {
    fn next_back(&mut self) -> Option<BlameHunk<'blame>> {
        self.range.next_back().and_then(|i| self.blame.get_index(i))
    }
}

impl<'blame> ExactSizeIterator for BlameIter<'blame> {}

#[cfg(test)]
mod tests {
    use std::fs::{self, File};
    use std::path::Path;

    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        let mut index = repo.index().unwrap();

        let root = repo.path().parent().unwrap();
        fs::create_dir(&root.join("foo")).unwrap();
        File::create(&root.join("foo/bar")).unwrap();
        index.add_path(Path::new("foo/bar")).unwrap();

        let id = index.write_tree().unwrap();
        let tree = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        let id = repo.refname_to_id("HEAD").unwrap();
        let parent = repo.find_commit(id).unwrap();
        let commit = repo.commit(Some("HEAD"), &sig, &sig, "commit",
                                 &tree, &[&parent]).unwrap();

        let blame = repo.blame_file(Path::new("foo/bar"), None).unwrap();

        assert_eq!(blame.len(), 1);
        assert_eq!(blame.iter().count(), 1);

        let hunk = blame.get_index(0).unwrap();
        assert_eq!(hunk.final_commit_id(), commit);
        assert_eq!(hunk.final_signature().name(), sig.name());
        assert_eq!(hunk.final_signature().email(), sig.email());
        assert_eq!(hunk.final_start_line(), 1);
        assert_eq!(hunk.path(), Some(Path::new("foo/bar")));
        assert_eq!(hunk.lines_in_hunk(), 0);
        assert!(!hunk.is_boundary())
    }

}

