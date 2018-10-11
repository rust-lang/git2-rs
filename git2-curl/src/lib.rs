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

use std::error;
use std::io::prelude::*;
use std::io::{self, Cursor};
use std::str;
use std::sync::{Once, ONCE_INIT, Arc, Mutex};

use curl::easy::{Easy, List};
use git2::Error;
use git2::transport::{SmartSubtransportStream};
use git2::transport::{Transport, SmartSubtransport, Service};
use url::Url;

struct CurlTransport {
    handle: Arc<Mutex<Easy>>,
    /// The URL of the remote server, e.g. "https://github.com/user/repo"
    ///
    /// This is an empty string until the first action is performed.
    /// If there is an HTTP redirect, this will be updated with the new URL.
    base_url: Arc<Mutex<String>>
}

struct CurlSubtransport {
    handle: Arc<Mutex<Easy>>,
    service: &'static str,
    url_path: &'static str,
    base_url: Arc<Mutex<String>>,
    method: &'static str,
    reader: Option<Cursor<Vec<u8>>>,
    sent_request: bool,
}

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
pub unsafe fn register(handle: Easy) {
    static INIT: Once = ONCE_INIT;

    let handle = Arc::new(Mutex::new(handle));
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

fn factory(remote: &git2::Remote, handle: Arc<Mutex<Easy>>)
           -> Result<Transport, Error> {
    Transport::smart(remote, true, CurlTransport {
        handle: handle,
        base_url: Arc::new(Mutex::new(String::new()))
    })
}

impl SmartSubtransport for CurlTransport {
    fn action(&self, url: &str, action: Service)
              -> Result<Box<SmartSubtransportStream>, Error> {
        let mut base_url = self.base_url.lock().unwrap();
        if base_url.len() == 0 {
            *base_url = url.to_string();
        }
        let (service, path, method) = match action {
            Service::UploadPackLs => {
                ("upload-pack", "/info/refs?service=git-upload-pack", "GET")
            }
            Service::UploadPack => {
                ("upload-pack", "/git-upload-pack", "POST")
            }
            Service::ReceivePackLs => {
                ("receive-pack", "/info/refs?service=git-receive-pack", "GET")
            }
            Service::ReceivePack => {
                ("receive-pack", "/git-receive-pack", "POST")
            }
        };
        info!("action {} {}", service, path);
        Ok(Box::new(CurlSubtransport {
            handle: self.handle.clone(),
            service: service,
            url_path: path,
            base_url: self.base_url.clone(),
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

    fn execute(&mut self, data: &[u8]) -> io::Result<()> {
        if self.sent_request {
            return Err(self.err("already sent HTTP request"))
        }
        let agent = format!("git/1.0 (git2-curl {})", env!("CARGO_PKG_VERSION"));

        // Parse our input URL to figure out the host
        let url = format!("{}{}", self.base_url.lock().unwrap(), self.url_path);
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
        try!(h.url(&url));
        try!(h.useragent(&agent));
        try!(h.follow_location(true));
        match self.method {
            "GET" => try!(h.get(true)),
            "PUT" => try!(h.put(true)),
            "POST" => try!(h.post(true)),
            other => try!(h.custom_request(other)),
        }

        let mut headers = List::new();
        try!(headers.append(&format!("Host: {}", host)));
        if data.len() > 0 {
            try!(h.post_fields_copy(data));
            try!(headers.append(&format!("Accept: application/x-git-{}-result",
                                         self.service)));
            try!(headers.append(&format!("Content-Type: \
                                          application/x-git-{}-request",
                                         self.service)));
        } else {
            try!(headers.append("Accept: */*"));
        }
        try!(headers.append("Expect:"));
        try!(h.http_headers(headers));

        let mut content_type = None;
        let mut data = Vec::new();
        {
            let mut h = h.transfer();

            // Look for the Content-Type header
            try!(h.header_function(|header| {
                let header = match str::from_utf8(header) {
                    Ok(s) => s,
                    Err(..) => return true,
                };
                let mut parts = header.splitn(2, ": ");
                let name = parts.next().unwrap();
                let value = match parts.next() {
                    Some(value) => value,
                    None => return true,
                };
                if name.eq_ignore_ascii_case("Content-Type") {
                    content_type = Some(value.trim().to_string());
                }

                true
            }));

            // Collect the request's response in-memory
            try!(h.write_function(|buf| {
                data.extend_from_slice(buf);
                Ok(buf.len())
            }));

            // Send the request
            try!(h.perform());
        }

        let code = try!(h.response_code());
        if code != 200 {
            return Err(self.err(&format!("failed to receive HTTP 200 response: \
                                          got {}", code)[..]))
        }

        // Check returned headers
        let expected = match self.method {
            "GET" => format!("application/x-git-{}-advertisement",
                             self.service),
            _ => format!("application/x-git-{}-result", self.service),
        };
        match content_type {
            Some(ref content_type) if *content_type != expected => {
                return Err(self.err(&format!("expected a Content-Type header \
                                              with `{}` but found `{}`",
                                             expected, content_type)[..]))
            }
            Some(..) => {}
            None => {
                return Err(self.err(&format!("expected a Content-Type header \
                                              with `{}` but didn't find one",
                                             expected)[..]))
            }
        }

        // Ok, time to read off some data.
        let rdr = Cursor::new(data);
        self.reader = Some(rdr);

        // If there was a redirect, update the `CurlTransport` with the new base.
        if let Ok(Some(effective_url)) = h.effective_url() {
            let new_base = if effective_url.ends_with(self.url_path) {
                // Strip the action from the end.
                &effective_url[..effective_url.len() - self.url_path.len()]
            } else {
                // I'm not sure if this code path makes sense, but it's what
                // libgit does.
                effective_url
            };
            *self.base_url.lock().unwrap() = new_base.to_string();
        }

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
