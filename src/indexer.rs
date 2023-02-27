use std::ffi::CStr;
use std::path::Path;
use std::{io, marker, mem, ptr};

use libc::c_void;

use crate::odb::{write_pack_progress_cb, OdbPackwriterCb};
use crate::util::Binding;
use crate::{raw, Error, IntoCString, Odb};

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

/// A stream to write and index a packfile
///
/// This is equivalent to [`crate::OdbPackwriter`], but allows to store the pack
/// and index at an arbitrary path. It also does not require access to an object
/// database if, and only if, the pack file is self-contained (i.e. not "thin").
pub struct Indexer<'odb> {
    raw: *mut raw::git_indexer,
    progress: raw::git_indexer_progress,
    progress_payload_ptr: *mut OdbPackwriterCb<'odb>,
}

impl<'a> Indexer<'a> {
    /// Create a new indexer
    ///
    /// The [`Odb`] is used to resolve base objects when fixing thin packs. It
    /// can be `None` if no thin pack is expected, in which case missing bases
    /// will result in an error.
    ///
    /// `mode` is the permissions to use for the output files, use `0` for defaults.
    ///
    /// If `verify` is `false`, the indexer will bypass object connectivity checks.
    pub fn new(odb: Option<&Odb<'a>>, path: &Path, mode: u32, verify: bool) -> Result<Self, Error> {
        let path = path.into_c_string()?;

        let odb = odb.map(Binding::raw).unwrap_or_else(ptr::null_mut);

        let mut out = ptr::null_mut();
        let progress_cb: raw::git_indexer_progress_cb = Some(write_pack_progress_cb);
        let progress_payload = Box::new(OdbPackwriterCb { cb: None });
        let progress_payload_ptr = Box::into_raw(progress_payload);

        unsafe {
            let mut opts = mem::zeroed();
            try_call!(raw::git_indexer_options_init(
                &mut opts,
                raw::GIT_INDEXER_OPTIONS_VERSION
            ));
            opts.progress_cb = progress_cb;
            opts.progress_cb_payload = progress_payload_ptr as *mut c_void;
            opts.verify = verify.into();

            try_call!(raw::git_indexer_new(&mut out, path, mode, odb, &mut opts));
        }

        Ok(Self {
            raw: out,
            progress: Default::default(),
            progress_payload_ptr,
        })
    }

    /// Finalize the pack and index
    ///
    /// Resolves any pending deltas and writes out the index file. The returned
    /// string is the hexadecimal checksum of the packfile, which is also used
    /// to name the pack and index files (`pack-<checksum>.pack` and
    /// `pack-<checksum>.idx` respectively).
    pub fn commit(mut self) -> Result<String, Error> {
        unsafe {
            try_call!(raw::git_indexer_commit(self.raw, &mut self.progress));

            let name = CStr::from_ptr(raw::git_indexer_name(self.raw));
            Ok(name.to_str().expect("pack name not utf8").to_owned())
        }
    }

    /// The callback through which progress is monitored. Be aware that this is
    /// called inline, so performance may be affected.
    pub fn progress<F>(&mut self, cb: F) -> &mut Self
    where
        F: FnMut(Progress<'_>) -> bool + 'a,
    {
        let progress_payload =
            unsafe { &mut *(self.progress_payload_ptr as *mut OdbPackwriterCb<'_>) };
        progress_payload.cb = Some(Box::new(cb) as Box<IndexerProgress<'a>>);

        self
    }
}

impl io::Write for Indexer<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        unsafe {
            let ptr = buf.as_ptr() as *mut c_void;
            let len = buf.len();

            let res = raw::git_indexer_append(self.raw, ptr, len, &mut self.progress);
            if res < 0 {
                Err(io::Error::new(
                    io::ErrorKind::Other,
                    Error::last_error(res).unwrap(),
                ))
            } else {
                Ok(buf.len())
            }
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Drop for Indexer<'_> {
    fn drop(&mut self) {
        unsafe {
            raw::git_indexer_free(self.raw);
            drop(Box::from_raw(self.progress_payload_ptr))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{Buf, Indexer};
    use std::io::prelude::*;

    #[test]
    fn indexer() {
        let (_td, repo_source) = crate::test::repo_init();
        let (_td, repo_target) = crate::test::repo_init();

        let mut progress_called = false;

        // Create an in-memory packfile
        let mut builder = t!(repo_source.packbuilder());
        let mut buf = Buf::new();
        let (commit_source_id, _tree) = crate::test::commit(&repo_source);
        t!(builder.insert_object(commit_source_id, None));
        t!(builder.write_buf(&mut buf));

        // Write it to the standard location in the target repo, but via indexer
        let odb = repo_source.odb().unwrap();
        let mut indexer = Indexer::new(
            Some(&odb),
            repo_target.path().join("objects").join("pack").as_path(),
            0o644,
            true,
        )
        .unwrap();
        indexer.progress(|_| {
            progress_called = true;
            true
        });
        indexer.write(&buf).unwrap();
        indexer.commit().unwrap();

        // Assert that target repo picks it up as valid
        let commit_target = repo_target.find_commit(commit_source_id).unwrap();
        assert_eq!(commit_target.id(), commit_source_id);
        assert!(progress_called);
    }
}
