//! Custom backends for [`Odb`]s.
//!
//! TODO: Merge a lot of these APIs with existing ones.
//!       Currently
use crate::util::Binding;
use crate::{raw, Error, ErrorClass, ErrorCode, IntoCString, ObjectType, Odb, Oid};
use bitflags::bitflags;
use std::convert::Infallible;
use std::mem::ManuallyDrop;
use std::path::Path;
use std::{marker, mem, ptr, slice};

/// A custom implementation of an [`Odb`] backend.
///
/// Most of the default implementations of this trait's methods panic when called as they are
/// intended to be overridden.
///
/// # Error recommendations
///
/// Errors are generally left at the implementation's discretion; some recommendations are
/// made regarding error codes and classes to ease implementation and usage of custom backends.
///
/// Read the individual methods' documentation for more specific recommendations.
///
/// If the backend does not have enough memory, the error SHOULD be code
/// [`ErrorCode::GenericError`] and the class SHOULD be [`ErrorClass::NoMemory`].
#[allow(unused_variables)]
pub trait OdbBackend: Sized {
    /// Backend-specific writepack implementation.
    ///
    /// If the backend doesn't support writepack, this type should be [`Infallible`].
    ///
    /// [`Infallible`]: std::convert::Infallible
    type Writepack: OdbWritepack<Self>;
    /// Backend-specific readable stream.
    ///
    /// If the backend doesn't support reading through streams, this type should be [`Infallible`].
    type ReadStream: OdbReadStream<Self>;
    /// Backend-specific writable stream.
    ///
    /// If the backend doesn't support writing through streams, this type should be [`Infallible`].
    type WriteStream: OdbWriteStream<Self>;

    /// Returns the supported operations of this backend.
    /// The return value is used to determine what functions to provide to libgit2.
    ///
    /// This method is only called once in [`Odb::add_custom_backend`] and once in every call to
    /// [`CustomOdbBackend::refresh_operations`]; in general, it is called very rarely.
    /// Very few implementations should change their available operations after being added to an
    /// [`Odb`].
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

    /// Write an object.
    ///
    /// Corresponds to the `write` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::WRITE`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// `oid` is calculated by libgit2 prior to this method being called.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn write(
        &mut self,
        ctx: &OdbBackendContext,
        oid: Oid,
        object_type: ObjectType,
        data: &[u8],
    ) -> Result<(), Error> {
        unimplemented!("OdbBackend::write")
    }

    /// Check if an object exists.
    ///
    /// Corresponds to the `exists` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::EXISTS`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
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

    /// Refreshes the backend.
    ///
    /// Corresponds to the `refresh` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::REFRESH`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method returns `Ok(())`.
    ///
    /// # Implementation notes
    ///
    /// This method is called automatically when a lookup fails (e.g. through
    /// [`OdbBackend::exists`], [`OdbBackend::read`], or [`OdbBackend::read_header`]),
    /// or when [`Odb::refresh`](crate::Odb::refresh) is invoked.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn refresh(&mut self, ctx: &OdbBackendContext) -> Result<(), Error> {
        Ok(())
    }

    /// "Freshens" an already existing object, updating its last-used time.
    ///
    /// Corresponds to the `freshen` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::FRESHEN`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// This method is called when [`Odb::write`](Odb::write) is called, but the object
    /// already exists and will not be rewritten.
    ///
    /// Implementations may want to update last-used timestamps.
    ///
    /// Implementations SHOULD return `Ok(())` if the object exists and was freshened; otherwise,
    /// they SHOULD return an error.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn freshen(&mut self, ctx: &OdbBackendContext, oid: Oid) -> Result<(), Error> {
        unimplemented!("OdbBackend::freshen")
    }

    /// Opens a stream to write a packfile to this backend.
    ///
    /// Corresponds to the `writepack` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::WRITE_PACK`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// TODO: More information on what this even is.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn open_writepack(
        &mut self,
        ctx: &OdbBackendContext,
        odb: &Odb<'_>,
        callback: IndexerProgressCallback,
    ) -> Result<Self::Writepack, Error> {
        unimplemented!("OdbBackend::open_writepack")
    }

    /// Creates a `multi-pack-index` file containing an index of all objects across all `.pack`
    /// files.
    ///
    /// Corresponds to the `writemidx` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::WRITE_MULTIPACK_INDEX`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// TODO: Implementation notes for `write_multipack_index`
    ///
    /// # Errors
    ///
    /// See [`OdbBackend`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn write_multipack_index(&mut self, ctx: &OdbBackendContext) -> Result<(), Error> {
        unimplemented!("OdbBackend::write_multipack_index")
    }

    /// Opens a stream to read an object.
    ///
    /// Corresponds to the `readstream` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::READSTREAM`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// If an implementation returns `Ok(stream)`, `length` and `object_type` MUST be set to the
    /// length of the object's contents and the object type respectively; see
    /// [`OdbBackend::read_header`].
    ///
    /// # Errors
    ///
    /// See [`OdbBackend::read`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn open_read_stream(
        &mut self,
        ctx: &OdbBackendContext,
        oid: Oid,
        length: &mut usize,
        object_type: &mut ObjectType,
    ) -> Result<Self::ReadStream, Error> {
        unimplemented!("OdbBackend::open_read_stream")
    }
    /// Opens a stream to write an object.
    ///
    /// Corresponds to the `writestream` function of [`git_odb_backend`].
    /// Requires that [`SupportedOperations::WRITESTREAM`] is present in the value returned from
    /// [`supported_operations`] to expose it to libgit2.
    ///
    /// The default implementation of this method panics.
    ///
    /// # Implementation notes
    ///
    /// The Oid of the object is calculated by libgit2 *after* all the data has been written.
    ///
    /// # Errors
    ///
    /// See [`OdbBackend::write`].
    ///
    /// [`git_odb_backend`]: raw::git_odb_backend
    /// [`supported_operations`]: Self::supported_operations
    fn open_write_stream(
        &mut self,
        ctx: &OdbBackendContext,
        length: usize,
        object_type: ObjectType,
    ) -> Result<Self::WriteStream, Error> {
        unimplemented!("OdbBackend::open_write_stream")
    }

    // TODO: fn foreach()
}

