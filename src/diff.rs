use libc::{c_char, c_int, c_void, size_t};
use std::ffi::CString;
use std::iter::FusedIterator;
use std::marker;
use std::mem;
use std::ops::Range;
use std::path::Path;
use std::ptr;
use std::slice;

use crate::util::{self, Binding};
use crate::{panic, raw, Buf, Delta, DiffFormat, Error, FileMode, Oid, Repository};
use crate::{DiffFlags, DiffStatsFormat, IntoCString};

/// The diff object that contains all individual file deltas.
///
/// This is an opaque structure which will be allocated by one of the diff
/// generator functions on the `Repository` structure (e.g. `diff_tree_to_tree`
/// or other `diff_*` functions).
pub struct Diff<'repo> {
    raw: *mut raw::git_diff,
    _marker: marker::PhantomData<&'repo Repository>,
}

unsafe impl<'repo> Send for Diff<'repo> {}

/// Description of changes to one entry.
pub struct DiffDelta<'a> {
    raw: *mut raw::git_diff_delta,
    _marker: marker::PhantomData<&'a raw::git_diff_delta>,
}

/// Description of one side of a delta.
///
/// Although this is called a "file" it could represent a file, a symbolic
/// link, a submodule commit id, or even a tree (although that only happens if
/// you are tracking type changes or ignored/untracked directories).
pub struct DiffFile<'a> {
    raw: *const raw::git_diff_file,
    _marker: marker::PhantomData<&'a raw::git_diff_file>,
}

/// Structure describing options about how the diff should be executed.
pub struct DiffOptions {
    pathspec: Vec<CString>,
    pathspec_ptrs: Vec<*const c_char>,
    old_prefix: Option<CString>,
    new_prefix: Option<CString>,
    raw: raw::git_diff_options,
}

/// Control behavior of rename and copy detection
pub struct DiffFindOptions {
    raw: raw::git_diff_find_options,
}

/// Control behavior of formatting emails
pub struct DiffFormatEmailOptions {
    raw: raw::git_diff_format_email_options,
}

/// Control behavior of formatting emails
pub struct DiffPatchidOptions {
    raw: raw::git_diff_patchid_options,
}

/// An iterator over the diffs in a delta
pub struct Deltas<'diff> {
    range: Range<usize>,
    diff: &'diff Diff<'diff>,
}

/// Structure describing a line (or data span) of a diff.
pub struct DiffLine<'a> {
    raw: *const raw::git_diff_line,
    _marker: marker::PhantomData<&'a raw::git_diff_line>,
}

/// Structure describing a hunk of a diff.
pub struct DiffHunk<'a> {
    raw: *const raw::git_diff_hunk,
    _marker: marker::PhantomData<&'a raw::git_diff_hunk>,
}

/// Structure describing a hunk of a diff.
pub struct DiffStats {
    raw: *mut raw::git_diff_stats,
}

/// Structure describing the binary contents of a diff.
pub struct DiffBinary<'a> {
    raw: *const raw::git_diff_binary,
    _marker: marker::PhantomData<&'a raw::git_diff_binary>,
}

/// The contents of one of the files in a binary diff.
pub struct DiffBinaryFile<'a> {
    raw: *const raw::git_diff_binary_file,
    _marker: marker::PhantomData<&'a raw::git_diff_binary_file>,
}

/// When producing a binary diff, the binary data returned will be
/// either the deflated full ("literal") contents of the file, or
/// the deflated binary delta between the two sides (whichever is
/// smaller).
#[derive(Copy, Clone, Debug)]
pub enum DiffBinaryKind {
    /// There is no binary delta
    None,
    /// The binary data is the literal contents of the file
    Literal,
    /// The binary data is the delta from one side to the other
    Delta,
}

type PrintCb<'a> = dyn FnMut(DiffDelta<'_>, Option<DiffHunk<'_>>, DiffLine<'_>) -> bool + 'a;

pub type FileCb<'a> = dyn FnMut(DiffDelta<'_>, f32) -> bool + 'a;
pub type BinaryCb<'a> = dyn FnMut(DiffDelta<'_>, DiffBinary<'_>) -> bool + 'a;
pub type HunkCb<'a> = dyn FnMut(DiffDelta<'_>, DiffHunk<'_>) -> bool + 'a;
pub type LineCb<'a> = dyn FnMut(DiffDelta<'_>, Option<DiffHunk<'_>>, DiffLine<'_>) -> bool + 'a;

pub struct DiffCallbacks<'a, 'b, 'c, 'd, 'e, 'f, 'g, 'h> {
    pub file: Option<&'a mut FileCb<'b>>,
    pub binary: Option<&'c mut BinaryCb<'d>>,
    pub hunk: Option<&'e mut HunkCb<'f>>,
    pub line: Option<&'g mut LineCb<'h>>,
}

impl<'repo> Diff<'repo> {
    /// Merge one diff into another.
    ///
    /// This merges items from the "from" list into the "self" list.  The
    /// resulting diff will have all items that appear in either list.
    /// If an item appears in both lists, then it will be "merged" to appear
    /// as if the old version was from the "onto" list and the new version
    /// is from the "from" list (with the exception that if the item has a
    /// pending DELETE in the middle, then it will show as deleted).
    pub fn merge(&mut self, from: &Diff<'repo>) -> Result<(), Error> {
        unsafe {
            try_call!(raw::git_diff_merge(self.raw, &*from.raw));
        }
        Ok(())
    }

