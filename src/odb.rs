use std::io;
use std::marker;
use std::ptr;
use std::slice;

use std::ffi::CString;

use libc::{c_char, c_int, c_uint, c_void, size_t};

use crate::panic;
use crate::util::Binding;
use crate::{
    raw, Error, IndexerProgress, Mempack, Object, ObjectType, OdbLookupFlags, Oid, Progress,
};

/// A structure to represent a git object database
pub struct Odb<'repo> {
    raw: *mut raw::git_odb,
    _marker: marker::PhantomData<Object<'repo>>,
}

// `git_odb` uses locking and atomics internally.
unsafe impl<'repo> Send for Odb<'repo> {}
unsafe impl<'repo> Sync for Odb<'repo> {}

impl<'repo> Binding for Odb<'repo> {
    type Raw = *mut raw::git_odb;

    unsafe fn from_raw(raw: *mut raw::git_odb) -> Odb<'repo> {
        Odb {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_odb {
        self.raw
    }
}

impl<'repo> Drop for Odb<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_odb_free(self.raw) }
    }
}

impl<'repo> Odb<'repo> {
    /// Creates an object database without any backends.
    pub fn new<'a>() -> Result<Odb<'a>, Error> {
        crate::init();
        unsafe {
            let mut out = ptr::null_mut();
            try_call!(raw::git_odb_new(&mut out));
            Ok(Odb::from_raw(out))
        }
    }

    /// Create object database reading stream.
    ///
    /// Note that most backends do not support streaming reads because they store their objects as compressed/delta'ed blobs.
    /// If the backend does not support streaming reads, use the `read` method instead.
    pub fn reader(&self, oid: Oid) -> Result<(OdbReader<'_>, usize, ObjectType), Error> {
        let mut out = ptr::null_mut();
        let mut size = 0usize;
        let mut otype: raw::git_object_t = ObjectType::Any.raw();
        unsafe {
            try_call!(raw::git_odb_open_rstream(
                &mut out,
                &mut size,
                &mut otype,
                self.raw,
                oid.raw()
            ));
            Ok((
                OdbReader::from_raw(out),
                size,
                ObjectType::from_raw(otype).unwrap(),
            ))
        }
    }