bitflags! {
    /// Supported operations for a backend.
    pub struct SupportedOperations: u32 {
        // NOTE: The names are mostly taken from the trait method names, but the order of the flags
        //       is taken from the fields of git_odb_backend.
        //       Essentially, choose a name that is tasteful.
        /// The backend supports the [`OdbBackend::read`] method.
        const READ = 1;
        /// The backend supports the [`OdbBackend::read_prefix`] method.
        const READ_PREFIX = 1 << 1;
        /// The backend supports the [`OdbBackend::read_header`] method.
        const READ_HEADER = 1 << 2;
        /// The backend supports the [`OdbBackend::write`] method.
        const WRITE = 1 << 3;
        /// The backend supports the [`OdbBackend::open_write_stream`] method.
        const WRITESTREAM = 1 << 4;
        /// The backend supports the [`OdbBackend::open_read_stream`] method.
        const READSTREAM = 1 << 5;
        /// The backend supports the [`OdbBackend::exists`] method.
        const EXISTS = 1 << 6;
        /// The backend supports the [`OdbBackend::exists_prefix`] method.
        const EXISTS_PREFIX = 1 << 7;
        /// The backend supports the [`OdbBackend::refresh`] method.
        const REFRESH = 1 << 7;
        /// The backend supports the [`OdbBackend::foreach`] method.
        const FOREACH = 1 << 8;
        /// The backend supports the [`OdbBackend::open_writepack`] method.
        const WRITE_PACK = 1 << 9;
        /// The backend supports the [`OdbBackend::write_multipack_index`] method.
        const WRITE_MULTIPACK_INDEX = 1 << 10;
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
    raw: ptr::NonNull<libc::c_void>,
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
            raw: ptr::NonNull::dangling(),
            size: 0,
        }
    }

    /// Attempts to allocate a buffer of size `size`.
    ///
    /// # Return value
    /// `Some(allocation)` if the allocation succeeded.
    /// `None` otherwise. This usually indicates that there is not enough memory.
    pub fn alloc(&self, size: usize) -> Option<OdbBackendAllocation> {
        let data =
            unsafe { raw::git_odb_backend_data_alloc(self.backend_ptr, size as libc::size_t) };
        let data = ptr::NonNull::new(data)?;
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
/// Indexer progress callback.
pub struct IndexerProgressCallback {
    callback: raw::git_indexer_progress_cb,
    payload: *mut libc::c_void,
}
impl IndexerProgressCallback {
    /// Invokes this callback.
    pub fn invoke(&mut self, progress: &IndexerProgress) -> Result<(), Error> {
        let Some(callback) = self.callback else {
            return Ok(());
        };
        let value = callback(unsafe { progress.raw.as_ref() }, self.payload);
        if value != raw::GIT_OK {
            return Err(Error::last_error(value));
        }
        Ok(())
    }
    /// Creates an [`Indexer`] using this callback. Compare with [`crate::indexer::Indexer`].
    pub fn into_indexer(self, odb: &Odb<'_>, path: &Path, verify: bool) -> Result<Indexer, Error> {
        let mut opts = unsafe { mem::zeroed::<raw::git_indexer_options>() };
        unsafe {
            try_call!(raw::git_indexer_options_init(
                &mut opts,
                raw::GIT_INDEXER_OPTIONS_VERSION
            ));
        }
        opts.progress_cb = self.callback;
        opts.progress_cb_payload = self.payload;
        opts.verify = verify.into();
        let mut indexer: *mut raw::git_indexer = ptr::null_mut();
        let path = path.into_c_string()?;
        unsafe {
            try_call!(raw::git_indexer_new(
                &mut indexer,
                path.as_ptr(),
                0,
                odb.raw(),
                &mut opts
            ));
        }

        Ok(Indexer {
            raw: ptr::NonNull::new(indexer).unwrap(),
        })
    }
}

/// Indexer that stores packfiles at an arbitrary path. See [`crate::indexer::Indexer`].
///
/// TODO: Merge this type with aforementioned [`crate::indexer::Indexer`]?
///       They do essentially the same thing, except that the older one allows setting a callback.
///       It's probably better to merge the two into one base type and then have the elder become a
///       wrapper around the base type similar to [`CustomOdbBackend`] that allows setting the
///       callback.
pub struct Indexer {
    raw: ptr::NonNull<raw::git_indexer>,
}
impl Indexer {
    /// Appends data to this indexer.
    pub fn append(&mut self, data: &[u8], stats: &IndexerProgress) -> Result<usize, Error> {
        let result = unsafe {
            raw::git_indexer_append(
                self.raw.as_ptr(),
                data.as_ptr().cast(),
                data.len(),
                stats.raw().as_ptr(),
            )
        };

        if result < 0 {
            Err(Error::last_error(result))
        } else {
            Ok(result as usize)
        }
    }
    /// Commit the packfile to disk.
    pub fn commit(&mut self, stats: &IndexerProgress) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_indexer_commit(
                self.raw.as_ptr(),
                stats.raw().as_ptr()
            ));
        }
        Ok(())
    }
}
impl Drop for Indexer {
    fn drop(&mut self) {
        unsafe { raw::git_indexer_free(self.raw.as_ptr()) }
    }
}