    /// Returns an iterator over the deltas in this diff.
    pub fn deltas(&self) -> Deltas<'_> {
        let num_deltas = unsafe { raw::git_diff_num_deltas(&*self.raw) };
        Deltas {
            range: 0..(num_deltas as usize),
            diff: self,
        }
    }

    /// Return the diff delta for an entry in the diff list.
    pub fn get_delta(&self, i: usize) -> Option<DiffDelta<'_>> {
        unsafe {
            let ptr = raw::git_diff_get_delta(&*self.raw, i as size_t);
            Binding::from_raw_opt(ptr as *mut _)
        }
    }

    /// Check if deltas are sorted case sensitively or insensitively.
    pub fn is_sorted_icase(&self) -> bool {
        unsafe { raw::git_diff_is_sorted_icase(&*self.raw) == 1 }
    }

    /// Iterate over a diff generating formatted text output.
    ///
    /// Returning `false` from the callback will terminate the iteration and
    /// return an error from this function.
    pub fn print<F>(&self, format: DiffFormat, mut cb: F) -> Result<(), Error>
    where
        F: FnMut(DiffDelta<'_>, Option<DiffHunk<'_>>, DiffLine<'_>) -> bool,
    {
        let mut cb: &mut PrintCb<'_> = &mut cb;
        let ptr = &mut cb as *mut _;
        let print: raw::git_diff_line_cb = Some(print_cb);
        unsafe {
            try_call!(raw::git_diff_print(self.raw, format, print, ptr as *mut _));
            Ok(())
        }
    }

    /// Loop over all deltas in a diff issuing callbacks.
    ///
    /// Returning `false` from any callback will terminate the iteration and
    /// return an error from this function.
    pub fn foreach(
        &self,
        file_cb: &mut FileCb<'_>,
        binary_cb: Option<&mut BinaryCb<'_>>,
        hunk_cb: Option<&mut HunkCb<'_>>,
        line_cb: Option<&mut LineCb<'_>>,
    ) -> Result<(), Error> {
        let mut cbs = DiffCallbacks {
            file: Some(file_cb),
            binary: binary_cb,
            hunk: hunk_cb,
            line: line_cb,
        };
        let ptr = &mut cbs as *mut _;
        unsafe {
            let binary_cb_c: raw::git_diff_binary_cb = if cbs.binary.is_some() {
                Some(binary_cb_c)
            } else {
                None
            };
            let hunk_cb_c: raw::git_diff_hunk_cb = if cbs.hunk.is_some() {
                Some(hunk_cb_c)
            } else {
                None
            };
            let line_cb_c: raw::git_diff_line_cb = if cbs.line.is_some() {
                Some(line_cb_c)
            } else {
                None
            };
            let file_cb: raw::git_diff_file_cb = Some(file_cb_c);
            try_call!(raw::git_diff_foreach(
                self.raw,
                file_cb,
                binary_cb_c,
                hunk_cb_c,
                line_cb_c,
                ptr as *mut _
            ));
            Ok(())
        }
    }

    /// Accumulate diff statistics for all patches.
    pub fn stats(&self) -> Result<DiffStats, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_diff_get_stats(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }

    /// Transform a diff marking file renames, copies, etc.
    ///
    /// This modifies a diff in place, replacing old entries that look like
    /// renames or copies with new entries reflecting those changes. This also
    /// will, if requested, break modified files into add/remove pairs if the
    /// amount of change is above a threshold.
    pub fn find_similar(&mut self, opts: Option<&mut DiffFindOptions>) -> Result<(), Error> {
        let opts = opts.map(|opts| &opts.raw);
        unsafe {
            try_call!(raw::git_diff_find_similar(self.raw, opts));
        }
        Ok(())
    }

    /// Create an e-mail ready patch from a diff.
    ///
    /// Matches the format created by `git format-patch`
    #[doc(hidden)]
    #[deprecated(note = "refactored to `Email::from_diff` to match upstream")]
    pub fn format_email(
        &mut self,
        patch_no: usize,
        total_patches: usize,
        commit: &crate::Commit<'repo>,
        opts: Option<&mut DiffFormatEmailOptions>,
    ) -> Result<Buf, Error> {
        assert!(patch_no > 0);
        assert!(patch_no <= total_patches);
        let mut default = DiffFormatEmailOptions::default();
        let raw_opts = opts.map_or(&mut default.raw, |opts| &mut opts.raw);
        let summary = commit.summary_bytes().unwrap();
        let mut message = commit.message_bytes();
        assert!(message.starts_with(summary));
        message = &message[summary.len()..];
        raw_opts.patch_no = patch_no;
        raw_opts.total_patches = total_patches;
        let id = commit.id();
        raw_opts.id = id.raw();
        raw_opts.summary = summary.as_ptr() as *const _;
        raw_opts.body = message.as_ptr() as *const _;
        raw_opts.author = commit.author().raw();
        let buf = Buf::new();
        #[allow(deprecated)]
        unsafe {
            try_call!(raw::git_diff_format_email(buf.raw(), self.raw, &*raw_opts));
        }
        Ok(buf)
    }

    /// Create a patch ID from a diff.
    pub fn patchid(&self, opts: Option<&mut DiffPatchidOptions>) -> Result<Oid, Error> {
        let mut raw = raw::git_oid {
            id: [0; raw::GIT_OID_RAWSZ],
        };
        unsafe {
            try_call!(raw::git_diff_patchid(
                &mut raw,
                self.raw,
                opts.map(|o| &mut o.raw)
            ));
            Ok(Binding::from_raw(&raw as *const _))
        }
    }

    // TODO: num_deltas_of_type, find_similar
}
impl Diff<'static> {
    /// Read the contents of a git patch file into a `git_diff` object.
    ///
    /// The diff object produced is similar to the one that would be
    /// produced if you actually produced it computationally by comparing
    /// two trees, however there may be subtle differences. For example,
    /// a patch file likely contains abbreviated object IDs, so the
    /// object IDs parsed by this function will also be abbreviated.
    pub fn from_buffer(buffer: &[u8]) -> Result<Diff<'static>, Error> {
        crate::init();
        let mut diff: *mut raw::git_diff = std::ptr::null_mut();
        unsafe {
            // NOTE: Doesn't depend on repo, so lifetime can be 'static
            try_call!(raw::git_diff_from_buffer(
                &mut diff,
                buffer.as_ptr() as *const c_char,
                buffer.len()
            ));
            Ok(Diff::from_raw(diff))
        }
    }
}

pub extern "C" fn print_cb(
    delta: *const raw::git_diff_delta,
    hunk: *const raw::git_diff_hunk,
    line: *const raw::git_diff_line,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let delta = Binding::from_raw(delta as *mut _);
        let hunk = Binding::from_raw_opt(hunk);
        let line = Binding::from_raw(line);

        let r = panic::wrap(|| {
            let data = data as *mut &mut PrintCb<'_>;
            (*data)(delta, hunk, line)
        });
        if r == Some(true) {
            raw::GIT_OK
        } else {
            raw::GIT_EUSER
        }
    }
}

pub extern "C" fn file_cb_c(
    delta: *const raw::git_diff_delta,
    progress: f32,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let delta = Binding::from_raw(delta as *mut _);

        let r = panic::wrap(|| {
            let cbs = data as *mut DiffCallbacks<'_, '_, '_, '_, '_, '_, '_, '_>;
            match (*cbs).file {
                Some(ref mut cb) => cb(delta, progress),
                None => false,
            }
        });
        if r == Some(true) {
            raw::GIT_OK
        } else {
            raw::GIT_EUSER
        }
    }
}

pub extern "C" fn binary_cb_c(
    delta: *const raw::git_diff_delta,
    binary: *const raw::git_diff_binary,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let delta = Binding::from_raw(delta as *mut _);
        let binary = Binding::from_raw(binary);

        let r = panic::wrap(|| {
            let cbs = data as *mut DiffCallbacks<'_, '_, '_, '_, '_, '_, '_, '_>;
            match (*cbs).binary {
                Some(ref mut cb) => cb(delta, binary),
                None => false,
            }
        });
        if r == Some(true) {
            raw::GIT_OK
        } else {
            raw::GIT_EUSER
        }
    }
}

pub extern "C" fn hunk_cb_c(
    delta: *const raw::git_diff_delta,
    hunk: *const raw::git_diff_hunk,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let delta = Binding::from_raw(delta as *mut _);
        let hunk = Binding::from_raw(hunk);

        let r = panic::wrap(|| {
            let cbs = data as *mut DiffCallbacks<'_, '_, '_, '_, '_, '_, '_, '_>;
            match (*cbs).hunk {
                Some(ref mut cb) => cb(delta, hunk),
                None => false,
            }
        });
        if r == Some(true) {
            raw::GIT_OK
        } else {
            raw::GIT_EUSER
        }
    }
}

pub extern "C" fn line_cb_c(
    delta: *const raw::git_diff_delta,
    hunk: *const raw::git_diff_hunk,
    line: *const raw::git_diff_line,
    data: *mut c_void,
) -> c_int {
    unsafe {
        let delta = Binding::from_raw(delta as *mut _);
        let hunk = Binding::from_raw_opt(hunk);
        let line = Binding::from_raw(line);

        let r = panic::wrap(|| {
            let cbs = data as *mut DiffCallbacks<'_, '_, '_, '_, '_, '_, '_, '_>;
            match (*cbs).line {
                Some(ref mut cb) => cb(delta, hunk, line),
                None => false,
            }
        });
        if r == Some(true) {
            raw::GIT_OK
        } else {
            raw::GIT_EUSER
        }
    }
}