    /// Create object database writing stream.
    ///
    /// The type and final length of the object must be specified when opening the stream.
    /// If the backend does not support streaming writes, use the `write` method instead.
    pub fn writer(&self, size: usize, obj_type: ObjectType) -> Result<OdbWriter<'_>, Error> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_odb_open_wstream(
                &mut out,
                self.raw,
                size as raw::git_object_size_t,
                obj_type.raw()
            ));
            Ok(OdbWriter::from_raw(out))
        }
    }

    /// Iterate over all objects in the object database.s
    pub fn foreach<C>(&self, mut callback: C) -> Result<(), Error>
    where
        C: FnMut(&Oid) -> bool,
    {
        unsafe {
            let mut data = ForeachCbData {
                callback: &mut callback,
            };
            let cb: raw::git_odb_foreach_cb = Some(foreach_cb);
            try_call!(raw::git_odb_foreach(
                self.raw(),
                cb,
                &mut data as *mut _ as *mut _
            ));
            Ok(())
        }
    }

    /// Read an object from the database.
    pub fn read(&self, oid: Oid) -> Result<OdbObject<'_>, Error> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_odb_read(&mut out, self.raw, oid.raw()));
            Ok(OdbObject::from_raw(out))
        }
    }

    /// Reads the header of an object from the database
    /// without reading the full content.
    pub fn read_header(&self, oid: Oid) -> Result<(usize, ObjectType), Error> {
        let mut size: usize = 0;
        let mut kind_id: i32 = ObjectType::Any.raw();

        unsafe {
            try_call!(raw::git_odb_read_header(
                &mut size as *mut size_t,
                &mut kind_id as *mut raw::git_object_t,
                self.raw,
                oid.raw()
            ));

            Ok((size, ObjectType::from_raw(kind_id).unwrap()))
        }
    }

    /// Write an object to the database.
    pub fn write(&self, kind: ObjectType, data: &[u8]) -> Result<Oid, Error> {
        unsafe {
            let mut out = raw::git_oid {
                id: [0; raw::GIT_OID_RAWSZ],
            };
            try_call!(raw::git_odb_write(
                &mut out,
                self.raw,
                data.as_ptr() as *const c_void,
                data.len(),
                kind.raw()
            ));
            Ok(Oid::from_raw(&mut out))
        }
    }

    /// Create stream for writing a pack file to the ODB
    pub fn packwriter(&self) -> Result<OdbPackwriter<'_>, Error> {
        let mut out = ptr::null_mut();
        let progress_cb: raw::git_indexer_progress_cb = Some(write_pack_progress_cb);
        let progress_payload = Box::new(OdbPackwriterCb { cb: None });
        let progress_payload_ptr = Box::into_raw(progress_payload);

        unsafe {
            try_call!(raw::git_odb_write_pack(
                &mut out,
                self.raw,
                progress_cb,
                progress_payload_ptr as *mut c_void
            ));
        }

        Ok(OdbPackwriter {
            raw: out,
            progress: Default::default(),
            progress_payload_ptr,
        })
    }

    /// Checks if the object database has an object.
    pub fn exists(&self, oid: Oid) -> bool {
        unsafe { raw::git_odb_exists(self.raw, oid.raw()) != 0 }
    }

    /// Checks if the object database has an object, with extended flags.
    pub fn exists_ext(&self, oid: Oid, flags: OdbLookupFlags) -> bool {
        unsafe { raw::git_odb_exists_ext(self.raw, oid.raw(), flags.bits() as c_uint) != 0 }
    }

    /// Potentially finds an object that starts with the given prefix.
    pub fn exists_prefix(&self, short_oid: Oid, len: usize) -> Result<Oid, Error> {
        unsafe {
            let mut out = raw::git_oid {
                id: [0; raw::GIT_OID_RAWSZ],
            };
            try_call!(raw::git_odb_exists_prefix(
                &mut out,
                self.raw,
                short_oid.raw(),
                len
            ));
            Ok(Oid::from_raw(&out))
        }
    }

    /// Refresh the object database.
    /// This should never be needed, and is
    /// provided purely for convenience.
    /// The object database will automatically
    /// refresh when an object is not found when
    /// requested.
    pub fn refresh(&self) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_odb_refresh(self.raw));
            Ok(())
        }
    }

    /// Adds an alternate disk backend to the object database.
    pub fn add_disk_alternate(&self, path: &str) -> Result<(), Error> {
        unsafe {
            let path = CString::new(path)?;
            try_call!(raw::git_odb_add_disk_alternate(self.raw, path));
            Ok(())
        }
    }

    /// Create a new mempack backend, and add it to this odb with the given
    /// priority. Higher values give the backend higher precedence. The default
    /// loose and pack backends have priorities 1 and 2 respectively (hard-coded
    /// in libgit2). A reference to the new mempack backend is returned on
    /// success. The lifetime of the backend must be contained within the
    /// lifetime of this odb, since deletion of the odb will also result in
    /// deletion of the mempack backend.
    ///
    /// Here is an example that fails to compile because it tries to hold the
    /// mempack reference beyond the Odb's lifetime:
    ///
    /// ```compile_fail
    /// use git2::Odb;
    /// let mempack = {
    ///     let odb = Odb::new().unwrap();
    ///     odb.add_new_mempack_backend(1000).unwrap()
    /// };
    /// ```
    pub fn add_new_mempack_backend<'odb>(
        &'odb self,
        priority: i32,
    ) -> Result<Mempack<'odb>, Error> {
        unsafe {
            let mut mempack = ptr::null_mut();
            // The mempack backend object in libgit2 is only ever freed by an
            // odb that has the backend in its list. So to avoid potentially
            // leaking the mempack backend, this API ensures that the backend
            // is added to the odb before returning it. The lifetime of the
            // mempack is also bound to the lifetime of the odb, so that users
            // can't end up with a dangling reference to a mempack object that
            // was actually freed when the odb was destroyed.
            try_call!(raw::git_mempack_new(&mut mempack));
            try_call!(raw::git_odb_add_backend(
                self.raw,
                mempack,
                priority as c_int
            ));
            Ok(Mempack::from_raw(mempack))
        }
    }
}