/// Implementation of the [`git_odb_writepack`] interface. See [`OdbBackend`].
///
/// This is the backend equivalent of [`OdbPackwriter`].
///
/// TODO: More documentation regarding what Writepack even is.
///       libgit2 is an enigma slowly peeled back in lines.
///
/// [`git_odb_writepack`]: raw::git_odb_writepack
/// [`OdbPackwriter`]: crate::odb::OdbPackwriter
pub trait OdbWritepack<B: OdbBackend<Writepack = Self>> {
    /// Append data to this stream.
    ///
    /// Corresponds to the `append` function of [`git_odb_writepack`].
    /// See [`OdbBackend::open_writepack`] for more information.
    /// See also [`OdbPackwriter`] (this method corresponds to its [`io::Write`] implementation).
    ///
    /// # Implementation notes
    ///
    /// TODO: Implementation notes
    ///
    /// [`git_odb_writepack`]: raw::git_odb_writepack
    /// [`OdbPackwriter`]: crate::odb::OdbPackwriter
    /// [`io::Write`]: std::io::Write
    fn append(
        &mut self,
        context: &mut OdbWritepackContext<B>,
        data: &[u8],
        stats: &mut IndexerProgress,
    ) -> Result<(), Error>;
    /// Finish writing this packfile.
    ///
    /// Corresponds to the `commit` function of [`git_odb_writepack`].
    /// See [`OdbBackend::open_writepack`] for more information.
    /// See also [`OdbPackwriter`] (this method corresponds to [`OdbPackwriter::commit`]).
    ///
    /// [`git_odb_writepack`]: raw::git_odb_writepack
    /// [`OdbPackwriter`]: crate::odb::OdbPackwriter
    /// [`OdbPackwriter::commit`]: crate::odb::OdbPackwriter::commit
    fn commit(
        &mut self,
        context: &mut OdbWritepackContext<B>,
        stats: &mut IndexerProgress,
    ) -> Result<(), Error>;
}

