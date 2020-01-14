use std::marker;

use crate::raw;
use crate::util::Binding;

/// Struct representing the progress by an in-flight transfer.
pub struct Progress<'a> {
    pub(crate) raw: ProgressState,
    pub(crate) _marker: marker::PhantomData<&'a raw::git_indexer_progress>,
}

pub(crate) enum ProgressState {
    Borrowed(*const raw::git_indexer_progress),
    Owned(raw::git_indexer_progress),
}

/// Callback to be invoked while indexing is in progress.
///
/// This callback will be periodically called with updates to the progress of
/// the indexing so far. The return value indicates whether the indexing or
/// transfer should continue. A return value of `false` will cancel the
/// indexing or transfer.
///
/// * `progress` - the progress being made so far.
pub type IndexerProgress<'a> = dyn FnMut(Progress<'_>) -> bool + 'a;

impl<'a> Progress<'a> {
    /// Number of objects in the packfile being downloaded
    pub fn total_objects(&self) -> usize {
        unsafe { (*self.raw()).total_objects as usize }
    }
    /// Received objects that have been hashed
    pub fn indexed_objects(&self) -> usize {
        unsafe { (*self.raw()).indexed_objects as usize }
    }
    /// Objects which have been downloaded
    pub fn received_objects(&self) -> usize {
        unsafe { (*self.raw()).received_objects as usize }
    }
    /// Locally-available objects that have been injected in order to fix a thin
    /// pack.
    pub fn local_objects(&self) -> usize {
        unsafe { (*self.raw()).local_objects as usize }
    }
    /// Number of deltas in the packfile being downloaded
    pub fn total_deltas(&self) -> usize {
        unsafe { (*self.raw()).total_deltas as usize }
    }
    /// Received deltas that have been hashed.
    pub fn indexed_deltas(&self) -> usize {
        unsafe { (*self.raw()).indexed_deltas as usize }
    }
    /// Size of the packfile received up to now
    pub fn received_bytes(&self) -> usize {
        unsafe { (*self.raw()).received_bytes as usize }
    }

    /// Convert this to an owned version of `Progress`.
    pub fn to_owned(&self) -> Progress<'static> {
        Progress {
            raw: ProgressState::Owned(unsafe { *self.raw() }),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a> Binding for Progress<'a> {
    type Raw = *const raw::git_indexer_progress;
    unsafe fn from_raw(raw: *const raw::git_indexer_progress) -> Progress<'a> {
        Progress {
            raw: ProgressState::Borrowed(raw),
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *const raw::git_indexer_progress {
        match self.raw {
            ProgressState::Borrowed(raw) => raw,
            ProgressState::Owned(ref raw) => raw as *const _,
        }
    }
}

/// Callback to be invoked while a transfer is in progress.
///
/// This callback will be periodically called with updates to the progress of
/// the transfer so far. The return value indicates whether the transfer should
/// continue. A return value of `false` will cancel the transfer.
///
/// * `progress` - the progress being made so far.
#[deprecated(
    since = "0.11.0",
    note = "renamed to `IndexerProgress` to match upstream"
)]
#[allow(dead_code)]
pub type TransportProgress<'a> = IndexerProgress<'a>;