impl<'repo> Binding for Diff<'repo> {
    type Raw = *mut raw::git_diff;
    unsafe fn from_raw(raw: *mut raw::git_diff) -> Diff<'repo> {
        Diff {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_diff {
        self.raw
    }
}

impl<'repo> Drop for Diff<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_diff_free(self.raw) }
    }
}

impl<'a> DiffDelta<'a> {
    /// Returns the flags on the delta.
    ///
    /// For more information, see `DiffFlags`'s documentation.
    pub fn flags(&self) -> DiffFlags {
        let flags = unsafe { (*self.raw).flags };
        let mut result = DiffFlags::empty();

        #[cfg(target_env = "msvc")]
        fn as_u32(flag: i32) -> u32 {
            flag as u32
        }
        #[cfg(not(target_env = "msvc"))]
        fn as_u32(flag: u32) -> u32 {
            flag
        }

        if (flags & as_u32(raw::GIT_DIFF_FLAG_BINARY)) != 0 {
            result |= DiffFlags::BINARY;
        }
        if (flags & as_u32(raw::GIT_DIFF_FLAG_NOT_BINARY)) != 0 {
            result |= DiffFlags::NOT_BINARY;
        }
        if (flags & as_u32(raw::GIT_DIFF_FLAG_VALID_ID)) != 0 {
            result |= DiffFlags::VALID_ID;
        }
        if (flags & as_u32(raw::GIT_DIFF_FLAG_EXISTS)) != 0 {
            result |= DiffFlags::EXISTS;
        }
        result
    }

    // TODO: expose when diffs are more exposed
    // pub fn similarity(&self) -> u16 {
    //     unsafe { (*self.raw).similarity }
    // }

    /// Returns the number of files in this delta.
    pub fn nfiles(&self) -> u16 {
        unsafe { (*self.raw).nfiles }
    }

    /// Returns the status of this entry
    ///
    /// For more information, see `Delta`'s documentation
    pub fn status(&self) -> Delta {
        match unsafe { (*self.raw).status } {
            raw::GIT_DELTA_UNMODIFIED => Delta::Unmodified,
            raw::GIT_DELTA_ADDED => Delta::Added,
            raw::GIT_DELTA_DELETED => Delta::Deleted,
            raw::GIT_DELTA_MODIFIED => Delta::Modified,
            raw::GIT_DELTA_RENAMED => Delta::Renamed,
            raw::GIT_DELTA_COPIED => Delta::Copied,
            raw::GIT_DELTA_IGNORED => Delta::Ignored,
            raw::GIT_DELTA_UNTRACKED => Delta::Untracked,
            raw::GIT_DELTA_TYPECHANGE => Delta::Typechange,
            raw::GIT_DELTA_UNREADABLE => Delta::Unreadable,
            raw::GIT_DELTA_CONFLICTED => Delta::Conflicted,
            n => panic!("unknown diff status: {}", n),
        }
    }

    /// Return the file which represents the "from" side of the diff.
    ///
    /// What side this means depends on the function that was used to generate
    /// the diff and will be documented on the function itself.
    pub fn old_file(&self) -> DiffFile<'a> {
        unsafe { Binding::from_raw(&(*self.raw).old_file as *const _) }
    }

    /// Return the file which represents the "to" side of the diff.
    ///
    /// What side this means depends on the function that was used to generate
    /// the diff and will be documented on the function itself.
    pub fn new_file(&self) -> DiffFile<'a> {
        unsafe { Binding::from_raw(&(*self.raw).new_file as *const _) }
    }
}

impl<'a> Binding for DiffDelta<'a> {
    type Raw = *mut raw::git_diff_delta;
    unsafe fn from_raw(raw: *mut raw::git_diff_delta) -> DiffDelta<'a> {
        DiffDelta {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_diff_delta {
        self.raw
    }
}

impl<'a> std::fmt::Debug for DiffDelta<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DiffDelta")
            .field("nfiles", &self.nfiles())
            .field("status", &self.status())
            .field("old_file", &self.old_file())
            .field("new_file", &self.new_file())
            .finish()
    }
}

impl<'a> DiffFile<'a> {
    /// Returns the Oid of this item.
    ///
    /// If this entry represents an absent side of a diff (e.g. the `old_file`
    /// of a `Added` delta), then the oid returned will be zeroes.
    pub fn id(&self) -> Oid {
        unsafe { Binding::from_raw(&(*self.raw).id as *const _) }
    }

    /// Returns the path, in bytes, of the entry relative to the working
    /// directory of the repository.
    pub fn path_bytes(&self) -> Option<&'a [u8]> {
        static FOO: () = ();
        unsafe { crate::opt_bytes(&FOO, (*self.raw).path) }
    }

    /// Returns the path of the entry relative to the working directory of the
    /// repository.
    pub fn path(&self) -> Option<&'a Path> {
        self.path_bytes().map(util::bytes2path)
    }

    /// Returns the size of this entry, in bytes
    pub fn size(&self) -> u64 {
        unsafe { (*self.raw).size as u64 }
    }

    /// Returns `true` if file(s) are treated as binary data.
    pub fn is_binary(&self) -> bool {
        unsafe { (*self.raw).flags & raw::GIT_DIFF_FLAG_BINARY as u32 != 0 }
    }

    /// Returns `true` if file(s) are treated as text data.
    pub fn is_not_binary(&self) -> bool {
        unsafe { (*self.raw).flags & raw::GIT_DIFF_FLAG_NOT_BINARY as u32 != 0 }
    }

    /// Returns `true` if `id` value is known correct.
    pub fn is_valid_id(&self) -> bool {
        unsafe { (*self.raw).flags & raw::GIT_DIFF_FLAG_VALID_ID as u32 != 0 }
    }

    /// Returns `true` if file exists at this side of the delta.
    pub fn exists(&self) -> bool {
        unsafe { (*self.raw).flags & raw::GIT_DIFF_FLAG_EXISTS as u32 != 0 }
    }

    /// Returns file mode.
    pub fn mode(&self) -> FileMode {
        match unsafe { (*self.raw).mode.into() } {
            raw::GIT_FILEMODE_UNREADABLE => FileMode::Unreadable,
            raw::GIT_FILEMODE_TREE => FileMode::Tree,
            raw::GIT_FILEMODE_BLOB => FileMode::Blob,
            raw::GIT_FILEMODE_BLOB_GROUP_WRITABLE => FileMode::BlobGroupWritable,
            raw::GIT_FILEMODE_BLOB_EXECUTABLE => FileMode::BlobExecutable,
            raw::GIT_FILEMODE_LINK => FileMode::Link,
            raw::GIT_FILEMODE_COMMIT => FileMode::Commit,
            mode => panic!("unknown mode: {}", mode),
        }
    }
}

impl<'a> Binding for DiffFile<'a> {
    type Raw = *const raw::git_diff_file;
    unsafe fn from_raw(raw: *const raw::git_diff_file) -> DiffFile<'a> {
        DiffFile {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_diff_file {
        self.raw
    }
}

impl<'a> std::fmt::Debug for DiffFile<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut ds = f.debug_struct("DiffFile");
        ds.field("id", &self.id());
        if let Some(path_bytes) = &self.path_bytes() {
            ds.field("path_bytes", path_bytes);
        }
        if let Some(path) = &self.path() {
            ds.field("path", path);
        }
        ds.field("size", &self.size()).finish()
    }
}

impl Default for DiffOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffOptions {
    /// Creates a new set of empty diff options.
    ///
    /// All flags and other options are defaulted to false or their otherwise
    /// zero equivalents.
    pub fn new() -> DiffOptions {
        let mut opts = DiffOptions {
            pathspec: Vec::new(),
            pathspec_ptrs: Vec::new(),
            raw: unsafe { mem::zeroed() },
            old_prefix: None,
            new_prefix: None,
        };
        assert_eq!(unsafe { raw::git_diff_init_options(&mut opts.raw, 1) }, 0);
        opts
    }