impl<B: OdbBackend<Writepack = Self>> OdbWritepack<B> for Infallible {
    fn append(
        &mut self,
        _context: &mut OdbWritepackContext<B>,
        _data: &[u8],
        _stats: &mut IndexerProgress,
    ) -> Result<(), Error> {
        unreachable!()
    }

    fn commit(
        &mut self,
        _context: &mut OdbWritepackContext<B>,
        _stats: &mut IndexerProgress,
    ) -> Result<(), Error> {
        unreachable!()
    }
}
/// Context struct passed to [`OdbWritepack`]'s methods.
///
/// This type allows access to the associated [`OdbBackend`].
pub struct OdbWritepackContext<B: OdbBackend> {
    backend_ptr: ptr::NonNull<Backend<B>>,
}
impl<B: OdbBackend> OdbWritepackContext<B> {
    /// Get a reference to the associated [`OdbBackend`].
    pub fn backend(&self) -> &B {
        unsafe { &self.backend_ptr.as_ref().inner }
    }
    /// Get a mutable reference to the associated [`OdbBackend`].
    pub fn backend_mut(&mut self) -> &mut B {
        unsafe { &mut self.backend_ptr.as_mut().inner }
    }
}

/// For tracking statistics in [`OdbWritepack`] implementations.
///
/// This type is essentially just a mutable version of the [`Progress`] type; in fact, their only
/// difference is that [`IndexerProgress`] contains a [`NonNull<git_indexer_progress>`] whereas
/// [`Progress`] is either an owned value or a pointer to [`git_indexer_progress`].
///
/// [`Progress`]: crate::Progress
/// [`git_indexer_progress`]: raw::git_indexer_progress
/// [`NonNull<git_indexer_progress>`]: ptr::NonNull
pub struct IndexerProgress {
    raw: ptr::NonNull<raw::git_indexer_progress>,
}

macro_rules! define_stats {
    (
        $(
        $field_name:ident: $set_fn:ident, $as_mut_fn:ident, $preferred_type:ident, $native_type:ident
        ),*
    ) => {
        impl IndexerProgress {
            $(
            #[doc = "Gets the `"]
            #[doc = stringify!($field_name)]
            #[doc = "` field's value from the underlying"]
            #[doc = "[`git_indexer_progress`](raw::git_indexer_progress) struct"]
            pub fn $field_name(&self) -> $preferred_type {
                unsafe { self.raw.as_ref() }.$field_name
            }
            #[doc = "Sets the `"]
            #[doc = stringify!($field_name)]
            #[doc = "` field's value from the underlying"]
            #[doc = "[`git_indexer_progress`](raw::git_indexer_progress) struct"]
            #[doc = ""]
            #[doc = "# Panics"]
            #[doc = ""]
            #[doc = "This method may panic if the value cannot be casted to the native type, which"]
            #[doc = "is only the case for platforms where [`libc::"]
            #[doc = stringify!($native_type)]
            #[doc = "`] is smaller than [`"]
            #[doc = stringify!($preferred_type)]
            #[doc = "`]."]
            pub fn $set_fn(&mut self, value: $preferred_type) {
                unsafe { self.raw.as_mut() }.$field_name = value as libc::$native_type;
            }
            #[doc = "Return a mutable reference to the underlying `"]
            #[doc = stringify!($field_name)]
            #[doc = "` field of the [`git_indexer_progress`](raw::git_indexer_progress) struct"]
            pub fn $as_mut_fn(&mut self) -> &mut libc::$native_type {
                &mut unsafe { self.raw.as_mut() }.$field_name
            }
            )*
        }
    };
}
define_stats!(
    total_objects:    set_total_objects,    total_objects_mut,    u32,   c_uint,
    indexed_objects:  set_indexed_objects,  indexed_objects_mut,  u32,   c_uint,
    received_objects: set_received_objects, received_objects_mut, u32,   c_uint,
    local_objects:    set_local_objects,    local_objects_mut,    u32,   c_uint,
    total_deltas:     set_total_deltas,     total_deltas_mut,     u32,   c_uint,
    indexed_deltas:   set_indexed_deltas,   indexed_deltas_mut,   u32,   c_uint,
    received_bytes:   set_received_bytes,   received_bytes_mut,   usize, size_t
);

