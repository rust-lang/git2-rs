//! Custom backends for [`Odb`]s.
use crate::util::Binding;
use crate::{raw, Error, ErrorClass, ErrorCode, ObjectType, Oid};
use libc::{c_int, size_t};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::{ptr, slice};

pub trait OdbBackend {
    fn supported_operations(&self) -> SupportedOperations;

    fn read(
        &mut self,
        ctx: &OdbBackendContext,
        oid: Oid,
        out: &mut OdbBackendAllocation,
    ) -> Result<ObjectType, Error> {
        (ctx, oid, out);
        unimplemented!("OdbBackend::read")
    }
    // TODO: fn read_prefix(&mut self, ctx: &OdbBackendContext);
    fn read_header(&mut self, ctx: &OdbBackendContext, oid: Oid) -> Result<OdbHeader, Error> {
        (ctx, oid);
        unimplemented!("OdbBackend::read")
    }
    // TODO: fn write()
    // TODO: fn writestream()
    // TODO: fn readstream()
    fn exists(&mut self, ctx: &OdbBackendContext, oid: Oid) -> Result<bool, Error>;
    // TODO: fn exists_prefix()
    // TODO: fn refresh()
    // TODO: fn foreach()
    // TODO: fn writepack()
    // TODO: fn writemidx()
    // TODO: fn freshen()
}

bitflags::bitflags! {
    pub struct SupportedOperations: u32 {
        /// The backend supports the [`OdbBackend::read`] method.
        const READ = 1;
        /// The backend supports the [`OdbBackend::read_prefix`] method.
        const READ_PREFIX = 1 << 1;
        /// The backend supports the [`OdbBackend::read_header`] method.
        const READ_HEADER = 1 << 2;
        /// The backend supports the [`OdbBackend::write`] method.
        const WRITE = 1 << 3;
        /// The backend supports the [`OdbBackend::writestream`] method.
        const WRITESTREAM = 1 << 4;
        /// The backend supports the [`OdbBackend::readstream`] method.
        const READSTREAM = 1 << 5;
        /// The backend supports the [`OdbBackend::exists`] method.
        const EXISTS = 1 << 6;
        /// The backend supports the [`OdbBackend::exists_prefix`] method.
        const EXISTS_PREFIX = 1 << 7;
        /// The backend supports the [`OdbBackend::refresh`] method.
        const REFRESH = 1 << 7;
        /// The backend supports the [`OdbBackend::foreach`] method.
        const FOREACH = 1 << 8;
        /// The backend supports the [`OdbBackend::writepack`] method.
        const WRITEPACK = 1 << 9;
        /// The backend supports the [`OdbBackend::writemidx`] method.
        const WRITEMIDX = 1 << 10;
        /// The backend supports the [`OdbBackend::freshen`] method.
        const FRESHEN = 1 << 11;
    }
}

pub struct OdbHeader {
    pub size: usize,
    pub object_type: ObjectType,
}

pub struct OdbBackendAllocation {
    backend_ptr: *mut raw::git_odb_backend,
    raw: *mut c_void,
    size: usize,
}
impl OdbBackendAllocation {
    pub fn as_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.raw.cast(), self.size) }
    }
}
impl Drop for OdbBackendAllocation {
    fn drop(&mut self) {
        unsafe {
            raw::git_odb_backend_data_free(self.backend_ptr, self.raw);
        }
    }
}

pub struct OdbBackendRead {}

pub struct OdbBackendContext {
    backend_ptr: *mut raw::git_odb_backend,
}
impl OdbBackendContext {
    /// Creates an instance of `OdbBackendAllocation` that points to `null`.
    /// Its size will be 0.
    pub fn null_alloc(&self) -> OdbBackendAllocation {
        OdbBackendAllocation {
            backend_ptr: self.backend_ptr,
            raw: ptr::null_mut(),
            size: 0,
        }
    }

    /// Attempts to allocate a buffer of size `size`.
    ///
    /// # Return value
    /// `Some(allocation)` if the allocation succeeded.
    /// `None` otherwise. This usually indicates that there is not enough memory.
    pub fn alloc(&self, size: usize) -> Option<OdbBackendAllocation> {
        let data = unsafe { raw::git_odb_backend_data_alloc(self.backend_ptr, size as size_t) };
        if data.is_null() {
            return None;
        }
        Some(OdbBackendAllocation {
            backend_ptr: self.backend_ptr,
            raw: data,
            size,
        })
    }

