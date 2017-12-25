use std::marker;
use std::io;
use std::ptr;
use std::slice;

use libc::{c_char, c_int, c_void};

use {raw, Oid, Object, ObjectType, Error};
use panic;
use util::Binding;

/// A structure to represent a git object database
pub struct Odb<'repo> {
    raw: *mut raw::git_odb,
    _marker: marker::PhantomData<Object<'repo>>,
}

impl<'repo> Binding for Odb<'repo> {
    type Raw = *mut raw::git_odb;

    unsafe fn from_raw(raw: *mut raw::git_odb) -> Odb<'repo> {
        Odb {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_odb { self.raw }
}

impl<'repo> Drop for Odb<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_odb_free(self.raw) }
    }
}

impl<'repo> Odb<'repo> {
    /// Create object database reading stream
    ///
    /// Note that most backends do not support streaming reads because they store their objects as compressed/delta'ed blobs.
    pub fn reader(&self, oid: Oid) -> Result<OdbReader, Error> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_odb_open_rstream(&mut out, self.raw, oid.raw()));
            Ok(OdbReader::from_raw(out))
        }
    }

    /// Create object database writing stream
    ///
    /// The type and final length of the object must be specified when opening the stream.
    pub fn writer(&self, size: usize, obj_type: ObjectType) -> Result<OdbWriter, Error> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_odb_open_wstream(&mut out, self.raw, size as raw::git_off_t, obj_type.raw()));
            Ok(OdbWriter::from_raw(out))
        }
    }

    /// Iterate over all objects in the object database
    pub fn foreach<C>(&self, mut callback: C) -> Result<(), Error>
        where C: FnMut(&Oid) -> bool
    {
        unsafe {
            let mut data = ForeachCbData { callback: &mut callback };
            try_call!(raw::git_odb_foreach(self.raw(),
                                           foreach_cb,
                                           &mut data as *mut _ as *mut _));
            Ok(())
        }
    }

    /// Read object from the database.
    pub fn read(&self, oid: Oid) -> Result<OdbObject, Error> {
        let mut out = ptr::null_mut();
        unsafe {
            try_call!(raw::git_odb_read(&mut out, self.raw, oid.raw()));
            Ok(OdbObject::from_raw(out))
        }
    }

    /// Checks if the object database has an object.
    pub fn exists(&self, oid: Oid) -> Result<bool, Error> {
        unsafe { Ok(raw::git_odb_exists(self.raw, oid.raw()) != -1) }
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
            raw: raw,
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *mut raw::git_odb_object { self.raw }
}

impl<'a> Drop for OdbObject<'a> {
    fn drop(&mut self) {
        unsafe { raw::git_odb_object_free(self.raw) }
    }
}

impl<'a> OdbObject<'a> {
    /// Get the object type.
    pub fn get_type(&self) -> Result<ObjectType, Error> {
        unsafe { Ok(ObjectType::from_raw(raw::git_odb_object_type(self.raw)).unwrap()) }
    }

    /// Get the object size.
    pub fn len(&self) -> Result<usize, Error> {
        unsafe { Ok(raw::git_odb_object_size(self.raw)) }
    }

    /// Get the object data.
    pub fn data(&self) -> Result<&[u8], Error> {
        unsafe {
            let result = self.len();
            if result.is_err() {
                return Err(result.err().unwrap());
            }
            let ptr : *const u8 = raw::git_odb_object_data(self.raw) as *const u8;
            let buffer = slice::from_raw_parts(ptr, result.unwrap());
            return Ok(buffer);
        }
    }
}

/// A structure to represent a git ODB rstream
pub struct OdbReader<'repo> {
    raw: *mut raw::git_odb_stream,
    _marker: marker::PhantomData<Object<'repo>>,
}

impl<'repo> Binding for OdbReader<'repo> {
    type Raw = *mut raw::git_odb_stream;

    unsafe fn from_raw(raw: *mut raw::git_odb_stream) -> OdbReader<'repo> {
        OdbReader {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_odb_stream { self.raw }
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

impl<'repo> OdbWriter<'repo> {
    /// Finish writing to an ODB stream
    ///
    /// This method can be used to finalize writing object to the database and get an identifier.
    /// The object will take its final name and will be available to the odb.
    /// This method will fail if the total number of received bytes differs from the size declared with odb_writer()
    /// Attepting write after finishing will be ignored.
    pub fn finalize(&mut self) -> Result<Oid, Error> {
        let mut raw = raw::git_oid { id: [0; raw::GIT_OID_RAWSZ] };
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
            raw: raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_odb_stream { self.raw }
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
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}

pub type ForeachCb<'a> = FnMut(&Oid) -> bool + 'a;

struct ForeachCbData<'a> {
    pub callback: &'a mut ForeachCb<'a>
}

extern fn foreach_cb(id: *const raw::git_oid,
                        payload: *mut c_void)
                        -> c_int
{
    panic::wrap(|| unsafe {
        let data = &mut *(payload as *mut ForeachCbData);
        let res = {
            let callback = &mut data.callback;
            callback(&Binding::from_raw(id))
        };

        if res { 0 } else { 1 }
    }).unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use tempdir::TempDir;
    use {Repository, ObjectType};

    #[test]
    fn reader() {
        let td = TempDir::new("test").unwrap();
        let repo = Repository::init(td.path()).unwrap();
        let dat = [4, 3, 5, 6, 9];
        let id = repo.blob(&dat).unwrap();
        let db = repo.odb().unwrap();
        let obj = db.read(id).unwrap();
        let data = obj.data().unwrap();
        let size = obj.len().unwrap();
        assert_eq!(size, 5);
        assert_eq!(dat, data);
    }

    #[test]
    fn writer() {
        let td = TempDir::new("test").unwrap();
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
}