impl Binding for IndexerProgress {
    type Raw = ptr::NonNull<raw::git_indexer_progress>;

    unsafe fn from_raw(raw: Self::Raw) -> Self {
        Self { raw }
    }

    fn raw(&self) -> Self::Raw {
        self.raw
    }
}

/// A stream that can be read from.
pub trait OdbReadStream<B: OdbBackend> {
    /// Read as many bytes as possible from this stream, returning how many bytes were read.
    ///
    /// Corresponds to the `read` function of [`git_odb_stream`].
    ///
    /// # Implementation notes
    ///
    /// If `Ok(read_bytes)` is returned, `read_bytes` should be how many bytes were read from this
    /// stream. This number must not exceed the length of `out`.
    ///
    /// `out` will never have a length greater than [`libc::c_int::MAX`].
    ///
    /// > Whilst a caller may be able to pass buffers longer than that, `read_bytes` (from the `Ok`
    /// > return value) must be convertible to a [`libc::c_int`] for git2 to be able to return the
    /// > value back to libgit2.
    /// > For that reason, git2 will automatically limit the buffer length to [`libc::c_int::MAX`].
    ///
    /// # Errors
    ///
    /// See [`OdbBackend`].
    ///
    /// [`git_odb_stream`]: raw::git_odb_stream
    fn read(&mut self, ctx: &mut OdbStreamContext<B>, out: &mut [u8]) -> Result<usize, Error>;
}

impl<B: OdbBackend> OdbReadStream<B> for Infallible {
    fn read(&mut self, _ctx: &mut OdbStreamContext<B>, _out: &mut [u8]) -> Result<usize, Error> {
        unreachable!()
    }
}

/// A stream that can be written to.
pub trait OdbWriteStream<B: OdbBackend> {
    /// Write bytes to this stream.
    ///
    /// Corresponds to the `write` function of [`git_odb_stream`].
    ///
    /// # Implementation notes
    ///
    /// All calls to `write` will be "finalized" by a single call to [`finalize_write`], after which
    /// no more calls to this stream will occur.
    ///
    /// [`git_odb_stream`]: raw::git_odb_stream
    /// [`finalize_write`]: OdbWriteStream::finalize_write
    fn write(&mut self, ctx: &mut OdbStreamContext<B>, data: &[u8]) -> Result<(), Error>;
    /// Store the contents of the stream as an object with the specified [`Oid`].
    ///
    /// Corresponds to the `finalize_write` function of [`git_odb_stream`].
    ///
    /// # Implementation notes
    ///
    /// This method might not be invoked if:
    /// - an error occurs in the [`write`] implementation,
    /// - `oid` refers to an already existing object in another backend, or
    /// - the final number of received bytes differs from the size declared when the stream was opened.
    ///
    ///
    /// [`git_odb_stream`]: raw::git_odb_stream
    /// [`write`]: OdbWriteStream::write
    fn finalize_write(&mut self, ctx: &mut OdbStreamContext<B>, oid: Oid) -> Result<(), Error>;
}

impl<B: OdbBackend> OdbWriteStream<B> for Infallible {
    fn write(&mut self, _ctx: &mut OdbStreamContext<B>, _data: &[u8]) -> Result<(), Error> {
        unreachable!()
    }

    fn finalize_write(&mut self, _ctx: &mut OdbStreamContext<B>, _oid: Oid) -> Result<(), Error> {
        unreachable!()
    }
}

/// Context struct passed to [`OdbReadStream`] and [`OdbWriteStream`]'s methods.
pub struct OdbStreamContext<B: OdbBackend> {
    backend_ptr: ptr::NonNull<Backend<B>>,
}
impl<B: OdbBackend> OdbStreamContext<B> {
    /// Get a reference to the associated [`OdbBackend`].
    pub fn backend(&self) -> &B {
        unsafe { &self.backend_ptr.as_ref().inner }
    }
    /// Get a mutable reference to the associated [`OdbBackend`].
    pub fn backend_mut(&mut self) -> &mut B {
        unsafe { &mut self.backend_ptr.as_mut().inner }
    }
}