/// An object from the Object Database.
pub struct OdbObject<'a> {
    raw: *mut raw::git_odb_object,
    _marker: marker::PhantomData<Object<'a>>,
}

impl<'a> Binding for OdbObject<'a> {
    type Raw = *mut raw::git_odb_object;

    unsafe fn from_raw(raw: *mut raw::git_odb_object) -> OdbObject<'a> {
        OdbObject {
            raw,
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *mut raw::git_odb_object {
        self.raw
    }
}

impl<'a> Drop for OdbObject<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_odb_object_free(self.raw) }
    }
}

impl<'a> OdbObject<'a> {
    /// Get the object type.
    pub fn kind(&self) -> ObjectType {
        unsafe { ObjectType::from_raw(raw::git_odb_object_type(self.raw)).unwrap() }
    }

    /// Get the object size.
    pub fn len(&self) -> usize {
        unsafe { raw::git_odb_object_size(self.raw) }
    }

    /// Get the object data.
    pub fn data(&self) -> &[u8] {
        unsafe {
            let size = self.len();
            let ptr: *const u8 = raw::git_odb_object_data(self.raw) as *const u8;
            let buffer = slice::from_raw_parts(ptr, size);
            return buffer;
        }
    }

    /// Get the object id.
    pub fn id(&self) -> Oid {
        unsafe { Oid::from_raw(raw::git_odb_object_id(self.raw)) }
    }
}

/// A structure to represent a git ODB rstream
pub struct OdbReader<'repo> {
    raw: *mut raw::git_odb_stream,
    _marker: marker::PhantomData<Object<'repo>>,
}

// `git_odb_stream` is not thread-safe internally, so it can't use `Sync`, but moving it to another
// thread and continuing to read will work.
unsafe impl<'repo> Send for OdbReader<'repo> {}

impl<'repo> Binding for OdbReader<'repo> {
    type Raw = *mut raw::git_odb_stream;

    unsafe fn from_raw(raw: *mut raw::git_odb_stream) -> OdbReader<'repo> {
        OdbReader {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_odb_stream {
        self.raw
    }
}

impl<'repo> Drop for OdbReader<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_odb_stream_free(self.raw) }
    }
}

impl<'repo> io::Read for OdbReader<'repo> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        unsafe {
            let ptr = buf.as_ptr() as *mut c_char;
            let len = buf.len();
            let res = raw::git_odb_stream_read(self.raw, ptr, len);
            if res < 0 {
                Err(io::Error::new(io::ErrorKind::Other, "Read error"))
            } else {
                Ok(len)
            }
        }
    }
}

/// A structure to represent a git ODB wstream
pub struct OdbWriter<'repo> {
    raw: *mut raw::git_odb_stream,
    _marker: marker::PhantomData<Object<'repo>>,
}

// `git_odb_stream` is not thread-safe internally, so it can't use `Sync`, but moving it to another
// thread and continuing to write will work.
unsafe impl<'repo> Send for OdbWriter<'repo> {}

impl<'repo> OdbWriter<'repo> {
    /// Finish writing to an ODB stream
    ///
    /// This method can be used to finalize writing object to the database and get an identifier.
    /// The object will take its final name and will be available to the odb.
    /// This method will fail if the total number of received bytes differs from the size declared with odb_writer()
    /// Attempting write after finishing will be ignored.
    pub fn finalize(&mut self) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_odb_stream_finalize_write(&mut raw, self.raw));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }
}

impl<'repo> Binding for OdbWriter<'repo> {
    type Raw = *mut raw::git_odb_stream;

    unsafe fn from_raw(raw: *mut raw::git_odb_stream) -> OdbWriter<'repo> {
        OdbWriter {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_odb_stream {
        self.raw
    }
}

impl<'repo> Drop for OdbWriter<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_odb_stream_free(self.raw) }
    }
}

impl<'repo> io::Write for OdbWriter<'repo> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ptr = buf.as_ptr() as *const c_char;
            let len = buf.len();
            let res = raw::git_odb_stream_write(self.raw, ptr, len);
            if res < 0 {
                Err(io::Error::new(io::ErrorKind::Other, "Write error"))
            } else {
                Ok(buf.len())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) struct OdbPackwriterCb<'repo> {
    pub(crate) cb: Option<Box<IndexerProgress<'repo>>>,
}

