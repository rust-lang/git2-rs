use {raw, Error, Oid, Repository};
use libc::{c_int, c_void};

use util::Binding;
use util::IntoCString;

use std::marker;
use std::mem;
use std::path::Path;
use std::ptr;

pub type TransferProgressCb = FnMut(&raw::git_transfer_progress) -> bool;

pub struct Indexer<'repo> {
    indexer: *mut raw::git_indexer,
    callback: Option<Box<TransferProgressCb>>,
    _marker: marker::PhantomData<&'repo Repository>,
}

impl<'repo> Indexer<'repo> {
    pub fn new<F>(
        repo: &Repository,
        path: &Path,
        mode: u32,
        progress: Option<F>,
    ) -> Result<Self, Error>
    where
        F: FnMut(&raw::git_transfer_progress) -> bool,
    {
        let mut callback_boxed: Option<Box<TransferProgressCb>> = None;
        let callback_ptr = if let Some(callback) = progress {
            let mut boxed = Box::new(callback);
            let ptr = &mut boxed as *mut _;
            callback_boxed = Some(boxed);
            ptr
        } else {
            ptr::null_mut()
        };

        let path = try!(path.into_c_string());
        let progress_c: Option<raw::git_transfer_progress> =
            if let Some(_) = callback_boxed {
                Some(mem::transmute(transfer_progress_cb))
            } else {
                None
            };

        let mut indexer: *mut raw::git_indexer;
        unsafe {
            try_call!(raw::git_indexer_new(
                &mut indexer,
                path,
                mode,
                ptr::null_mut(),
                progress_c,
                callback_ptr as *mut _
            ));
        }

        Ok(Self {
            indexer: indexer,
            callback: callback_boxed,
            _marker: marker::PhantomData,
        })
    }

    pub fn append(
        &mut self,
        data: &[u8],
        stats: &mut raw::git_transfer_progress,
    ) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_indexer_append(
                self.indexer,
                data.as_ptr(),
                data.len(),
                stats
            ))
        }
    }

    pub fn commit(
        &mut self,
        stats: &mut raw::git_transfer_progress,
    ) -> Result<(), Error> {
        unsafe { try_call!(raw::git_indexer_commit(self.indexer, stats)) }
    }

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