/// A handle to an [`OdbBackend`] that has been added to an [`Odb`].
pub struct CustomOdbBackend<'a, B: OdbBackend> {
    // NOTE: Any pointer in this field must be both non-null and properly aligned.
    raw: ptr::NonNull<Backend<B>>,
    phantom: marker::PhantomData<fn() -> &'a ()>,
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
        let backend = unsafe { ptr::NonNull::new_unchecked(backend) };
        Self {
            raw: backend,
            phantom: marker::PhantomData,
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
        op_if!(write if WRITE);
        op_if!(writestream if WRITESTREAM);
        op_if!(readstream if READSTREAM);
        op_if!(exists if EXISTS);
        op_if!(exists_prefix if EXISTS_PREFIX);
        op_if!(refresh if REFRESH);
        op_if!(writepack if WRITE_PACK);
        op_if!(writemidx if WRITE_MULTIPACK_INDEX);
        op_if!(freshen if FRESHEN);

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
        data_ptr: *mut *mut libc::c_void,
        size_ptr: *mut libc::size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> libc::c_int {
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
        data_ptr: *mut *mut libc::c_void,
        size_ptr: *mut libc::size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_prefix_ptr: *const raw::git_oid,
        oid_prefix_len: libc::size_t,
    ) -> libc::c_int {
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
        size_ptr: *mut libc::size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> libc::c_int {
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
            return unsafe { e.raw_set_git_error() };
        };
        *otype = object_type.raw();

        raw::GIT_OK
    }

    extern "C" fn write(
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
        data_ptr: *const libc::c_void,
        len: usize,
        otype: raw::git_object_t,
    ) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Backend<B>>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };
        let data = unsafe { slice::from_raw_parts(data_ptr.cast::<u8>(), len) };
        let object_type = ObjectType::from_raw(otype).unwrap();
        let context = OdbBackendContext { backend_ptr };
        if let Err(e) = backend.inner.write(&context, oid, object_type, data) {
            return unsafe { e.raw_set_git_error() };
        }
        raw::GIT_OK
    }
    extern "C" fn writestream(
        stream_out: *mut *mut raw::git_odb_stream,
        backend_ptr: *mut raw::git_odb_backend,
        length: raw::git_object_size_t,
        object_type: raw::git_object_t,
    ) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Backend<B>>().as_mut().unwrap() };
        let object_type = ObjectType::from_raw(object_type).unwrap();
        let context = OdbBackendContext { backend_ptr };
        let stream_out = unsafe { stream_out.as_mut().unwrap() };
        let stream = match backend
            .inner
            .open_write_stream(&context, length as usize, object_type)
        {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };

        let stream = WriteStream::<B> {
            parent: raw::git_odb_stream {
                backend: backend_ptr,
                mode: raw::GIT_STREAM_WRONLY as _,
                hash_ctx: ptr::null_mut(),
                declared_size: 0,
                received_bytes: 0,
                read: None,
                write: Some(WriteStream::<B>::write),
                finalize_write: Some(WriteStream::<B>::finalize_write),
                free: Some(WriteStream::<B>::free),
            },
            _marker: marker::PhantomData,
            inner: stream,
        };

        *stream_out = unsafe { box_allocate(stream).cast() };

        raw::GIT_OK
    }
    extern "C" fn readstream(
        stream_out: *mut *mut raw::git_odb_stream,
        length_ptr: *mut libc::size_t,
        otype_ptr: *mut raw::git_object_t,
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> libc::c_int {
        let size = unsafe { length_ptr.as_mut().unwrap() };
        let otype = unsafe { otype_ptr.as_mut().unwrap() };
        let backend = unsafe { backend_ptr.cast::<Backend<B>>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };
        let stream_out = unsafe { stream_out.as_mut().unwrap() };

        let context = OdbBackendContext { backend_ptr };

        let mut object_type = ObjectType::Any;
        let stream = match backend
            .inner
            .open_read_stream(&context, oid, size, &mut object_type)
        {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };

        *otype = object_type.raw();

        let stream = ReadStream::<B> {
            parent: raw::git_odb_stream {
                backend: backend_ptr,
                mode: raw::GIT_STREAM_RDONLY as _,
                hash_ctx: ptr::null_mut(),
                declared_size: 0,
                received_bytes: 0,
                read: Some(ReadStream::<B>::read),
                write: None,
                finalize_write: None,
                free: Some(ReadStream::<B>::free),
            },
            _marker: marker::PhantomData,
            inner: stream,
        };

        *stream_out = unsafe { box_allocate(stream).cast() };

        raw::GIT_OK
    }

    extern "C" fn exists(
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> libc::c_int {
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
        oid_prefix_len: libc::size_t,
    ) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let oid_prefix = unsafe { Oid::from_raw(oid_prefix_ptr) };
        let oid = unsafe { oid_ptr.cast::<Oid>().as_mut().unwrap() };

        let context = OdbBackendContext { backend_ptr };
        *oid = match backend
            .inner
            .exists_prefix(&context, oid_prefix, oid_prefix_len)
        {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };
        raw::GIT_OK
    }

    extern "C" fn refresh(backend_ptr: *mut raw::git_odb_backend) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let context = OdbBackendContext { backend_ptr };
        if let Err(e) = backend.inner.refresh(&context) {
            return unsafe { e.raw_set_git_error() };
        }
        raw::GIT_OK
    }

    extern "C" fn writepack(
        out_writepack_ptr: *mut *mut raw::git_odb_writepack,
        backend_ptr: *mut raw::git_odb_backend,
        odb_ptr: *mut raw::git_odb,
        progress_cb: raw::git_indexer_progress_cb,
        progress_payload: *mut libc::c_void,
    ) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let context = OdbBackendContext { backend_ptr };

        let odb = unsafe { Odb::from_raw(odb_ptr) };
        let callback = IndexerProgressCallback {
            callback: progress_cb,
            payload: progress_payload,
        };

        let writepack = match backend.inner.open_writepack(&context, &odb, callback) {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };

        let writepack = Writepack::<B> {
            writepack: raw::git_odb_writepack {
                backend: backend_ptr,
                append: Some(Writepack::<B>::append),
                commit: Some(Writepack::<B>::commit),
                free: Some(Writepack::<B>::free),
            },
            inner: writepack,
        };

        let out_writepack = unsafe { out_writepack_ptr.as_mut().unwrap() };
        *out_writepack = unsafe { box_allocate(writepack).cast() };

        raw::GIT_OK
    }

    extern "C" fn writemidx(backend_ptr: *mut raw::git_odb_backend) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let context = OdbBackendContext { backend_ptr };
        if let Err(e) = backend.inner.write_multipack_index(&context) {
            return unsafe { e.raw_set_git_error() };
        }
        raw::GIT_OK
    }

    extern "C" fn freshen(
        backend_ptr: *mut raw::git_odb_backend,
        oid_ptr: *const raw::git_oid,
    ) -> libc::c_int {
        let backend = unsafe { backend_ptr.cast::<Self>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };
        let context = OdbBackendContext { backend_ptr };
        if let Err(e) = backend.inner.freshen(&context, oid) {
            return unsafe { e.raw_set_git_error() };
        }

        raw::GIT_OK
    }

    extern "C" fn free(backend: *mut raw::git_odb_backend) {
        unsafe { box_free(backend.cast::<Self>()) }
    }
}

