use {raw, Error, Oid, Repository};
use libc::{c_int, c_void};

use util::Binding;
use util::IntoCString;

use std::marker;
use std::mem;
use std::path::Path;
use std::ptr;

pub type TransferProgressCb<'a> =
    FnMut(&raw::git_transfer_progress) -> bool + 'a;

/// Indexer
///
pub struct Indexer<'repo> {
    indexer: *mut raw::git_indexer,
    _callback: Option<Box<TransferProgressCb<'repo>>>,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> Indexer<'repo> {
    /// Create a new indexer instance.
    pub fn new(path: &Path, mode: u32) -> Result<Self, Error> {
        let path = try!(path.into_c_string());
        let mut indexer: *mut raw::git_indexer = ptr::null_mut();
        unsafe {
            try_call!(raw::git_indexer_new(
                &mut indexer,
                path,
                mode,
                ptr::null_mut(),
                None,
                ptr::null_mut()
            ));
            Ok(Self {
                indexer: indexer,
                _callback: None,
                _marker: marker::PhantomData,
            })
        }
    }

    /// Create a new indexer instance.
    pub fn new_with_progress<F>(
        path: &Path,
        mode: u32,
        progress: F,
    ) -> Result<Self, Error>
    where
        F: FnMut(&raw::git_transfer_progress) -> bool + 'repo,
    {
        let mut callback_boxed: Box<TransferProgressCb> = Box::new(progress);
        let callback_ptr = &mut callback_boxed as *mut _;

        let path = try!(path.into_c_string());
        let progress_cb =
            Some(transfer_progress_cb as raw::git_transfer_progress_cb);

        let mut indexer: *mut raw::git_indexer = ptr::null_mut();
        unsafe {
            try_call!(raw::git_indexer_new(
                &mut indexer,
                path,
                mode,
                ptr::null_mut(),
                progress_cb,
                callback_ptr as *mut _
            ));
            Ok(Self {
                indexer: indexer,
                _callback: Some(callback_boxed),
                _marker: marker::PhantomData,
            })
        }
    }

    /// Add data to the indexer
    pub fn append(
        &mut self,
        data: &[u8],
        stats: Option<&mut raw::git_transfer_progress>,
    ) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_indexer_append(
                self.indexer,
                data.as_ptr() as *const _,
                data.len(),
                stats
            ));
        }
        Ok(())
    }

    /// Finalize the pack and index.
    ///
    /// Resolve any pending deltas and write out the index file.
    pub fn commit(
        &mut self,
        stats: Option<&mut raw::git_transfer_progress>,
    ) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_indexer_commit(self.indexer, stats));
        }
        Ok(())
    }

    /// Get the packfile's hash
    ///
    /// A packfile's name is derived from the sorted hashing of all object names.
    /// This is only correct after the index has been finalized.
    pub fn hash(&mut self) -> Oid {
        unsafe { Binding::from_raw(raw::git_indexer_hash(self.indexer)) }
    }
}

impl<'repo> Drop for Indexer<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_indexer_free(self.indexer) }
    }
}

extern "C" fn transfer_progress_cb(
    progress: *const raw::git_transfer_progress,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let data = data as *mut Box<TransferProgressCb>;
        let param: &raw::git_transfer_progress = mem::transmute(progress);
        (*data)(param) as c_int
    }
}
