//! Interfaces for adding custom filter to libgit2

use libc::{c_int, c_char, c_void, size_t};
use std::{slice, ptr, marker};
use {raw, call, util, Error, panic};
use std::ffi::CString;
use std::path::Path;
use raw::{git_filter, git_writestream};

#[repr(C)]
struct Filter {
    filter: git_filter,
    attrs: Option<CString>,
    instance: Box<StreamFilter>,
}

#[repr(C)]
struct Stream {
    parent: git_writestream,
    handler: Box<WriteStream>,
}

/// Wrapper of the raw git_writestream object, representing the input
/// of next filter of the chain
struct RawWriteStream {
    stream: *mut git_writestream,
}

/// A trait representing operations to handle filter data stream.
///
/// These callbacks are typically called by libgit2.
pub trait WriteStream : Send + 'static {
    /// New stream data, Implementations should process the buffer data
    /// and then write the output to the next stream if needed.
    fn write(&mut self, buf: &[u8]) -> Result<(), Error>;

    /// Close stream. Implementations should do finalizing work and then
    /// close the next stream.
    fn close(&mut self) -> Result<(), Error>;
}

/// Information to construct a new stream filter.
pub struct FilterSource<'a> {
    raw: *const raw::git_filter_source,
    _marker: marker::PhantomData<&'a str>,
}

impl<'a> FilterSource<'a> {
    fn from_raw(raw: *const raw::git_filter_source) -> FilterSource<'a> {
        FilterSource {
            raw: raw,
            _marker: marker::PhantomData,
        }
    }

    /// Returns `true` if it is a smudge filter, else a clean filter.
    pub fn to_worktree(&self) -> bool {
        unsafe {
            raw::git_filter_source_mode(self.raw) == raw::GIT_FILTER_TO_WORKTREE
        }
    }

    /// Returns the path that the source data is coming from.
    pub fn source_path(&self) -> &Path {
        unsafe {
            let ptr = raw::git_filter_source_path(self.raw);
            util::bytes2path(::opt_bytes(self, ptr).unwrap())
        }
    }
}

/// A trait representing a customized filter to be registered into libgit2
/// filter chain.
///
/// Modeled after git2/sys/filter.h in a streaming manner.
pub trait StreamFilter : Send + Sync + 'static {
	/// Called when the filter is first used for any file.
    fn init(&self) -> Result<(), Error> { Ok(()) }

    /// Create a new stream to handle new blob, should return a initialized
    /// WriteStream.
    ///
    /// Custom filters can retrieve information of source blob from `source`,
    /// and custom filter output should be written to `next` later.
    fn open_stream(&self, source: &FilterSource, next: Box<WriteStream>)
                   -> Result<Box<WriteStream>, Error>;
}

unsafe impl Send for RawWriteStream {}

impl WriteStream for RawWriteStream {
    fn write(&mut self, buf: &[u8])
        -> Result<(), Error> {
        let code = unsafe {
            ((*self.stream).write)((*self).stream,
                buf.as_ptr() as *const c_char, buf.len())
        };
        call::try(code).and(Ok(()))
    }

    fn close(&mut self) -> Result<(), Error> {
        let code = unsafe { ((*self.stream).close)(self.stream) };
        call::try(code).and(Ok(()))
    }
}

extern fn stream_write(stream: *mut git_writestream,
                       buf: *const c_char,
                       len: size_t) -> c_int {
    let ws = stream as *mut Stream;
    panic::wrap(|| unsafe {
        let input = slice::from_raw_parts(buf as *const u8, len);
        match (*ws).handler.write(input) {
            Ok(..) => 0,
            Err(e) => e.raw_code() as c_int,
        }
    }).unwrap_or(-1)
}

extern fn stream_free(stream: *mut git_writestream) {
    let _ = unsafe { Box::from_raw(stream as *mut Stream) };
}

extern fn stream_close(stream: *mut git_writestream) -> c_int {
    let ws = stream as *mut Stream;
    panic::wrap(|| unsafe {
        match (*ws).handler.close() {
            Ok(..) => 0,
            Err(e) => e.raw_code() as c_int,
        }
    }).unwrap_or(-1)
}

extern fn filter_init(filter: *mut git_filter) -> c_int {
    let filter = filter as *mut Filter;
    panic::wrap(|| unsafe {
        match (*filter).instance.init() {
            Ok(..) => 0,
            Err(e) => e.raw_code() as c_int,
        }
    }).unwrap_or(-1)
}

extern fn filter_shutdown(filter: *mut git_filter) {
    let _ = unsafe { Box::from_raw(filter as *mut Filter) };
}

extern fn filter_stream_init(out: *mut *mut raw::git_writestream,
                             filter: *mut git_filter,
                             _payload: *mut *mut c_void,
                             src: *const raw::git_filter_source,
                             next: *mut raw::git_writestream) -> c_int
{
    let filter = filter as *mut Filter;
    // XXX: should bound lifetime of `next` and `source` to some hidden
    // target blob object
    let next = RawWriteStream { stream: next };
    let source = FilterSource::from_raw(src);
    panic::wrap(|| unsafe {
        match (*filter).instance.open_stream(&source, Box::new(next)) {
            Ok(s) => {
                let stream = Box::new(Stream {
                    parent: git_writestream {
                        write: stream_write,
                        close: stream_close,
                        free: stream_free,
                    },
                    handler: s,
                });
                let p = Box::into_raw(stream) as *mut git_writestream;
                *out = p;
                0
            },
            Err(e) => e.raw_code() as c_int,
        }
    }).unwrap_or(-1)
}

