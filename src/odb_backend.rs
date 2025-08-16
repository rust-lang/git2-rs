//! Custom backends for [`Odb`]s.
//!
//! [`Odb`]: crate::Odb
use crate::util::Binding;
use crate::{raw, Error, ErrorClass, ErrorCode, ObjectType, Oid};
use bitflags::bitflags;
use libc::{c_int, size_t};
use std::ffi::c_void;
use std::marker::PhantomData;
use std::mem::ManuallyDrop;
use std::ptr::NonNull;
use std::{ptr, slice};

/// A custom implementation of an [`Odb`] backend.
///
/// # Errors
///
/// If the backend does not have enough memory, the error code should be
/// [`ErrorCode::GenericError`] and the class should be [`ErrorClass::NoMemory`].
#[allow(unused_variables)]
pub trait OdbBackend {
    /// Returns the supported operations of this backend.
    /// The return value is used to determine what functions to provide to libgit2.
    ///
    /// This method is only called once in [`Odb::add_custom_backend`] and once in every call to
    /// [`CustomOdbBackend::refresh_operations`]; in general, it is called very rarely.
    /// Very few implementations should change their available operations after being added to an
    /// [`Odb`].
    ///
    /// [`Odb`]: crate::Odb
    /// [`Odb::add_custom_backend`]: crate::Odb::add_custom_backend
    fn supported_operations(&self) -> SupportedOperations;

    /// Read an object.
    ///
    /// Corresponds to the `read` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::READ`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// If an implementation returns `Ok(())`, `object_type` and `data` SHOULD be
    /// set to the object type and the contents of the object respectively.
    ///
    /// [`OdbBackendAllocation`]s SHOULD be created using `ctx` (see
    /// [`OdbBackendContext::try_alloc`]).
    ///
    /// # Errors
    ///
    /// If an object does not exist, the error code should be [`ErrorCode::NotFound`].
    ///
    /// See [`OdbBackend`] for more recommendations.
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn read(
        &mut self,
        ctx: &OdbBackendContext,
        oid: Oid,
        object_type: &mut ObjectType,
        data: &mut OdbBackendAllocation,
    ) -> Result<(), Error> {
        unimplemented!("OdbBackend::read")
    }

    /// Find and read an object based on a prefix of its [`Oid`].
    ///
    /// Corresponds to the `read_prefix` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::READ_PREFIX`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// Only the first `oid_prefix_len * 4` bits of `oid_prefix` are set.
    /// The remaining `(GIT_OID_SHA1_HEXSIZE - oid_prefix_len) * 4` bits are set to 0.
    ///
    /// If an implementation returns `Ok(())`, `oid`, `data`, and `object_type` MUST be set to the
    /// full object ID, the object type, and the contents of the object respectively.
    ///
    /// [`OdbBackendAllocation`]s SHOULD be created using `ctx` (see
    /// [`OdbBackendContext::try_alloc`]).
    ///
    /// # Errors
    ///
    /// See [`OdbBackend::read`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn read_prefix(
        &mut self,
        ctx: &OdbBackendContext,
        oid_prefix: Oid,
        oid_prefix_length: usize,
        oid: &mut Oid,
        object_type: &mut ObjectType,
        data: &mut OdbBackendAllocation,
    ) -> Result<(), Error> {
        unimplemented!("OdbBackend::read_prefix")
    }

    /// Read an object's length and object type but not its contents.
    ///
    /// Corresponds to the `read_header` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::READ_HEADER`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// If an implementation returns `Ok(())`, `length` and `object_type` MUST be set to the
    /// length of the object's contents and the object type respectively.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend::read`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn read_header(
        &mut self,
        ctx: &OdbBackendContext,
        oid: Oid,
        length: &mut usize,
        object_type: &mut ObjectType,
    ) -> Result<(), Error> {
        unimplemented!("OdbBackend::read_header")
    }

    /// Check if an object exists.
    ///
    /// Corresponds to the `exists` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::EXISTS`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// # Implementation notes
    ///
    /// An implementation SHOULD return `Ok(true)` if the object exists, and `Ok(false)` if the
    /// object does not exist.
    ///
    /// # Errors
    ///
    /// Errors SHOULD NOT be indicated through returning `Ok(false)`, but SHOULD through the use
    /// of [`Error`].
    ///
    /// See [`OdbBackend`] for more recommendations.
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn exists(&mut self, ctx: &OdbBackendContext, oid: Oid) -> Result<bool, Error> {
        unimplemented!("OdbBackend::exists")
    }

    /// Check if an object exists based on a prefix of its [`Oid`].
    ///
    /// Corresponds to the `exists_prefix` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::EXISTS_PREFIX`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// Only the first `oid_prefix_len * 4` bits of `oid_prefix` are set.
    /// The remaining `(GIT_OID_SHA1_HEXSIZE - oid_prefix_len) * 4` bits are set to 0.
    ///
    /// If an implementation returns `Ok(oid)`, `oid` SHOULD be the full Oid of the object.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend::exists`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn exists_prefix(
        &mut self,
        ctx: &OdbBackendContext,
        oid_prefix: Oid,
        oid_prefix_length: usize,
    ) -> Result<Oid, Error> {
        unimplemented!("OdbBackend::exists_prefix")
    }

    // TODO: fn write()
    // TODO: fn writestream()
    // TODO: fn readstream()
    // TODO: fn exists_prefix()
    // TODO: fn refresh()
    // TODO: fn foreach()
    // TODO: fn writepack()
    // TODO: fn writemidx()
    // TODO: fn freshen()
}

