use std::kinds::marker;
use std::mem;
use std::raw as stdraw;

use {raw, Oid, Repository};

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
    use Repository;

    #[test]
    fn buffer() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let id = repo.blob(&[5, 4, 6]).unwrap();
        let blob = repo.find_blob(id).unwrap();

        assert_eq!(blob.id(), id);
        assert_eq!(blob.content(), [5, 4, 6].as_slice());
    }

    #[test]
    fn path() {
        let td = TempDir::new("test").unwrap();
        let path = td.path().join("foo");
        File::create(&path).write(&[7, 8, 9]).unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let id = repo.blob_path(&path).unwrap();
        let blob = repo.find_blob(id).unwrap();
        assert_eq!(blob.content(), [7, 8, 9].as_slice());
    }
}
