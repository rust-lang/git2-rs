use std::marker;
use std::ptr;
use std::slice;
use libc::{c_int, c_uint, c_void, size_t};

use {panic, raw, Buf, Error, Oid, Repository, Revwalk};
use util::Binding;

/// Stages that are reported by the `PackBuilder` progress callback.
pub enum PackBuilderStage {
    /// Adding objects to the pack
    AddingObjects,
    /// Deltafication of the pack
    Deltafication,
}

pub type ProgressCb<'a> = FnMut(PackBuilderStage, u32, u32) -> bool + 'a;
pub type ForEachCb<'a> = FnMut(&[u8]) -> bool + 'a;

/// A builder for creating a packfile
pub struct PackBuilder<'repo> {
    raw: *mut raw::git_packbuilder,
    progress: Option<Box<Box<ProgressCb<'repo>>>>,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> PackBuilder<'repo> {
    /// Insert a single object. For an optimal pack it's mandatory to insert
    /// objects in recency order, commits followed by trees and blobs.
    pub fn insert_object(&mut self, id: Oid, name: Option<&str>) -> Result<(), Error> {
        let name = try!(::opt_cstr(name));
        unsafe {
            try_call!(raw::git_packbuilder_insert(self.raw, id.raw(), name));
        }
        Ok(())
    }

    /// Insert a root tree object. This will add the tree as well as all
    /// referenced trees and blobs.
    pub fn insert_tree(&mut self, id: Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_packbuilder_insert_tree(self.raw, id.raw()));
        }
        Ok(())
    }

    /// Insert a commit object. This will add a commit as well as the completed
    /// referenced tree.
    pub fn insert_commit(&mut self, id: Oid) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_packbuilder_insert_commit(self.raw, id.raw()));
        }
        Ok(())
    }

    /// Insert objects as given by the walk. Those commits and all objects they
    /// reference will be inserted into the packbuilder.
    pub fn insert_walk(&mut self, walk: &mut Revwalk) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_packbuilder_insert_walk(self.raw, walk.raw()));
        }
        Ok(())
    }

    /// Recursively insert an object and its referenced objects. Insert the
    /// object as well as any object it references.
    pub fn insert_recursive(&mut self, id: Oid, name: Option<&str>) -> Result<(), Error> {
        let name = try!(::opt_cstr(name));
        unsafe {
            try_call!(raw::git_packbuilder_insert_recur(self.raw, id.raw(), name));
        }
        Ok(())
    }

    /// Write the contents of the packfile to an in-memory buffer. The contents
    /// of the buffer will become a valid packfile, even though there will be
    /// no attached index.
    pub fn write_buf(&mut self, buf: &mut Buf) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_packbuilder_write_buf(buf.raw(), self.raw));
        }
        Ok(())
    }

    /// Create the new pack and pass each object to the callback.
    pub fn foreach<F>(&mut self, mut cb: F) -> Result<(), Error>
    where
        F: FnMut(&[u8]) -> bool,
    {
        let mut cb = &mut cb as &mut ForEachCb;
        let ptr = &mut cb as *mut _;
        unsafe {
            try_call!(raw::git_packbuilder_foreach(
                self.raw,
                foreach_c,
                ptr as *mut _
            ));
        }
        Ok(())
    }

    /// `progress` will be called with progress information during pack
    /// building. Be aware that this is called inline with pack building
    /// operations, so performance may be affected.
    ///
    /// There can only be one progress callback attached, this will replace any
    /// existing one. See `unset_progress_callback` to remove the current
    /// progress callback without attaching a new one.
    pub fn set_progress_callback<F>(&mut self, progress: F) -> Result<(), Error>
    where
        F: FnMut(PackBuilderStage, u32, u32) -> bool + 'repo,
    {
        let mut progress = Box::new(Box::new(progress) as Box<ProgressCb>);
        let ptr = &mut *progress as *mut _;
        let progress_c = Some(progress_c as raw::git_packbuilder_progress);
        unsafe {
            try_call!(raw::git_packbuilder_set_callbacks(
                self.raw,
                progress_c,
                ptr as *mut _
            ));
        }
        self.progress = Some(progress);
        Ok(())
    }

    /// Remove the current progress callback.  See `set_progress_callback` to
    /// set the progress callback.
    pub fn unset_progress_callback(&mut self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_packbuilder_set_callbacks(
                self.raw,
                None,
                ptr::null_mut()
            ));
            self.progress = None;
        }
        Ok(())
    }

    /// Get the total number of objects the packbuilder will write out.
    pub fn object_count(&self) -> usize {
        unsafe { raw::git_packbuilder_object_count(self.raw) }
    }

    /// Get the number of objects the packbuilder has already written out.
    pub fn written(&self) -> usize {
        unsafe { raw::git_packbuilder_written(self.raw) }
    }

    /// Get the packfile's hash. A packfile's name is derived from the sorted
    /// hashing of all object names. This is only correct after the packfile
    /// has been written.
    pub fn hash(&self) -> Option<Oid> {
        if self.object_count() == 0 {
            unsafe { Some(Binding::from_raw(raw::git_packbuilder_hash(self.raw))) }
        } else {
            None
        }
    }
}