bitflags! {
    /// Supported operations for a backend.
    pub struct SupportedOperations: u32 {
        // NOTE: The names are taken from the trait method names, but the order is taken from the
        //       fields of git_odb_backend.
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

/// An allocation that can be passed to libgit2.
///
/// In addition to managing the pointer, this struct also keeps track of the size of an allocation.
///
/// Internally, allocations are made using [`git_odb_backend_data_malloc`] and freed using
/// [`git_odb_backend_data_free`].
///
/// [`git_odb_backend_data_malloc`]: raw::git_odb_backend_data_alloc
/// [`git_odb_backend_data_free`]: raw::git_odb_backend_data_free
pub struct OdbBackendAllocation {
    backend_ptr: *mut raw::git_odb_backend,
    raw: NonNull<c_void>,
    size: usize,
}
impl OdbBackendAllocation {
    /// Returns this allocation as a byte slice.
    pub fn as_mut(&mut self) -> &mut [u8] {
        unsafe { slice::from_raw_parts_mut(self.raw.cast().as_ptr(), self.size) }
    }
}
impl Drop for OdbBackendAllocation {
    fn drop(&mut self) {
        unsafe {
            raw::git_odb_backend_data_free(self.backend_ptr, self.raw.as_ptr());
        }
    }
}

/// Information passed to most of [`OdbBackend`]'s methods.
pub struct OdbBackendContext {
    backend_ptr: *mut raw::git_odb_backend,
}
impl OdbBackendContext {
    /// Creates an instance of `OdbBackendAllocation` that is zero-sized.
    /// This is useful for representing non-allocations.
    pub const fn alloc_0(&self) -> OdbBackendAllocation {
        OdbBackendAllocation {
            backend_ptr: self.backend_ptr,
            raw: NonNull::dangling(),
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
        let data = NonNull::new(data)?;
        Some(OdbBackendAllocation {
            backend_ptr: self.backend_ptr,
            raw: data,
            size,
        })
    }

    /// Attempts to allocate a buffer of size `size`, returning an error when that fails.
    /// Essentially the same as [`OdbBackendContext::alloc`], but returns a [`Result`] instead.
    ///
    /// # Return value
    ///
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

    /// Refreshes the available operations that libgit2 can see.
    pub fn refresh_operations(&mut self) {
        Self::set_operations(self.as_inner().supported_operations(), unsafe {
            &mut self.raw.as_mut().parent
        });
    }

    /// Returns a reference to the inner implementation of `OdbBackend`.
    pub fn as_inner(&self) -> &'a B {
        unsafe { &self.raw.as_ref().inner }
    }
    /// Returns a mutable reference to the inner implementation of `OdbBackend`.
    pub fn as_inner_mut(&mut self) -> &'a mut B {
        unsafe { &mut self.raw.as_mut().inner }
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
        op_if!(read_prefix if READ_PREFIX);
        op_if!(read_header if READ_HEADER);
        op_if!(exists if EXISTS);
        op_if!(exists_prefix if EXISTS_PREFIX);

        backend.free = Some(Backend::<B>::free);
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
    ) -> c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let data = unsafe { data_ptr.as_mut().unwrap() };
        let size = unsafe { size_ptr.as_mut().unwrap() };
        let object_type = unsafe { otype_ptr.as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };

        let context = OdbBackendContext { backend_ptr };

        let mut allocation = ManuallyDrop::new(context.alloc_0());

        let mut object_type2 = ObjectType::Any;

        if let Err(e) = backend
            .inner
            .read(&context, oid, &mut object_type2, &mut allocation)
        {
            ManuallyDrop::into_inner(allocation);
            return e.raw_code();
        }

        *size = allocation.size;
        *data = allocation.raw.as_ptr();
        *object_type = object_type2.raw();

        raw::GIT_OK
    }
    extern "C" fn read_prefix(
        oid_ptr: *mut raw::git_oid,
        data_ptr: *mut *mut c_void,
        size_ptr: *mut size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_prefix_ptr: *const raw::git_oid,
        oid_prefix_len: size_t,
    ) -> c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let data = unsafe { data_ptr.as_mut().unwrap() };
        let size = unsafe { size_ptr.as_mut().unwrap() };
        let object_type = unsafe { otype_ptr.as_mut().unwrap() };
        let oid_prefix = unsafe { Oid::from_raw(oid_prefix_ptr) };
        // This is a small hack because Oid doesn't expose the raw data which we need
        let oid = unsafe { oid_ptr.cast::<Oid>().as_mut().unwrap() };

        let context = OdbBackendContext { backend_ptr };

        let mut allocation = ManuallyDrop::new(context.alloc_0());

        let mut object_type2 = ObjectType::Any;
        let mut oid2 = Oid::zero();

        if let Err(e) = backend.inner.read_prefix(
            &context,
            oid_prefix,
            oid_prefix_len as usize,
            &mut oid2,
            &mut object_type2,
            &mut allocation,
        ) {
            ManuallyDrop::into_inner(allocation);
            return e.raw_code();
        }

        *oid = oid2;
        *size = allocation.size;
        *data = allocation.raw.as_ptr();
        *object_type = object_type2.raw();

        raw::GIT_OK
    }
    extern "C" fn read_header(
        size_ptr: *mut size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> c_int {
        let size = unsafe { size_ptr.as_mut().unwrap() };
        let otype = unsafe { otype_ptr.as_mut().unwrap() };
        let backend = unsafe { backend_ptr.cast::<Backend<B>>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };

        let context = OdbBackendContext { backend_ptr };

        let mut object_type = ObjectType::Any;
        if let Err(e) = backend
            .inner
            .read_header(&context, oid, size, &mut object_type)
        {
            unsafe { return e.raw_set_git_error() }
        };
        *otype = object_type.raw();

        raw::GIT_OK
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
    
    extern "C" fn exists_prefix(
        oid_ptr: *mut raw::git_oid,
        backend_ptr: *mut raw::git_odb_backend,
        oid_prefix_ptr: *const raw::git_oid,
        oid_prefix_len: size_t
    ) -> c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let oid_prefix = unsafe { Oid::from_raw(oid_prefix_ptr) };
        let oid = unsafe { oid_ptr.cast::<Oid>().as_mut().unwrap() };

        let context = OdbBackendContext { backend_ptr };
        *oid = match backend.inner.exists_prefix(&context, oid_prefix, oid_prefix_len) {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };
        raw::GIT_OK
    }

    extern "C" fn free(backend: *mut raw::git_odb_backend) {
        let inner = unsafe { Box::from_raw(backend.cast::<Self>()) };
        drop(inner);
    }
}