    fn flag(&mut self, opt: raw::git_diff_option_t, val: bool) -> &mut DiffOptions {
        let opt = opt as u32;
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Flag indicating whether the sides of the diff will be reversed.
    pub fn reverse(&mut self, reverse: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_REVERSE, reverse)
    }

    /// Flag indicating whether ignored files are included.
    pub fn include_ignored(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_IGNORED, include)
    }

    /// Flag indicating whether ignored directories are traversed deeply or not.
    pub fn recurse_ignored_dirs(&mut self, recurse: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_RECURSE_IGNORED_DIRS, recurse)
    }

    /// Flag indicating whether untracked files are in the diff
    pub fn include_untracked(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_UNTRACKED, include)
    }

    /// Flag indicating whether untracked directories are traversed deeply or
    /// not.
    pub fn recurse_untracked_dirs(&mut self, recurse: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_RECURSE_UNTRACKED_DIRS, recurse)
    }

    /// Flag indicating whether unmodified files are in the diff.
    pub fn include_unmodified(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_UNMODIFIED, include)
    }

    /// If enabled, then Typechange delta records are generated.
    pub fn include_typechange(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_TYPECHANGE, include)
    }

    /// Event with `include_typechange`, the tree returned generally shows a
    /// deleted blob. This flag correctly labels the tree transitions as a
    /// typechange record with the `new_file`'s mode set to tree.
    ///
    /// Note that the tree SHA will not be available.
    pub fn include_typechange_trees(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_TYPECHANGE_TREES, include)
    }

    /// Flag indicating whether file mode changes are ignored.
    pub fn ignore_filemode(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_FILEMODE, ignore)
    }

    /// Flag indicating whether all submodules should be treated as unmodified.
    pub fn ignore_submodules(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_SUBMODULES, ignore)
    }

    /// Flag indicating whether case insensitive filenames should be used.
    pub fn ignore_case(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_CASE, ignore)
    }

    /// If pathspecs are specified, this flag means that they should be applied
    /// as an exact match instead of a fnmatch pattern.
    pub fn disable_pathspec_match(&mut self, disable: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_DISABLE_PATHSPEC_MATCH, disable)
    }

    /// Disable updating the `binary` flag in delta records. This is useful when
    /// iterating over a diff if you don't need hunk and data callbacks and want
    /// to avoid having to load a file completely.
    pub fn skip_binary_check(&mut self, skip: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_SKIP_BINARY_CHECK, skip)
    }

    /// When diff finds an untracked directory, to match the behavior of core
    /// Git, it scans the contents for ignored and untracked files. If all
    /// contents are ignored, then the directory is ignored; if any contents are
    /// not ignored, then the directory is untracked. This is extra work that
    /// may not matter in many cases.
    ///
    /// This flag turns off that scan and immediately labels an untracked
    /// directory as untracked (changing the behavior to not match core git).
    pub fn enable_fast_untracked_dirs(&mut self, enable: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_ENABLE_FAST_UNTRACKED_DIRS, enable)
    }

    /// When diff finds a file in the working directory with stat information
    /// different from the index, but the OID ends up being the same, write the
    /// correct stat information into the index. Note: without this flag, diff
    /// will always leave the index untouched.
    pub fn update_index(&mut self, update: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_UPDATE_INDEX, update)
    }

    /// Include unreadable files in the diff
    pub fn include_unreadable(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_UNREADABLE, include)
    }

    /// Include unreadable files in the diff as untracked files
    pub fn include_unreadable_as_untracked(&mut self, include: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INCLUDE_UNREADABLE_AS_UNTRACKED, include)
    }

    /// Treat all files as text, disabling binary attributes and detection.
    pub fn force_text(&mut self, force: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_FORCE_TEXT, force)
    }

    /// Treat all files as binary, disabling text diffs
    pub fn force_binary(&mut self, force: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_FORCE_BINARY, force)
    }

    /// Ignore all whitespace
    pub fn ignore_whitespace(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_WHITESPACE, ignore)
    }

    /// Ignore changes in the amount of whitespace
    pub fn ignore_whitespace_change(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_WHITESPACE_CHANGE, ignore)
    }

    /// Ignore whitespace at the end of line
    pub fn ignore_whitespace_eol(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_WHITESPACE_EOL, ignore)
    }

    /// Ignore blank lines
    pub fn ignore_blank_lines(&mut self, ignore: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_IGNORE_BLANK_LINES, ignore)
    }

    /// When generating patch text, include the content of untracked files.
    ///
    /// This automatically turns on `include_untracked` but it does not turn on
    /// `recurse_untracked_dirs`. Add that flag if you want the content of every
    /// single untracked file.
    pub fn show_untracked_content(&mut self, show: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_SHOW_UNTRACKED_CONTENT, show)
    }

    /// When generating output, include the names of unmodified files if they
    /// are included in the `Diff`. Normally these are skipped in the formats
    /// that list files (e.g. name-only, name-status, raw). Even with this these
    /// will not be included in the patch format.
    pub fn show_unmodified(&mut self, show: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_SHOW_UNMODIFIED, show)
    }

    /// Use the "patience diff" algorithm
    pub fn patience(&mut self, patience: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_PATIENCE, patience)
    }

    /// Take extra time to find the minimal diff
    pub fn minimal(&mut self, minimal: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_MINIMAL, minimal)
    }

    /// Include the necessary deflate/delta information so that `git-apply` can
    /// apply given diff information to binary files.
    pub fn show_binary(&mut self, show: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_SHOW_BINARY, show)
    }

    /// Use a heuristic that takes indentation and whitespace into account
    /// which generally can produce better diffs when dealing with ambiguous
    /// diff hunks.
    pub fn indent_heuristic(&mut self, heuristic: bool) -> &mut DiffOptions {
        self.flag(raw::GIT_DIFF_INDENT_HEURISTIC, heuristic)
    }

    /// Set the number of unchanged lines that define the boundary of a hunk
    /// (and to display before and after).
    ///
    /// The default value for this is 3.
    pub fn context_lines(&mut self, lines: u32) -> &mut DiffOptions {
        self.raw.context_lines = lines;
        self
    }

    /// Set the maximum number of unchanged lines between hunk boundaries before
    /// the hunks will be merged into one.
    ///
    /// The default value for this is 0.
    pub fn interhunk_lines(&mut self, lines: u32) -> &mut DiffOptions {
        self.raw.interhunk_lines = lines;
        self
    }

    /// The default value for this is `core.abbrev` or 7 if unset.
    pub fn id_abbrev(&mut self, abbrev: u16) -> &mut DiffOptions {
        self.raw.id_abbrev = abbrev;
        self
    }

    /// Maximum size (in bytes) above which a blob will be marked as binary
    /// automatically.
    ///
    /// A negative value will disable this entirely.
    ///
    /// The default value for this is 512MB.
    pub fn max_size(&mut self, size: i64) -> &mut DiffOptions {
        self.raw.max_size = size as raw::git_off_t;
        self
    }

    /// The virtual "directory" to prefix old file names with in hunk headers.
    ///
    /// The default value for this is "a".
    pub fn old_prefix<T: IntoCString>(&mut self, t: T) -> &mut DiffOptions {
        self.old_prefix = Some(t.into_c_string().unwrap());
        self
    }

    /// The virtual "directory" to prefix new file names with in hunk headers.
    ///
    /// The default value for this is "b".
    pub fn new_prefix<T: IntoCString>(&mut self, t: T) -> &mut DiffOptions {
        self.new_prefix = Some(t.into_c_string().unwrap());
        self
    }

    /// Add to the array of paths/fnmatch patterns to constrain the diff.
    pub fn pathspec<T: IntoCString>(&mut self, pathspec: T) -> &mut DiffOptions {
        let s = util::cstring_to_repo_path(pathspec).unwrap();
        self.pathspec_ptrs.push(s.as_ptr());
        self.pathspec.push(s);
        self
    }

    /// Acquire a pointer to the underlying raw options.
    ///
    /// This function is unsafe as the pointer is only valid so long as this
    /// structure is not moved, modified, or used elsewhere.
    pub unsafe fn raw(&mut self) -> *const raw::git_diff_options {
        self.raw.old_prefix = self
            .old_prefix
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());
        self.raw.new_prefix = self
            .new_prefix
            .as_ref()
            .map(|s| s.as_ptr())
            .unwrap_or(ptr::null());
        self.raw.pathspec.count = self.pathspec_ptrs.len() as size_t;
        self.raw.pathspec.strings = self.pathspec_ptrs.as_ptr() as *mut _;
        &self.raw as *const _
    }

    // TODO: expose ignore_submodules, notify_cb/notify_payload
}