impl Filter {
    /// create a command filter
    fn new<T: StreamFilter>(user_filter: T, attrs: Option<&str>) -> Filter {
        let mut f = Filter {
            filter: git_filter {
                version: raw::GIT_FILTER_VERSION,
                attributes: ptr::null(),
                initialize: Some(filter_init),
                shutdown: Some(filter_shutdown),
                check: None,

                // The traditional apply callback can be implemented with
                // a proxy stream as libgit2
                //
                // See libgit2/src/filter.c
                apply: None,
                stream: Some(filter_stream_init),

                // stream filter do not need this
                cleanup: None,
            },
            attrs: attrs.map(|e| { CString::new(e).unwrap() }),
            instance: Box::new(user_filter),
        };
        if let Some(v) = f.attrs.as_ref() {
            f.filter.attributes = v.as_ptr();
        }
        f
    }
}

/// register a command filter.
pub fn register<T: StreamFilter>(name: &str, attributes: Option<&str>,
                                 priority: i32, filter: T)
                                 -> Result<(), Error> {
    let f = Filter::new(filter, attributes);
    _register(name, priority, Box::new(f))
}

fn _register(name: &str, priority: i32, f: Box<Filter>)
             -> Result<(), Error> {
    ::init();
    let name = try!(CString::new(name));
    unsafe {
        let p = Box::into_raw(f) as *mut git_filter;
        try_call!(raw::git_filter_register(name, p,
                                           raw::GIT_FILTER_DRIVER_PRIORITY + priority as c_int));
    }
    Ok(())
}

/// Unregister a command filter.
/// The filter maybe leaked without further tracking if unregistered.
/// libgit2 won't call the shutdown callback if the filter is unregister
/// before any use.
pub unsafe fn unregister(name: &str) -> Result<(), Error> {
    ::init();
    let name = try!(CString::new(name));
    try_call!(raw::git_filter_unregister(name));
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::io::prelude::*;
    use std::fs;
    use std::path::Path;
    use std::ascii::AsciiExt;
    use Error;
    use Repository;
    use filter;
    use filter::{StreamFilter, WriteStream, FilterSource};

    struct CaseStream {
        total_bytes: usize,
        to_worktree: bool,
        next: Box<WriteStream>,
    }

    struct CaseFilter;
    impl StreamFilter for CaseFilter {
        fn open_stream(&self, source: &FilterSource, next: Box<WriteStream>) -> Result<Box<WriteStream>, Error> {
            Ok(Box::new(CaseStream {
                total_bytes: 0,
                to_worktree: source.to_worktree(),
                next: next,
            }))
        }
    }

    impl WriteStream for CaseStream {
        fn write(&mut self, raw_buf: &[u8]) -> Result<(), Error> {
            let buf: Vec<u8> = if self.to_worktree {
                raw_buf.iter().map(|e| { e.to_ascii_uppercase() }).collect()
            } else {
                raw_buf.iter().map(|e| { e.to_ascii_lowercase() }).collect()
            };

            self.total_bytes += raw_buf.len();
            self.next.write(&buf)
        }
        fn close(&mut self) -> Result<(), Error> {
            println!("nop stream closed");
            self.next.close()
        }
    }

    #[test]
    fn filter_register() {
        let f = CaseFilter;
        filter::register("test1", Some("filter=test1"), 50, f).unwrap();
        // filter::unregister("test1").unwrap();
    }

    fn write_file(repo: &Repository, filename: &str, content: &[u8]) {
        let p = Path::new(repo.workdir().unwrap()).join(filename);
        let mut f = fs::File::create(&p).unwrap();
        f.write_all(content).unwrap();
    }

    #[test]
    fn filter_simple() {
        let f = CaseFilter;
        filter::register("casetest", Some("filter=casetest"), 50, f).unwrap();

        let (_td, repo) = ::test::repo_init();
        let sig = repo.signature().unwrap();
        let oid1 = repo.head().unwrap().target().unwrap();
        let commit1 = repo.find_commit(oid1).unwrap();
        let mut index = repo.index().unwrap();

        write_file(&repo, "file_a.bin", b"HELLOWORLD");
        write_file(&repo, "file_b.txt", b"HELLOWORLD");
        write_file(&repo, ".gitattributes", b"*.txt filter=casetest\n");

        index.add_path(Path::new("file_a.bin")).unwrap();
        index.add_path(Path::new("file_b.txt")).unwrap();

        let id_a = index.write_tree().unwrap();
        let tree_a = repo.find_tree(id_a).unwrap();
        let oid2 = repo.commit(Some("refs/heads/branch_a"), &sig, &sig,
        "commit 2", &tree_a, &[&commit1]).unwrap();
        let _commit2 = repo.find_commit(oid2).unwrap();

        // filter::unregister("casetest").unwrap();
    }
}