impl<'repo> Binding for PackBuilder<'repo> {
    type Raw = *mut raw::git_packbuilder;
    unsafe fn from_raw(ptr: *mut raw::git_packbuilder) -> PackBuilder<'repo> {
        PackBuilder {
            raw: ptr,
            progress: None,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_packbuilder {
        self.raw
    }
}

impl<'repo> Drop for PackBuilder<'repo> {
    fn drop(&mut self) {
        unsafe {
            raw::git_packbuilder_set_callbacks(self.raw, None, ptr::null_mut());
            raw::git_packbuilder_free(self.raw);
        }
    }
}

impl Binding for PackBuilderStage {
    type Raw = raw::git_packbuilder_stage_t;
    unsafe fn from_raw(raw: raw::git_packbuilder_stage_t) -> PackBuilderStage {
        match raw {
            raw::GIT_PACKBUILDER_ADDING_OBJECTS => PackBuilderStage::AddingObjects,
            raw::GIT_PACKBUILDER_DELTAFICATION => PackBuilderStage::Deltafication,
            _ => panic!("Unknown git diff binary kind"),
        }
    }
    fn raw(&self) -> raw::git_packbuilder_stage_t {
        match *self {
            PackBuilderStage::AddingObjects => raw::GIT_PACKBUILDER_ADDING_OBJECTS,
            PackBuilderStage::Deltafication => raw::GIT_PACKBUILDER_DELTAFICATION,
        }
    }
}

extern "C" fn foreach_c(buf: *const c_void, size: size_t, data: *mut c_void) -> c_int {
    unsafe {
        let buf = slice::from_raw_parts(buf as *const u8, size as usize);

        let r = panic::wrap(|| {
            let data = data as *mut &mut ForEachCb;
            (*data)(buf)
        });
        if r == Some(true) {
            0
        } else {
            -1
        }
    }
}

extern "C" fn progress_c(
    stage: raw::git_packbuilder_stage_t,
    current: c_uint,
    total: c_uint,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let stage = Binding::from_raw(stage);

        let r = panic::wrap(|| {
            let data = data as *mut Box<ProgressCb>;
            (*data)(stage, current, total)
        });
        if r == Some(true) {
            0
        } else {
            -1
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs::File;
    use std::path::Path;
    use {Buf, Oid, Repository};

    fn commit(repo: &Repository) -> (Oid, Oid) {
        let mut index = t!(repo.index());
        let root = repo.path().parent().unwrap();
        t!(File::create(&root.join("foo")));
        t!(index.add_path(Path::new("foo")));

        let tree_id = t!(index.write_tree());
        let tree = t!(repo.find_tree(tree_id));
        let sig = t!(repo.signature());
        let head_id = t!(repo.refname_to_id("HEAD"));
        let parent = t!(repo.find_commit(head_id));
        let commit = t!(repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "commit",
            &tree,
            &[&parent]
        ));
        (commit, tree_id)
    }

    fn pack_header(len: u8) -> Vec<u8> {
        [].into_iter()
          .chain(b"PACK") // signature
          .chain(&[0, 0, 0, 2]) // version number
          .chain(&[0, 0, 0, len]) // number of objects
          .cloned().collect::<Vec<u8>>()
    }

    fn empty_pack_header() -> Vec<u8> {
        pack_header(0).iter()
          .chain(&[0x02, 0x9d, 0x08, 0x82, 0x3b,  // ^
                   0xd8, 0xa8, 0xea, 0xb5, 0x10,  // | SHA-1 of the zero
                   0xad, 0x6a, 0xc7, 0x5c, 0x82,  // | object pack header
                   0x3c, 0xfd, 0x3e, 0xd3, 0x1e]) // v
          .cloned().collect::<Vec<u8>>()
    }

    #[test]
    fn smoke() {
        let (_td, repo) = ::test::repo_init();
        let _builder = t!(repo.packbuilder());
    }

    #[test]
    fn smoke_write_buf() {
        let (_td, repo) = ::test::repo_init();
        let mut builder = t!(repo.packbuilder());
        let mut buf = Buf::new();
        t!(builder.write_buf(&mut buf));
        assert!(builder.hash().unwrap().is_zero());
        assert_eq!(&*buf, &*empty_pack_header());
    }

    #[test]
    fn smoke_foreach() {
        let (_td, repo) = ::test::repo_init();
        let mut builder = t!(repo.packbuilder());
        let mut buf = Vec::<u8>::new();
        t!(builder.foreach(|bytes| {
            buf.extend(bytes);
            true
        }));
        assert_eq!(&*buf, &*empty_pack_header());
    }

    #[test]
    fn insert_write_buf() {
        let (_td, repo) = ::test::repo_init();
        let mut builder = t!(repo.packbuilder());
        let mut buf = Buf::new();
        let (commit, _tree) = commit(&repo);
        t!(builder.insert_object(commit, None));
        assert_eq!(builder.object_count(), 1);
        t!(builder.write_buf(&mut buf));
        // Just check that the correct number of objects are written
        assert_eq!(&buf[0..12], &*pack_header(1));
    }

    #[test]
    fn insert_tree_write_buf() {
        let (_td, repo) = ::test::repo_init();
        let mut builder = t!(repo.packbuilder());
        let mut buf = Buf::new();
        let (_commit, tree) = commit(&repo);
        // will insert the tree itself and the blob, 2 objects
        t!(builder.insert_tree(tree));
        assert_eq!(builder.object_count(), 2);
        t!(builder.write_buf(&mut buf));
        // Just check that the correct number of objects are written
        assert_eq!(&buf[0..12], &*pack_header(2));
    }

    #[test]
    fn insert_commit_write_buf() {
        let (_td, repo) = ::test::repo_init();
        let mut builder = t!(repo.packbuilder());
        let mut buf = Buf::new();
        let (commit, _tree) = commit(&repo);
        // will insert the commit, its tree and the blob, 3 objects
        t!(builder.insert_commit(commit));
        assert_eq!(builder.object_count(), 3);
        t!(builder.write_buf(&mut buf));
        // Just check that the correct number of objects are written
        assert_eq!(&buf[0..12], &*pack_header(3));
    }

    #[test]
    fn progress_callback() {
        let mut progress_called = false;
        {
            let (_td, repo) = ::test::repo_init();
            let mut builder = t!(repo.packbuilder());
            let (commit, _tree) = commit(&repo);
            t!(builder.set_progress_callback(|_, _, _| {
                progress_called = true;
                true
            }));
            t!(builder.insert_commit(commit));
            t!(builder.write_buf(&mut Buf::new()));
        }
        assert_eq!(progress_called, true);
    }

    #[test]
    fn clear_progress_callback() {
        let mut progress_called = false;
        {
            let (_td, repo) = ::test::repo_init();
            let mut builder = t!(repo.packbuilder());
            let (commit, _tree) = commit(&repo);
            t!(builder.set_progress_callback(|_, _, _| {
                progress_called = true;
                true
            }));
            t!(builder.unset_progress_callback());
            t!(builder.insert_commit(commit));
            t!(builder.write_buf(&mut Buf::new()));
        }
        assert_eq!(progress_called, false);
    }
}