impl<'diff> Iterator for Deltas<'diff> {
    type Item = DiffDelta<'diff>;
    fn next(&mut self) -> Option<DiffDelta<'diff>> {
        self.range.next().and_then(|i| self.diff.get_delta(i))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.range.size_hint()
    }
}
impl<'diff> DoubleEndedIterator for Deltas<'diff> {
    fn next_back(&mut self) -> Option<DiffDelta<'diff>> {
        self.range.next_back().and_then(|i| self.diff.get_delta(i))
    }
}
impl<'diff> FusedIterator for Deltas<'diff> {}

impl<'diff> ExactSizeIterator for Deltas<'diff> {}

/// Line origin constants.
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum DiffLineType {
    /// These values will be sent to `git_diff_line_cb` along with the line
    Context,
    ///
    Addition,
    ///
    Deletion,
    /// Both files have no LF at end
    ContextEOFNL,
    /// Old has no LF at end, new does
    AddEOFNL,
    /// Old has LF at end, new does not
    DeleteEOFNL,
    /// The following values will only be sent to a `git_diff_line_cb` when
    /// the content of a diff is being formatted through `git_diff_print`.
    FileHeader,
    ///
    HunkHeader,
    /// For "Binary files x and y differ"
    Binary,
}

impl Binding for DiffLineType {
    type Raw = raw::git_diff_line_t;
    unsafe fn from_raw(raw: raw::git_diff_line_t) -> Self {
        match raw {
            raw::GIT_DIFF_LINE_CONTEXT => DiffLineType::Context,
            raw::GIT_DIFF_LINE_ADDITION => DiffLineType::Addition,
            raw::GIT_DIFF_LINE_DELETION => DiffLineType::Deletion,
            raw::GIT_DIFF_LINE_CONTEXT_EOFNL => DiffLineType::ContextEOFNL,
            raw::GIT_DIFF_LINE_ADD_EOFNL => DiffLineType::AddEOFNL,
            raw::GIT_DIFF_LINE_DEL_EOFNL => DiffLineType::DeleteEOFNL,
            raw::GIT_DIFF_LINE_FILE_HDR => DiffLineType::FileHeader,
            raw::GIT_DIFF_LINE_HUNK_HDR => DiffLineType::HunkHeader,
            raw::GIT_DIFF_LINE_BINARY => DiffLineType::Binary,
            _ => panic!("Unknown git diff line type"),
        }
    }
    fn raw(&self) -> raw::git_diff_line_t {
        match *self {
            DiffLineType::Context => raw::GIT_DIFF_LINE_CONTEXT,
            DiffLineType::Addition => raw::GIT_DIFF_LINE_ADDITION,
            DiffLineType::Deletion => raw::GIT_DIFF_LINE_DELETION,
            DiffLineType::ContextEOFNL => raw::GIT_DIFF_LINE_CONTEXT_EOFNL,
            DiffLineType::AddEOFNL => raw::GIT_DIFF_LINE_ADD_EOFNL,
            DiffLineType::DeleteEOFNL => raw::GIT_DIFF_LINE_DEL_EOFNL,
            DiffLineType::FileHeader => raw::GIT_DIFF_LINE_FILE_HDR,
            DiffLineType::HunkHeader => raw::GIT_DIFF_LINE_HUNK_HDR,
            DiffLineType::Binary => raw::GIT_DIFF_LINE_BINARY,
        }
    }
}

impl<'a> DiffLine<'a> {
    /// Line number in old file or `None` for added line
    pub fn old_lineno(&self) -> Option<u32> {
        match unsafe { (*self.raw).old_lineno } {
            n if n < 0 => None,
            n => Some(n as u32),
        }
    }

    /// Line number in new file or `None` for deleted line
    pub fn new_lineno(&self) -> Option<u32> {
        match unsafe { (*self.raw).new_lineno } {
            n if n < 0 => None,
            n => Some(n as u32),
        }
    }

    /// Number of newline characters in content
    pub fn num_lines(&self) -> u32 {
        unsafe { (*self.raw).num_lines as u32 }
    }

    /// Offset in the original file to the content
    pub fn content_offset(&self) -> i64 {
        unsafe { (*self.raw).content_offset as i64 }
    }

    /// Content of this line as bytes.
    pub fn content(&self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(
                (*self.raw).content as *const u8,
                (*self.raw).content_len as usize,
            )
        }
    }

    /// origin of this `DiffLine`.
    ///
    pub fn origin_value(&self) -> DiffLineType {
        unsafe { Binding::from_raw((*self.raw).origin as raw::git_diff_line_t) }
    }

    /// Sigil showing the origin of this `DiffLine`.
    ///
    ///  * ` ` - Line context
    ///  * `+` - Line addition
    ///  * `-` - Line deletion
    ///  * `=` - Context (End of file)
    ///  * `>` - Add (End of file)
    ///  * `<` - Remove (End of file)
    ///  * `F` - File header
    ///  * `H` - Hunk header
    ///  * `B` - Line binary
    pub fn origin(&self) -> char {
        match unsafe { (*self.raw).origin as raw::git_diff_line_t } {
            raw::GIT_DIFF_LINE_CONTEXT => ' ',
            raw::GIT_DIFF_LINE_ADDITION => '+',
            raw::GIT_DIFF_LINE_DELETION => '-',
            raw::GIT_DIFF_LINE_CONTEXT_EOFNL => '=',
            raw::GIT_DIFF_LINE_ADD_EOFNL => '>',
            raw::GIT_DIFF_LINE_DEL_EOFNL => '<',
            raw::GIT_DIFF_LINE_FILE_HDR => 'F',
            raw::GIT_DIFF_LINE_HUNK_HDR => 'H',
            raw::GIT_DIFF_LINE_BINARY => 'B',
            _ => ' ',
        }
    }
}

impl<'a> Binding for DiffLine<'a> {
    type Raw = *const raw::git_diff_line;
    unsafe fn from_raw(raw: *const raw::git_diff_line) -> DiffLine<'a> {
        DiffLine {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_diff_line {
        self.raw
    }
}

impl<'a> std::fmt::Debug for DiffLine<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        let mut ds = f.debug_struct("DiffLine");
        if let Some(old_lineno) = &self.old_lineno() {
            ds.field("old_lineno", old_lineno);
        }
        if let Some(new_lineno) = &self.new_lineno() {
            ds.field("new_lineno", new_lineno);
        }
        ds.field("num_lines", &self.num_lines())
            .field("content_offset", &self.content_offset())
            .field("content", &self.content())
            .field("origin", &self.origin())
            .finish()
    }
}

