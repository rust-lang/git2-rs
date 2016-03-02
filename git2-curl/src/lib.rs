//! A crate for using libcurl as a backend for HTTP git requests with git2-rs.
//!
//! This crate provides one public function, `register`, which will register
//! a custom HTTP transport with libcurl for any HTTP requests made by libgit2.
//! At this time the `register` function is unsafe for the same reasons that
//! `git2::transport::register` is also unsafe.
//!
//! It is not recommended to use this crate wherever possible. The current
//! libcurl backend used, `curl-rust`, only supports executing a request in one
//! method call implying no streaming support. This consequently means that
//! when a repository is cloned the entire contents of the repo are downloaded
//! into memory, and *then* written off to disk by libgit2 afterwards. It
//! should be possible to alleviate this problem in the future.
//!
//! > **NOTE**: At this time this crate likely does not support a `git push`
//! >           operation, only clones.

#![doc(html_root_url = "http://alexcrichton.com/git2-rs")]

extern crate git2;
extern crate curl;
extern crate url;
#[macro_use] extern crate log;

use std::io::prelude::*;
use std::io::{self, Cursor};
use std::sync::{Once, ONCE_INIT, Arc, Mutex};
use std::error;

use curl::http::handle::Method;
use curl::http::{Handle, Request};
use git2::Error;
use git2::transport::{SmartSubtransportStream};
use git2::transport::{Transport, SmartSubtransport, Service};
use url::Url;

struct CurlTransport {
    handle: Arc<Mutex<MyHandle>>,
}

struct CurlSubtransport {
    handle: Arc<Mutex<MyHandle>>,
    service: &'static str,
    url_path: &'static str,
    base_url: String,
    method: Method,
    reader: Option<Cursor<Vec<u8>>>,
    sent_request: bool,
}

struct MyHandle(Handle);
unsafe impl Send for MyHandle {} // Handle is not send...

/// Register the libcurl backend for HTTP requests made by libgit2.
///
/// This function takes one parameter, a `handle`, which is used to perform all
/// future HTTP requests. The handle can be previously configured with
/// information such as proxies, SSL information, etc.
///
/// This function is unsafe largely for the same reasons as
/// `git2::transport::register`:
///
/// * The function needs to be synchronized against all other creations of
///   transport (any API calls to libgit2).
/// * The function will leak `handle` as once registered it is not currently
///   possible to unregister the backend.
///
/// This function may be called concurrently, but only the first `handle` will
/// be used. All others will be discarded.
pub unsafe fn register(handle: Handle) {
    static INIT: Once = ONCE_INIT;

    let handle = Arc::new(Mutex::new(MyHandle(handle)));
    let handle2 = handle.clone();
    INIT.call_once(move || {
        git2::transport::register("http", move |remote| {
            factory(remote, handle.clone())
        }).unwrap();
        git2::transport::register("https", move |remote| {
            factory(remote, handle2.clone())
        }).unwrap();
    });
}

fn factory(remote: &git2::Remote, handle: Arc<Mutex<MyHandle>>)
           -> Result<Transport, Error> {
    Transport::smart(remote, true, CurlTransport { handle: handle })
}

impl SmartSubtransport for CurlTransport {
    fn action(&self, url: &str, action: Service)
              -> Result<Box<SmartSubtransportStream>, Error> {
        let (service, path, method) = match action {
            Service::UploadPackLs => {
                ("upload-pack", "/info/refs?service=git-upload-pack", Method::Get)
            }
            Service::UploadPack => {
                ("upload-pack", "/git-upload-pack", Method::Post)
            }
            Service::ReceivePackLs => {
                ("receive-pack", "/info/refs?service=git-receive-pack",
                 Method::Get)
            }
            Service::ReceivePack => {
                ("receive-pack", "/git-receive-pack", Method::Post)
            }
        };
        info!("action {} {}", service, path);
        Ok(Box::new(CurlSubtransport {
            handle: self.handle.clone(),
            service: service,
            url_path: path,
            base_url: url.to_string(),
            method: method,
            reader: None,
            sent_request: false,
        }))
    }

    fn close(&self) -> Result<(), Error> {
        Ok(()) // ...
    }
}

impl CurlSubtransport {
    fn err<E: Into<Box<error::Error+Send+Sync>>>(&self, err: E) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err)
    }

    fn execute(&mut self, mut data: &[u8]) -> io::Result<()> {
        if self.sent_request {
            return Err(self.err("already sent HTTP request"))
        }
        let agent = format!("git/1.0 (git2-curl {})", env!("CARGO_PKG_VERSION"));

        // Parse our input URL to figure out the host
        let url = format!("{}{}", self.base_url, self.url_path);
        let parsed = try!(Url::parse(&url).map_err(|_| {
            self.err("invalid url, failed to parse")
        }));
        let host = match parsed.host_str() {
            Some(host) => host,
            None => return Err(self.err("invalid url, did not have a host")),
        };

        // Prep the request
        debug!("request to {}", url);
        let mut h = self.handle.lock().unwrap();
        let mut req = Request::new(&mut h.0, self.method)
                              .uri(url)
                              .header("User-Agent", &agent)
                              .header("Host", host)
                              .follow_redirects(true);
        if data.len() > 0 {
            req = req.body(&mut data)
                     .content_length(data.len())
                     .header("Accept", &format!("application/x-git-{}-result",
                                                self.service))
                     .header("Content-Type",
                             &format!("application/x-git-{}-request",
                                      self.service));
        } else {
            req = req.header("Accept", "*/*");
        }

        // Send the request
        let resp = try!(req.exec().map_err(|e| self.err(e)));
        debug!("response: {}", resp);
        if resp.get_code() != 200 {
            return Err(self.err(&format!("failed to receive HTTP 200 response: \
                                          got {}", resp.get_code())[..]))
        }

        // Check returned headers
        let expected = match self.method {
            Method::Get => format!("application/x-git-{}-advertisement",
                                   self.service),
            _ => format!("application/x-git-{}-result", self.service),
        };
        if &resp.get_header("content-type") != &[expected.clone()] {
            return Err(self.err(&format!("invalid Content-Type header: \
                                          found `{:?}` expected `{}`",
                                         resp.get_header("Content-Type"),
                                         expected)[..]))
        }

        // Ok, time to read off some data.
        let rdr = Cursor::new(resp.move_body());
        self.reader = Some(rdr);
        Ok(())
    }
}

impl Read for CurlSubtransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.reader.is_none() {
            try!(self.execute(&[]));
        }
        self.reader.as_mut().unwrap().read(buf)
    }
}

impl Write for CurlSubtransport {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.reader.is_none() {
            try!(self.execute(data));
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> { Ok(()) }
}
