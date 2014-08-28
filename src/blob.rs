use std::kinds::marker;
use std::mem;
use std::raw as stdraw;
use libc;

use {raw, Oid, Repository, Error};

/// A structure to represent a git [blob][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Blob<'a> {
    raw: *mut raw::git_blob,
    marker1: marker::ContravariantLifetime<'a>,
    marker2: marker::NoSend,
    marker3: marker::NoSync,
}

impl<'a> Blob<'a> {
    /// Create a new object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(_repo: &Repository,
                           raw: *mut raw::git_blob) -> Blob {
        Blob {
            raw: raw,
            marker1: marker::ContravariantLifetime,
            marker2: marker::NoSend,
            marker3: marker::NoSync,
        }
    }

    /// Write an in-memory buffer to the ODB as a blob.
    ///
    /// The Oid returned can in turn be passed to `lookup` to get a handle to
    /// the blob.
    pub fn new(repo: &Repository, data: &[u8]) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            let ptr = data.as_ptr() as *const libc::c_void;
            let len = data.len() as libc::size_t;
            try_call!(raw::git_blob_create_frombuffer(&mut raw, repo.raw(),
                                                      ptr, len));
            Ok(Oid::from_raw(&raw))
        }
    }

    /// Read a file from the filesystem and write its content to the Object
    /// Database as a loose blob
    ///
    /// The Oid returned can in turn be passed to `lookup` to get a handle to
    /// the blob.
    pub fn new_path(repo: &Repository, path: &Path) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0, ..raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_blob_create_fromdisk(&mut raw, repo.raw(),
                                                    path.to_c_str()));
            Ok(Oid::from_raw(&raw))
        }
    }

    /// Lookup a reference to one of the objects in a repository.
    pub fn lookup(repo: &Repository, oid: Oid) -> Result<Blob, Error> {
        let mut raw = 0 as *mut raw::git_blob;
        unsafe {
            try_call!(raw::git_blob_lookup(&mut raw, repo.raw(), oid.raw()));
            Ok(Blob::from_raw(repo, raw))
        }
    }

    /// Get the id (SHA1) of a repository blob
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_blob_id(&*self.raw)) }
    }

    /// Get access to the underlying raw pointer.
    pub fn raw(&self) -> *mut raw::git_blob { self.raw }

    /// Determine if the blob content is most certainly binary or not.
    pub fn is_binary(&self) -> bool {
        unsafe { raw::git_blob_is_binary(&*self.raw) == 1 }
    }

    /// Get the content of this blob.
    pub fn content(&self) -> &[u8] {
        unsafe {
            mem::transmute(stdraw::Slice {
                data: raw::git_blob_rawcontent(&*self.raw) as *const u8,
                len: raw::git_blob_rawsize(&*self.raw) as uint,
            })
        }
    }
}

#[unsafe_destructor]
impl<'a> Drop for Blob<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_blob_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{TempDir, File};
    use {Repository, Blob};

    #[test]
    fn buffer() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let id = Blob::new(&repo, &[5, 4, 6]).unwrap();
        let blob = Blob::lookup(&repo, id).unwrap();

        assert_eq!(blob.id(), id);
        assert_eq!(blob.content(), [5, 4, 6].as_slice());
    }

    #[test]
    fn path() {
        let td = TempDir::new("test").unwrap();
        let path = td.path().join("foo");
        File::create(&path).write(&[7, 8, 9]).unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let id = Blob::new_path(&repo, &path).unwrap();
        let blob = Blob::lookup(&repo, id).unwrap();
        assert_eq!(blob.content(), [7, 8, 9].as_slice());
    }
}