impl<'a> DiffHunk<'a> {
    /// Starting line number in old_file
    pub fn old_start(&self) -> u32 {
        unsafe { (*self.raw).old_start as u32 }
    }

    /// Number of lines in old_file
    pub fn old_lines(&self) -> u32 {
        unsafe { (*self.raw).old_lines as u32 }
    }

    /// Starting line number in new_file
    pub fn new_start(&self) -> u32 {
        unsafe { (*self.raw).new_start as u32 }
    }

    /// Number of lines in new_file
    pub fn new_lines(&self) -> u32 {
        unsafe { (*self.raw).new_lines as u32 }
    }

    /// Header text
    pub fn header(&self) -> &'a [u8] {
        unsafe {
            slice::from_raw_parts(
                (*self.raw).header.as_ptr() as *const u8,
                (*self.raw).header_len as usize,
            )
        }
    }
}

impl<'a> Binding for DiffHunk<'a> {
    type Raw = *const raw::git_diff_hunk;
    unsafe fn from_raw(raw: *const raw::git_diff_hunk) -> DiffHunk<'a> {
        DiffHunk {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_diff_hunk {
        self.raw
    }
}

impl<'a> std::fmt::Debug for DiffHunk<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DiffHunk")
            .field("old_start", &self.old_start())
            .field("old_lines", &self.old_lines())
            .field("new_start", &self.new_start())
            .field("new_lines", &self.new_lines())
            .field("header", &self.header())
            .finish()
    }
}

impl DiffStats {
    /// Get the total number of files changed in a diff.
    pub fn files_changed(&self) -> usize {
        unsafe { raw::git_diff_stats_files_changed(&*self.raw) as usize }
    }

    /// Get the total number of insertions in a diff
    pub fn insertions(&self) -> usize {
        unsafe { raw::git_diff_stats_insertions(&*self.raw) as usize }
    }

    /// Get the total number of deletions in a diff
    pub fn deletions(&self) -> usize {
        unsafe { raw::git_diff_stats_deletions(&*self.raw) as usize }
    }

    /// Print diff statistics to a Buf
    pub fn to_buf(&self, format: DiffStatsFormat, width: usize) -> Result<Buf, Error> {
        let buf = Buf::new();
        unsafe {
            try_call!(raw::git_diff_stats_to_buf(
                buf.raw(),
                self.raw,
                format.bits(),
                width as size_t
            ));
        }
        Ok(buf)
    }
}

impl Binding for DiffStats {
    type Raw = *mut raw::git_diff_stats;

    unsafe fn from_raw(raw: *mut raw::git_diff_stats) -> DiffStats {
        DiffStats { raw }
    }
    fn raw(&self) -> *mut raw::git_diff_stats {
        self.raw
    }
}

impl Drop for DiffStats {
    fn drop(&mut self) {
        unsafe { raw::git_diff_stats_free(self.raw) }
    }
}

impl std::fmt::Debug for DiffStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("DiffStats")
            .field("files_changed", &self.files_changed())
            .field("insertions", &self.insertions())
            .field("deletions", &self.deletions())
            .finish()
    }
}

impl<'a> DiffBinary<'a> {
    /// Returns whether there is data in this binary structure or not.
    ///
    /// If this is `true`, then this was produced and included binary content.
    /// If this is `false` then this was generated knowing only that a binary
    /// file changed but without providing the data, probably from a patch that
    /// said `Binary files a/file.txt and b/file.txt differ`.
    pub fn contains_data(&self) -> bool {
        unsafe { (*self.raw).contains_data == 1 }
    }

    /// The contents of the old file.
    pub fn old_file(&self) -> DiffBinaryFile<'a> {
        unsafe { Binding::from_raw(&(*self.raw).old_file as *const _) }
    }

    /// The contents of the new file.
    pub fn new_file(&self) -> DiffBinaryFile<'a> {
        unsafe { Binding::from_raw(&(*self.raw).new_file as *const _) }
    }
}

impl<'a> Binding for DiffBinary<'a> {
    type Raw = *const raw::git_diff_binary;
    unsafe fn from_raw(raw: *const raw::git_diff_binary) -> DiffBinary<'a> {
        DiffBinary {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_diff_binary {
        self.raw
    }
}

impl<'a> DiffBinaryFile<'a> {
    /// The type of binary data for this file
    pub fn kind(&self) -> DiffBinaryKind {
        unsafe { Binding::from_raw((*self.raw).kind) }
    }

    /// The binary data, deflated
    pub fn data(&self) -> &[u8] {
        unsafe {
            slice::from_raw_parts((*self.raw).data as *const u8, (*self.raw).datalen as usize)
        }
    }

    /// The length of the binary data after inflation
    pub fn inflated_len(&self) -> usize {
        unsafe { (*self.raw).inflatedlen as usize }
    }
}

impl<'a> Binding for DiffBinaryFile<'a> {
    type Raw = *const raw::git_diff_binary_file;
    unsafe fn from_raw(raw: *const raw::git_diff_binary_file) -> DiffBinaryFile<'a> {
        DiffBinaryFile {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *const raw::git_diff_binary_file {
        self.raw
    }
}

impl Binding for DiffBinaryKind {
    type Raw = raw::git_diff_binary_t;
    unsafe fn from_raw(raw: raw::git_diff_binary_t) -> DiffBinaryKind {
        match raw {
            raw::GIT_DIFF_BINARY_NONE => DiffBinaryKind::None,
            raw::GIT_DIFF_BINARY_LITERAL => DiffBinaryKind::Literal,
            raw::GIT_DIFF_BINARY_DELTA => DiffBinaryKind::Delta,
            _ => panic!("Unknown git diff binary kind"),
        }
    }
    fn raw(&self) -> raw::git_diff_binary_t {
        match *self {
            DiffBinaryKind::None => raw::GIT_DIFF_BINARY_NONE,
            DiffBinaryKind::Literal => raw::GIT_DIFF_BINARY_LITERAL,
            DiffBinaryKind::Delta => raw::GIT_DIFF_BINARY_DELTA,
        }
    }
}

impl Default for DiffFindOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffFindOptions {
    /// Creates a new set of empty diff find options.
    ///
    /// All flags and other options are defaulted to false or their otherwise
    /// zero equivalents.
    pub fn new() -> DiffFindOptions {
        let mut opts = DiffFindOptions {
            raw: unsafe { mem::zeroed() },
        };
        assert_eq!(
            unsafe { raw::git_diff_find_init_options(&mut opts.raw, 1) },
            0
        );
        opts
    }

    fn flag(&mut self, opt: u32, val: bool) -> &mut DiffFindOptions {
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Reset all flags back to their unset state, indicating that
    /// `diff.renames` should be used instead. This is overridden once any flag
    /// is set.
    pub fn by_config(&mut self) -> &mut DiffFindOptions {
        self.flag(0xffffffff, false)
    }

    /// Look for renames?
    pub fn renames(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_RENAMES, find)
    }

    /// Consider old side of modified for renames?
    pub fn renames_from_rewrites(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_RENAMES_FROM_REWRITES, find)
    }

    /// Look for copies?
    pub fn copies(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_COPIES, find)
    }

