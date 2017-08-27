use std::marker;
use std::mem;
use std::slice;
use std::io;

use {raw, Oid, Object, Error};
use util::Binding;

/// A structure to represent a git [blob][1]
///
/// [1]: http://git-scm.com/book/en/Git-Internals-Git-Objects
pub struct Blob<'repo> {
    raw: *mut raw::git_blob,
    _marker: marker::PhantomData<Object<'repo>>,
}

impl<'repo> Blob<'repo> {
    /// Get the id (SHA1) of a repository blob
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(raw::git_blob_id(&*self.raw)) }
    }

    /// Determine if the blob content is most certainly binary or not.
    pub fn is_binary(&self) -> bool {
        unsafe { raw::git_blob_is_binary(&*self.raw) == 1 }
    }

    /// Get the content of this blob.
    pub fn content(&self) -> &[u8] {
        unsafe {
            let data = raw::git_blob_rawcontent(&*self.raw) as *const u8;
            let len = raw::git_blob_rawsize(&*self.raw) as usize;
            slice::from_raw_parts(data, len)
        }
    }

    /// Casts this Blob to be usable as an `Object`
    pub fn as_object(&self) -> &Object<'repo> {
        unsafe {
            &*(self as *const _ as *const Object<'repo>)
        }
    }

    /// Consumes Blob to be returned as an `Object`
    pub fn into_object(self) -> Object<'repo> {
        assert_eq!(mem::size_of_val(&self), mem::size_of::<Object>());
        unsafe {
            mem::transmute(self)
        }
    }
}

impl<'repo> Binding for Blob<'repo> {
    type Raw = *mut raw::git_blob;

    unsafe fn from_raw(raw: *mut raw::git_blob) -> Blob<'repo> {
        Blob {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_blob { self.raw }
}


impl<'repo> Drop for Blob<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_blob_free(self.raw) }
    }
}

/// A structure to represent a git writestream for blobs
pub struct BlobWriter<'repo> {
    raw: *mut raw::git_writestream,
    need_cleanup: bool,
    _marker: marker::PhantomData<Object<'repo>>,
}

impl<'repo> BlobWriter<'repo> {
    /// Finalize blob writing stream and write the blob to the object db
    pub fn commit(mut self) -> Result<Oid, Error> {
        // After commit we already doesn't need cleanup on drop
        self.need_cleanup = false;
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
        unsafe {
            try_call!(raw::git_blob_create_fromstream_commit(&mut raw, self.raw));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }
}

impl<'repo> Binding for BlobWriter<'repo> {
    type Raw = *mut raw::git_writestream;

    unsafe fn from_raw(raw: *mut raw::git_writestream) -> BlobWriter<'repo> {
        BlobWriter {
            raw: raw,
            need_cleanup: true,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_writestream { self.raw }
}

impl<'repo> Drop for BlobWriter<'repo> {
    fn drop(&mut self) {
        // We need cleanup in case the stream has not been committed
        if self.need_cleanup {
            unsafe { ((*self.raw).free)(self.raw) }
        }
    }
}

impl<'repo> io::Write for BlobWriter<'repo> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let res = ((*self.raw).write)(self.raw, buf.as_ptr() as *const i8, buf.len());
            if res < 0 {
                Err(io::Error::new(io::ErrorKind::Other, "Write error"))
            } else {
                Ok(buf.len())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::fs::File;
    use std::path::Path;
    use tempdir::TempDir;
    use Repository;

    #[test]
    fn buffer() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let id = repo.blob(&[5, 4, 6]).unwrap();
        let blob = repo.find_blob(id).unwrap();

        assert_eq!(blob.id(), id);
        assert_eq!(blob.content(), [5, 4, 6]);
        assert!(blob.is_binary());

        repo.find_object(id, None).unwrap().as_blob().unwrap();
        repo.find_object(id, None).unwrap().into_blob().ok().unwrap();
    }

    #[test]
    fn path() {
        let td = TempDir::new("test").unwrap();
        let path = td.path().join("foo");
        File::create(&path).unwrap().write_all(&[7, 8, 9]).unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let id = repo.blob_path(&path).unwrap();
        let blob = repo.find_blob(id).unwrap();
        assert_eq!(blob.content(), [7, 8, 9]);
        blob.into_object();
    }

    #[test]
    fn stream() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let mut ws = repo.blob_writer(Some(Path::new("foo"))).unwrap();
        let wl = ws.write(&[10, 11, 12]).unwrap();
        assert_eq!(wl, 3);
        let id = ws.commit().unwrap();
        let blob = repo.find_blob(id).unwrap();
        assert_eq!(blob.content(), [10, 11, 12]);
        blob.into_object();
    }
}