/// A stream to write a packfile to the ODB
pub struct OdbPackwriter<'repo> {
    raw: *mut raw::git_odb_writepack,
    progress: raw::git_indexer_progress,
    progress_payload_ptr: *mut OdbPackwriterCb<'repo>,
}

impl<'repo> OdbPackwriter<'repo> {
    /// Finish writing the packfile
    pub fn commit(&mut self) -> Result<i32, Error> {
        unsafe {
            let writepack = &*self.raw;
            let res = match writepack.commit {
                Some(commit) => commit(self.raw, &mut self.progress),
                None => -1,
            };

            if res < 0 {
                Err(Error::last_error(res).unwrap())
            } else {
                Ok(res)
            }
        }
    }

    /// The callback through which progress is monitored. Be aware that this is
    /// called inline, so performance may be affected.
    pub fn progress<F>(&mut self, cb: F) -> &mut OdbPackwriter<'repo>
    where
        F: FnMut(Progress<'_>) -> bool + 'repo,
    {
        let progress_payload =
            unsafe { &mut *(self.progress_payload_ptr as *mut OdbPackwriterCb<'_>) };

        progress_payload.cb = Some(Box::new(cb) as Box<IndexerProgress<'repo>>);
        self
    }
}

impl<'repo> io::Write for OdbPackwriter<'repo> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ptr = buf.as_ptr() as *mut c_void;
            let len = buf.len();

            let writepack = &*self.raw;
            let res = match writepack.append {
                Some(append) => append(self.raw, ptr, len, &mut self.progress),
                None => -1,
            };

            if res < 0 {
                Err(io::Error::new(io::ErrorKind::Other, "Write error"))
            } else {
                Ok(buf.len())
            }
        }
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<'repo> Drop for OdbPackwriter<'repo> {
    fn drop(&mut self) {
        unsafe {
            let writepack = &*self.raw;
            match writepack.free {
                Some(free) => free(self.raw),
                None => (),
            };

            drop(Box::from_raw(self.progress_payload_ptr));
        }
    }
}

pub type ForeachCb<'a> = dyn FnMut(&Oid) -> bool + 'a;

struct ForeachCbData<'a> {
    pub callback: &'a mut ForeachCb<'a>,
}

extern "C" fn foreach_cb(id: *const raw::git_oid, payload: *mut c_void) -> c_int {
    panic::wrap(|| unsafe {
        let data = &mut *(payload as *mut ForeachCbData<'_>);
        let res = {
            let callback = &mut data.callback;
            callback(&Binding::from_raw(id))
        };

        if res {
            0
        } else {
            1
        }
    })
    .unwrap_or(1)
}

pub(crate) extern "C" fn write_pack_progress_cb(
    stats: *const raw::git_indexer_progress,
    payload: *mut c_void,
) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(payload as *mut OdbPackwriterCb<'_>);

        let callback = match payload.cb {
            Some(ref mut cb) => cb,
            None => return true,
        };

        let progress: Progress<'_> = Binding::from_raw(stats);
        callback(progress)
    });
    if ok == Some(true) {
        0
    } else {
        -1
    }
}

#[cfg(test)]
mod tests {
    use crate::{Buf, ObjectType, Oid, Repository};
    use std::io::prelude::*;
    use tempfile::TempDir;

    #[test]
    fn read() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let id = repo.blob(&dat).unwrap();
        let db = repo.odb().unwrap();
        let obj = db.read(id).unwrap();
        let data = obj.data();
        let size = obj.len();
        assert_eq!(size, 5);
        assert_eq!(dat, data);
        assert_eq!(id, obj.id());
    }

    #[test]
    fn read_header() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let id = repo.blob(&dat).unwrap();
        let db = repo.odb().unwrap();
        let (size, kind) = db.read_header(id).unwrap();