    /// Consider unmodified as copy sources?
    ///
    /// For this to work correctly, use `include_unmodified` when the initial
    /// diff is being generated.
    pub fn copies_from_unmodified(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_COPIES_FROM_UNMODIFIED, find)
    }

    /// Mark significant rewrites for split.
    pub fn rewrites(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_REWRITES, find)
    }

    /// Actually split large rewrites into delete/add pairs
    pub fn break_rewrites(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_BREAK_REWRITES, find)
    }

    #[doc(hidden)]
    pub fn break_rewries(&mut self, find: bool) -> &mut DiffFindOptions {
        self.break_rewrites(find)
    }

    /// Find renames/copies for untracked items in working directory.
    ///
    /// For this to work correctly use the `include_untracked` option when the
    /// initial diff is being generated.
    pub fn for_untracked(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_FOR_UNTRACKED, find)
    }

    /// Turn on all finding features.
    pub fn all(&mut self, find: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_ALL, find)
    }

    /// Measure similarity ignoring leading whitespace (default)
    pub fn ignore_leading_whitespace(&mut self, ignore: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_IGNORE_LEADING_WHITESPACE, ignore)
    }

    /// Measure similarity ignoring all whitespace
    pub fn ignore_whitespace(&mut self, ignore: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_IGNORE_WHITESPACE, ignore)
    }

    /// Measure similarity including all data
    pub fn dont_ignore_whitespace(&mut self, dont: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_DONT_IGNORE_WHITESPACE, dont)
    }

    /// Measure similarity only by comparing SHAs (fast and cheap)
    pub fn exact_match_only(&mut self, exact: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_EXACT_MATCH_ONLY, exact)
    }

    /// Do not break rewrites unless they contribute to a rename.
    ///
    /// Normally, `break_rewrites` and `rewrites` will measure the
    /// self-similarity of modified files and split the ones that have changed a
    /// lot into a delete/add pair.  Then the sides of that pair will be
    /// considered candidates for rename and copy detection
    ///
    /// If you add this flag in and the split pair is not used for an actual
    /// rename or copy, then the modified record will be restored to a regular
    /// modified record instead of being split.
    pub fn break_rewrites_for_renames_only(&mut self, b: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_BREAK_REWRITES_FOR_RENAMES_ONLY, b)
    }

    /// Remove any unmodified deltas after find_similar is done.
    ///
    /// Using `copies_from_unmodified` to emulate the `--find-copies-harder`
    /// behavior requires building a diff with the `include_unmodified` flag. If
    /// you do not want unmodified records in the final result, pas this flag to
    /// have them removed.
    pub fn remove_unmodified(&mut self, remove: bool) -> &mut DiffFindOptions {
        self.flag(raw::GIT_DIFF_FIND_REMOVE_UNMODIFIED, remove)
    }

    /// Similarity to consider a file renamed (default 50)
    pub fn rename_threshold(&mut self, thresh: u16) -> &mut DiffFindOptions {
        self.raw.rename_threshold = thresh;
        self
    }

    /// Similarity of modified to be eligible rename source (default 50)
    pub fn rename_from_rewrite_threshold(&mut self, thresh: u16) -> &mut DiffFindOptions {
        self.raw.rename_from_rewrite_threshold = thresh;
        self
    }

    /// Similarity to consider a file copy (default 50)
    pub fn copy_threshold(&mut self, thresh: u16) -> &mut DiffFindOptions {
        self.raw.copy_threshold = thresh;
        self
    }

    /// Similarity to split modify into delete/add pair (default 60)
    pub fn break_rewrite_threshold(&mut self, thresh: u16) -> &mut DiffFindOptions {
        self.raw.break_rewrite_threshold = thresh;
        self
    }

    /// Maximum similarity sources to examine for a file (somewhat like
    /// git-diff's `-l` option or `diff.renameLimit` config)
    ///
    /// Defaults to 200
    pub fn rename_limit(&mut self, limit: usize) -> &mut DiffFindOptions {
        self.raw.rename_limit = limit as size_t;
        self
    }

    // TODO: expose git_diff_similarity_metric

    /// Acquire a pointer to the underlying raw options.
    pub unsafe fn raw(&mut self) -> *const raw::git_diff_find_options {
        &self.raw
    }
}

impl Default for DiffFormatEmailOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DiffFormatEmailOptions {
    /// Creates a new set of email options,
    /// initialized to the default values
    pub fn new() -> Self {
        let mut opts = DiffFormatEmailOptions {
            raw: unsafe { mem::zeroed() },
        };
        assert_eq!(
            unsafe { raw::git_diff_format_email_options_init(&mut opts.raw, 1) },
            0
        );
        opts
    }

    fn flag(&mut self, opt: u32, val: bool) -> &mut Self {
        if val {
            self.raw.flags |= opt;
        } else {
            self.raw.flags &= !opt;
        }
        self
    }

    /// Exclude `[PATCH]` from the subject header
    pub fn exclude_subject_patch_header(&mut self, should_exclude: bool) -> &mut Self {
        self.flag(
            raw::GIT_DIFF_FORMAT_EMAIL_EXCLUDE_SUBJECT_PATCH_MARKER,
            should_exclude,
        )
    }
}

impl DiffPatchidOptions {
    /// Creates a new set of patchid options,
    /// initialized to the default values
    pub fn new() -> Self {
        let mut opts = DiffPatchidOptions {
            raw: unsafe { mem::zeroed() },
        };
        assert_eq!(
            unsafe {
                raw::git_diff_patchid_options_init(
                    &mut opts.raw,
                    raw::GIT_DIFF_PATCHID_OPTIONS_VERSION,
                )
            },
            0
        );
        opts
    }
}