#[repr(C)]
struct Writepack<B>
where
    B: OdbBackend,
{
    writepack: raw::git_odb_writepack,
    inner: B::Writepack,
}

impl<B> Writepack<B>
where
    B: OdbBackend,
{
    extern "C" fn append(
        writepack_ptr: *mut raw::git_odb_writepack,
        data_ptr: *const libc::c_void,
        data_len: libc::size_t,
        progress_ptr: *mut raw::git_indexer_progress,
    ) -> libc::c_int {
        let writepack_ptr = unsafe { ptr::NonNull::new_unchecked(writepack_ptr) };
        let data = unsafe { slice::from_raw_parts(data_ptr.cast::<u8>(), data_len) };
        let mut progress =
            unsafe { IndexerProgress::from_raw(ptr::NonNull::new_unchecked(progress_ptr)) };
        let writepack = unsafe { writepack_ptr.cast::<Self>().as_mut() };

        let mut context = OdbWritepackContext {
            backend_ptr: unsafe { ptr::NonNull::new_unchecked(writepack.writepack.backend) }.cast(),
        };

        if let Err(e) = writepack.inner.append(&mut context, data, &mut progress) {
            return unsafe { e.raw_set_git_error() };
        }

        raw::GIT_OK
    }
    extern "C" fn commit(
        writepack_ptr: *mut raw::git_odb_writepack,
        progress_ptr: *mut raw::git_indexer_progress,
    ) -> libc::c_int {
        let writepack_ptr = unsafe { ptr::NonNull::new_unchecked(writepack_ptr) };
        let writepack = unsafe { writepack_ptr.cast::<Self>().as_mut() };
        let mut progress =
            unsafe { IndexerProgress::from_raw(ptr::NonNull::new_unchecked(progress_ptr)) };
        let mut context = OdbWritepackContext {
            backend_ptr: unsafe { ptr::NonNull::new_unchecked(writepack.writepack.backend) }.cast(),
        };
        if let Err(e) = writepack.inner.commit(&mut context, &mut progress) {
            return unsafe { e.raw_set_git_error() };
        }
        raw::GIT_OK
    }
    extern "C" fn free(writepack_ptr: *mut raw::git_odb_writepack) {
        unsafe { box_free(writepack_ptr.cast::<Self>()) }
    }
}