        assert_eq!(size, 5);
        assert_eq!(kind, ObjectType::Blob);
    }

    #[test]
    fn write() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let db = repo.odb().unwrap();
        let id = db.write(ObjectType::Blob, &dat).unwrap();
        let blob = repo.find_blob(id).unwrap();
        assert_eq!(blob.content(), dat);
    }

    #[test]
    fn writer() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let db = repo.odb().unwrap();
        let mut ws = db.writer(dat.len(), ObjectType::Blob).unwrap();
        let wl = ws.write(&dat[0..3]).unwrap();
        assert_eq!(wl, 3);
        let wl = ws.write(&dat[3..5]).unwrap();
        assert_eq!(wl, 2);
        let id = ws.finalize().unwrap();
        let blob = repo.find_blob(id).unwrap();
        assert_eq!(blob.content(), dat);
    }

    #[test]
    fn exists() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let db = repo.odb().unwrap();
        let id = db.write(ObjectType::Blob, &dat).unwrap();
        assert!(db.exists(id));
    }

    #[test]
    fn exists_prefix() {
        let td = TempDir::new().unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let db = repo.odb().unwrap();
        let id = db.write(ObjectType::Blob, &dat).unwrap();
        let id_prefix_str = &id.to_string()[0..10];
        let id_prefix = Oid::from_str(id_prefix_str).unwrap();
        let found_oid = db.exists_prefix(id_prefix, 10).unwrap();
        assert_eq!(found_oid, id);
    }

    #[test]
    fn packwriter() {
        let (_td, repo_source) = crate::test::repo_init();
        let (_td, repo_target) = crate::test::repo_init();
        let mut builder = t!(repo_source.packbuilder());
        let mut buf = Buf::new();
        let (commit_source_id, _tree) = crate::test::commit(&repo_source);
        t!(builder.insert_object(commit_source_id, None));
        t!(builder.write_buf(&mut buf));
        let db = repo_target.odb().unwrap();
        let mut packwriter = db.packwriter().unwrap();
        packwriter.write(&buf).unwrap();
        packwriter.commit().unwrap();
        let commit_target = repo_target.find_commit(commit_source_id).unwrap();
        assert_eq!(commit_target.id(), commit_source_id);
    }

    #[test]
    fn packwriter_progress() {
        let mut progress_called = false;
        {
            let (_td, repo_source) = crate::test::repo_init();
            let (_td, repo_target) = crate::test::repo_init();
            let mut builder = t!(repo_source.packbuilder());
            let mut buf = Buf::new();
            let (commit_source_id, _tree) = crate::test::commit(&repo_source);
            t!(builder.insert_object(commit_source_id, None));
            t!(builder.write_buf(&mut buf));
            let db = repo_target.odb().unwrap();
            let mut packwriter = db.packwriter().unwrap();
            packwriter.progress(|_| {
                progress_called = true;
                true
            });
            packwriter.write(&buf).unwrap();
            packwriter.commit().unwrap();
        }
        assert_eq!(progress_called, true);
    }

    #[test]
    fn write_with_mempack() {
        use crate::{Buf, ResetType};
        use std::io::Write;
        use std::path::Path;

        // Create a repo, add a mempack backend
        let (_td, repo) = crate::test::repo_init();
        let odb = repo.odb().unwrap();
        let mempack = odb.add_new_mempack_backend(1000).unwrap();

        // Sanity check that foo doesn't exist initially
        let foo_file = Path::new(repo.workdir().unwrap()).join("foo");
        assert!(!foo_file.exists());

        // Make a commit that adds foo. This writes new stuff into the mempack
        // backend.
        let (oid1, _id) = crate::test::commit(&repo);
        let commit1 = repo.find_commit(oid1).unwrap();
        t!(repo.reset(commit1.as_object(), ResetType::Hard, None));
        assert!(foo_file.exists());

        // Dump the mempack modifications into a buf, and reset it. This "erases"
        // commit-related objects from the repository. Ensure the commit appears
        // to have become invalid, by checking for failure in `reset --hard`.
        let mut buf = Buf::new();
        mempack.dump(&repo, &mut buf).unwrap();
        mempack.reset().unwrap();
        assert!(repo
            .reset(commit1.as_object(), ResetType::Hard, None)
            .is_err());

        // Write the buf into a packfile in the repo. This brings back the
        // missing objects, and we verify everything is good again.
        let mut packwriter = odb.packwriter().unwrap();
        packwriter.write(&buf).unwrap();
        packwriter.commit().unwrap();
        t!(repo.reset(commit1.as_object(), ResetType::Hard, None));
        assert!(foo_file.exists());
    }
}