    /// Attempts to allocate a buffer of size `size`, returning an error when that fails.
    /// Essentially the same as [`alloc`], but returns a [`Result`].
    ///
    /// # Return value
    /// `Ok(allocation)` if the allocation succeeded.
    /// `Err(error)` otherwise. The error is always a `GenericError` of class `NoMemory`.
    pub fn try_alloc(&self, size: usize) -> Result<OdbBackendAllocation, Error> {
        self.alloc(size).ok_or_else(|| {
            Error::new(
                ErrorCode::GenericError,
                ErrorClass::NoMemory,
                "out of memory",
            )
        })
    }
}

/// A handle to an [`OdbBackend`] that has been added to an [`Odb`](crate::Odb).
pub struct CustomOdbBackend<'a, B: OdbBackend> {
    // NOTE: Any pointer in this field must be both non-null and properly aligned.
    raw: NonNull<Backend<B>>,
    phantom: PhantomData<fn() -> &'a ()>,
}

impl<'a, B: OdbBackend> CustomOdbBackend<'a, B> {
    pub(crate) fn new_inner(backend: B) -> Box<Backend<B>> {
        let mut parent = raw::git_odb_backend {
            version: raw::GIT_ODB_BACKEND_VERSION,
            odb: ptr::null_mut(),
            read: None,
            read_prefix: None,
            read_header: None,
            write: None,
            writestream: None,
            readstream: None,
            exists: None,
            exists_prefix: None,
            refresh: None,
            foreach: None,
            writepack: None,
            writemidx: None,
            freshen: None,
            free: None,
        };
        Self::set_operations(backend.supported_operations(), &mut parent);

        Box::new(Backend {
            parent,
            inner: backend,
        })
    }
    pub(crate) fn new(backend: Box<Backend<B>>) -> Self {
        // SAFETY: Box::into_raw guarantees that the pointer is properly aligned and non-null
        let backend = Box::into_raw(backend);
        let backend = unsafe { NonNull::new_unchecked(backend) };
        Self {
            raw: backend,
            phantom: PhantomData,
        }
    }
    fn set_operations(
        supported_operations: SupportedOperations,
        backend: &mut raw::git_odb_backend,
    ) {
        macro_rules! op_if {
            ($name:ident if $flag:ident) => {
                backend.$name = supported_operations
                    .contains(SupportedOperations::$flag)
                    .then_some(Backend::<B>::$name)
            };
        }
        op_if!(read if READ);
        op_if!(read_header if READ_HEADER);
        op_if!(exists if EXISTS);

        backend.free = Some(Backend::<B>::free);
    }

    pub(crate) fn as_git_odb_backend(&self) -> *mut raw::git_odb_backend {
        self.raw.cast()
    }
}

#[repr(C)]
pub(crate) struct Backend<B> {
    parent: raw::git_odb_backend,
    inner: B,
}
impl<B: OdbBackend> Backend<B> {
    extern "C" fn read(
        data_ptr: *mut *mut c_void,
        size_ptr: *mut size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> raw::git_error_code {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let data = unsafe { data_ptr.as_mut().unwrap() };
        let size = unsafe { size_ptr.as_mut().unwrap() };
        let object_type = unsafe { otype_ptr.as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };

        let context = OdbBackendContext { backend_ptr };

        let mut allocation = ManuallyDrop::new(context.null_alloc());

        let output = match backend.inner.read(&context, oid, &mut allocation) {
            Err(e) => {
                ManuallyDrop::into_inner(allocation);
                return e.raw_code();
            }
            Ok(o) => o,
        };

        *size = allocation.size;
        *data = allocation.raw;
        *object_type = output.raw();

        raw::GIT_OK
    }
    extern "C" fn read_header(
        size_ptr: *mut size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> raw::git_error_code {
        let size = unsafe { size_ptr.as_mut().unwrap() };
        let otype = unsafe { otype_ptr.as_mut().unwrap() };
        let backend = unsafe { backend_ptr.cast::<Backend<B>>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };

        let context = OdbBackendContext { backend_ptr };

        let header = match backend.inner.read_header(&context, oid) {
            Err(e) => unsafe { return e.raw_set_git_error() },
            Ok(header) => header,
        };
        *size = header.size;
        *otype = header.object_type.raw();
        raw::GIT_OK
    }

    extern "C" fn free(backend: *mut raw::git_odb_backend) {
        let inner = unsafe { Box::from_raw(backend.cast::<Self>()) };
        drop(inner);
    }

    extern "C" fn exists(
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> c_int {
        let backend = unsafe { backend_ptr.cast::<Backend<B>>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };
        let context = OdbBackendContext { backend_ptr };
        let exists = match backend.inner.exists(&context, oid) {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };
        if exists {
            1
        } else {
            0
        }
    }
}