struct Stream<B, T> {
    parent: raw::git_odb_stream,
    _marker: marker::PhantomData<B>,
    inner: T,
}
impl<B, T> Stream<B, T> {
    extern "C" fn read(
        stream_ptr: *mut raw::git_odb_stream,
        out_ptr: *mut libc::c_char,
        out_len: libc::size_t,
    ) -> libc::c_int
    where
        B: OdbBackend<ReadStream = T>,
        T: OdbReadStream<B>,
    {
        let stream = unsafe { stream_ptr.cast::<Self>().as_mut().unwrap() };
        let buf_len = (out_len as usize).max(libc::c_int::MAX as usize);
        let buf = unsafe { slice::from_raw_parts_mut(out_ptr.cast::<u8>(), buf_len) };
        let mut context = OdbStreamContext {
            backend_ptr: ptr::NonNull::new(stream.parent.backend).unwrap().cast(),
        };
        let read_bytes = match stream.inner.read(&mut context, buf) {
            Err(e) => return unsafe { e.raw_set_git_error() },
            Ok(x) => x,
        };
        read_bytes as libc::c_int
    }

    extern "C" fn write(
        stream_ptr: *mut raw::git_odb_stream,
        data: *const libc::c_char,
        len: libc::size_t,
    ) -> libc::c_int
    where
        B: OdbBackend<WriteStream = T>,
        T: OdbWriteStream<B>,
    {
        let stream = unsafe { stream_ptr.cast::<Self>().as_mut().unwrap() };
        let data = unsafe { slice::from_raw_parts(data.cast::<u8>(), len) };
        let mut context = OdbStreamContext {
            backend_ptr: ptr::NonNull::new(stream.parent.backend).unwrap().cast(),
        };
        if let Err(e) = stream.inner.write(&mut context, data) {
            return unsafe { e.raw_set_git_error() };
        }

        raw::GIT_OK
    }
    extern "C" fn finalize_write(
        stream_ptr: *mut raw::git_odb_stream,
        oid_ptr: *const raw::git_oid,
    ) -> libc::c_int
    where
        B: OdbBackend<WriteStream = T>,
        T: OdbWriteStream<B>,
    {
        let stream = unsafe { stream_ptr.cast::<Self>().as_mut().unwrap() };
        let oid = unsafe { Oid::from_raw(oid_ptr) };
        let mut context = OdbStreamContext {
            backend_ptr: ptr::NonNull::new(stream.parent.backend).unwrap().cast(),
        };
        if let Err(e) = stream.inner.finalize_write(&mut context, oid) {
            return unsafe { e.raw_set_git_error() };
        }
        raw::GIT_OK
    }

    extern "C" fn free(stream_ptr: *mut raw::git_odb_stream) {
        unsafe { box_free(stream_ptr.cast::<Self>()) }
    }
}
type WriteStream<B> = Stream<B, <B as OdbBackend>::WriteStream>;
type ReadStream<B> = Stream<B, <B as OdbBackend>::ReadStream>;

unsafe fn box_allocate<T>(value: T) -> *mut T {
    Box::into_raw(Box::new(value))
}
unsafe fn box_free<T>(ptr: *mut T) {
    drop(Box::from_raw(ptr))
}