#[cfg(test)]
mod tests {
    use crate::{DiffLineType, DiffOptions, Oid, Signature, Time};
    use std::borrow::Borrow;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let diff = repo.diff_tree_to_workdir(None, None).unwrap();
        assert_eq!(diff.deltas().len(), 0);
        let stats = diff.stats().unwrap();
        assert_eq!(stats.insertions(), 0);
        assert_eq!(stats.deletions(), 0);
        assert_eq!(stats.files_changed(), 0);
        let patchid = diff.patchid(None).unwrap();
        assert_ne!(patchid, Oid::zero());
    }

    #[test]
    fn foreach_smoke() {
        let (_td, repo) = crate::test::repo_init();
        let diff = t!(repo.diff_tree_to_workdir(None, None));
        let mut count = 0;
        t!(diff.foreach(
            &mut |_file, _progress| {
                count = count + 1;
                true
            },
            None,
            None,
            None
        ));
        assert_eq!(count, 0);
    }

    #[test]
    fn foreach_file_only() {
        let path = Path::new("foo");
        let (td, repo) = crate::test::repo_init();
        t!(t!(File::create(&td.path().join(path))).write_all(b"bar"));
        let mut opts = DiffOptions::new();
        opts.include_untracked(true);
        let diff = t!(repo.diff_tree_to_workdir(None, Some(&mut opts)));
        let mut count = 0;
        let mut result = None;
        t!(diff.foreach(
            &mut |file, _progress| {
                count = count + 1;
                result = file.new_file().path().map(ToOwned::to_owned);
                true
            },
            None,
            None,
            None
        ));
        assert_eq!(result.as_ref().map(Borrow::borrow), Some(path));
        assert_eq!(count, 1);
    }

    #[test]
    fn foreach_file_and_hunk() {
        let path = Path::new("foo");
        let (td, repo) = crate::test::repo_init();
        t!(t!(File::create(&td.path().join(path))).write_all(b"bar"));
        let mut index = t!(repo.index());
        t!(index.add_path(path));
        let mut opts = DiffOptions::new();
        opts.include_untracked(true);
        let diff = t!(repo.diff_tree_to_index(None, Some(&index), Some(&mut opts)));
        let mut new_lines = 0;
        t!(diff.foreach(
            &mut |_file, _progress| { true },
            None,
            Some(&mut |_file, hunk| {
                new_lines = hunk.new_lines();
                true
            }),
            None
        ));
        assert_eq!(new_lines, 1);
    }

    #[test]
    fn foreach_all_callbacks() {
        let fib = vec![0, 1, 1, 2, 3, 5, 8];
        // Verified with a node implementation of deflate, might be worth
        // adding a deflate lib to do this inline here.
        let deflated_fib = vec![120, 156, 99, 96, 100, 100, 98, 102, 229, 0, 0, 0, 53, 0, 21];
        let foo_path = Path::new("foo");
        let bin_path = Path::new("bin");
        let (td, repo) = crate::test::repo_init();
        t!(t!(File::create(&td.path().join(foo_path))).write_all(b"bar\n"));
        t!(t!(File::create(&td.path().join(bin_path))).write_all(&fib));
        let mut index = t!(repo.index());
        t!(index.add_path(foo_path));
        t!(index.add_path(bin_path));
        let mut opts = DiffOptions::new();
        opts.include_untracked(true).show_binary(true);
        let diff = t!(repo.diff_tree_to_index(None, Some(&index), Some(&mut opts)));
        let mut bin_content = None;
        let mut new_lines = 0;
        let mut line_content = None;
        t!(diff.foreach(
            &mut |_file, _progress| { true },
            Some(&mut |_file, binary| {
                bin_content = Some(binary.new_file().data().to_owned());
                true
            }),
            Some(&mut |_file, hunk| {
                new_lines = hunk.new_lines();
                true
            }),
            Some(&mut |_file, _hunk, line| {
                line_content = String::from_utf8(line.content().into()).ok();
                true
            })
        ));
        assert_eq!(bin_content, Some(deflated_fib));
        assert_eq!(new_lines, 1);
        assert_eq!(line_content, Some("bar\n".to_string()));
    }

    #[test]
    fn format_email_simple() {
        let (_td, repo) = crate::test::repo_init();
        const COMMIT_MESSAGE: &str = "Modify some content";
        const EXPECTED_EMAIL_START: &str = concat!(
            "From f1234fb0588b6ed670779a34ba5c51ef962f285f Mon Sep 17 00:00:00 2001\n",
            "From: Techcable <dummy@dummy.org>\n",
            "Date: Tue, 11 Jan 1972 17:46:40 +0000\n",
            "Subject: [PATCH] Modify some content\n",
            "\n",
            "---\n",
            " file1.txt | 8 +++++---\n",
            " 1 file changed, 5 insertions(+), 3 deletions(-)\n",
            "\n",
            "diff --git a/file1.txt b/file1.txt\n",
            "index 94aaae8..af8f41d 100644\n",
            "--- a/file1.txt\n",
            "+++ b/file1.txt\n",
            "@@ -1,15 +1,17 @@\n",
            " file1.txt\n",
            " file1.txt\n",
            "+_file1.txt_\n",
            " file1.txt\n",
            " file1.txt\n",
            " file1.txt\n",
            " file1.txt\n",
            "+\n",
            "+\n",
            " file1.txt\n",
            " file1.txt\n",
            " file1.txt\n",
            " file1.txt\n",
            " file1.txt\n",
            "-file1.txt\n",
            "-file1.txt\n",
            "-file1.txt\n",
            "+_file1.txt_\n",
            "+_file1.txt_\n",
            " file1.txt\n",
            "--\n"
        );
        const ORIGINAL_FILE: &str = concat!(
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n"
        );
        const UPDATED_FILE: &str = concat!(
            "file1.txt\n",
            "file1.txt\n",
            "_file1.txt_\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "\n",
            "\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "file1.txt\n",
            "_file1.txt_\n",
            "_file1.txt_\n",
            "file1.txt\n"
        );
        const FILE_MODE: i32 = 0o100644;
        let original_file = repo.blob(ORIGINAL_FILE.as_bytes()).unwrap();
        let updated_file = repo.blob(UPDATED_FILE.as_bytes()).unwrap();
        let mut original_tree = repo.treebuilder(None).unwrap();
        original_tree
            .insert("file1.txt", original_file, FILE_MODE)
            .unwrap();
        let original_tree = original_tree.write().unwrap();
        let mut updated_tree = repo.treebuilder(None).unwrap();
        updated_tree
            .insert("file1.txt", updated_file, FILE_MODE)
            .unwrap();
        let updated_tree = updated_tree.write().unwrap();
        let time = Time::new(64_000_000, 0);
        let author = Signature::new("Techcable", "dummy@dummy.org", &time).unwrap();
        let updated_commit = repo
            .commit(
                None,
                &author,
                &author,
                COMMIT_MESSAGE,
                &repo.find_tree(updated_tree).unwrap(),
                &[], // NOTE: Have no parents to ensure stable hash
            )
            .unwrap();
        let updated_commit = repo.find_commit(updated_commit).unwrap();
        let mut diff = repo
            .diff_tree_to_tree(
                Some(&repo.find_tree(original_tree).unwrap()),
                Some(&repo.find_tree(updated_tree).unwrap()),
                None,
            )
            .unwrap();
        #[allow(deprecated)]
        let actual_email = diff.format_email(1, 1, &updated_commit, None).unwrap();
        let actual_email = actual_email.as_str().unwrap();
        assert!(
            actual_email.starts_with(EXPECTED_EMAIL_START),
            "Unexpected email:\n{}",
            actual_email
        );
        let mut remaining_lines = actual_email[EXPECTED_EMAIL_START.len()..].lines();
        let version_line = remaining_lines.next();
        assert!(
            version_line.unwrap().starts_with("libgit2"),
            "Invalid version line: {:?}",
            version_line
        );
        while let Some(line) = remaining_lines.next() {
            assert_eq!(line.trim(), "")
        }
    }

    #[test]
    fn foreach_diff_line_origin_value() {
        let foo_path = Path::new("foo");
        let (td, repo) = crate::test::repo_init();
        t!(t!(File::create(&td.path().join(foo_path))).write_all(b"bar\n"));
        let mut index = t!(repo.index());
        t!(index.add_path(foo_path));
        let mut opts = DiffOptions::new();
        opts.include_untracked(true);
        let diff = t!(repo.diff_tree_to_index(None, Some(&index), Some(&mut opts)));
        let mut origin_values: Vec<DiffLineType> = Vec::new();
        t!(diff.foreach(
            &mut |_file, _progress| { true },
            None,
            None,
            Some(&mut |_file, _hunk, line| {
                origin_values.push(line.origin_value());
                true
            })
        ));
        assert_eq!(origin_values.len(), 1);
        assert_eq!(origin_values[0], DiffLineType::Addition);
    }

    #[test]
    fn foreach_exits_with_euser() {
        let foo_path = Path::new("foo");
        let bar_path = Path::new("foo");

        let (td, repo) = crate::test::repo_init();
        t!(t!(File::create(&td.path().join(foo_path))).write_all(b"bar\n"));

        let mut index = t!(repo.index());
        t!(index.add_path(foo_path));
        t!(index.add_path(bar_path));

        let mut opts = DiffOptions::new();
        opts.include_untracked(true);
        let diff = t!(repo.diff_tree_to_index(None, Some(&index), Some(&mut opts)));

        let mut calls = 0;
        let result = diff.foreach(
            &mut |_file, _progress| {
                calls += 1;
                false
            },
            None,
            None,
            None,
        );

        assert_eq!(result.unwrap_err().code(), crate::ErrorCode::User);
    }
}
