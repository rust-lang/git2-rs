//! A crate for using hyper as a backend for HTTP(S) git requests with git2-rs.
//!
//! This crate provides one public function, `register`, which will register
//! a custom HTTP transport with hyper for any HTTP(S) requests made by libgit2.
//! At this time the `register` function is unsafe for the same reasons that
//! `git2::transport::register` is also unsafe.
//!
//! > **NOTE**: At this time this crate likely does not support a `git push`
//! >           operation, only clones.

#![doc(html_root_url = "https://docs.rs/git2-curl/0.14")]
#![deny(missing_docs)]
#![warn(rust_2018_idioms)]
#![cfg_attr(test, deny(warnings))]

use std::error;
use std::io::prelude::*;
use std::io::{self, Cursor};
use std::str::FromStr;
use std::sync::{Arc, Mutex, Once};

use hyper::body::HttpBody;
use hyper::client::HttpConnector;
use hyper::http::header;
use hyper::Body;
use hyper::Request;
use hyper::{Method, Uri};
use hyper_rustls::HttpsConnector;
use log::{debug, info};

use git2::transport::{Service, SmartSubtransport, SmartSubtransportStream, Transport};
use git2::Error;

struct HyperTransport {
    handle: Arc<Mutex<hyper::Client<HttpsConnector<HttpConnector>>>>,
    /// The URL of the remote server, e.g. "https://github.com/user/repo"
    ///
    /// This is an empty string until the first action is performed.
    /// If there is an HTTP redirect, this will be updated with the new URL.
    base_url: Arc<Mutex<String>>,
}

struct HyperSubtransport {
    handle: Arc<Mutex<hyper::Client<HttpsConnector<HttpConnector>>>>,
    service: &'static str,
    url_path: &'static str,
    base_url: Arc<Mutex<String>>,
    method: &'static str,
    reader: Option<Cursor<Vec<u8>>>,
    sent_request: bool,
}

/// Register the hyper backend for HTTP requests made by libgit2.
///
/// This function takes one parameter, a `handle`, which is used to perform all
/// future HTTP(S) requests. The handle can be previously configured with
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
pub unsafe fn register(handle: hyper::Client<HttpsConnector<HttpConnector>>) {
    static INIT: Once = Once::new();

    let handle = Arc::new(Mutex::new(handle));
    let handle2 = handle.clone();
    INIT.call_once(move || {
        git2::transport::register("http", move |remote| factory(remote, handle.clone())).unwrap();
        git2::transport::register("https", move |remote| factory(remote, handle2.clone())).unwrap();
    });
}

fn factory(
    remote: &git2::Remote<'_>,
    handle: Arc<Mutex<hyper::Client<HttpsConnector<HttpConnector>>>>,
) -> Result<Transport, Error> {
    Transport::smart(
        remote,
        true,
        HyperTransport {
            handle,
            base_url: Arc::new(Mutex::new(String::new())),
        },
    )
}

impl SmartSubtransport for HyperTransport {
    fn action(
        &self,
        url: &str,
        action: Service,
    ) -> Result<Box<dyn SmartSubtransportStream>, Error> {
        let mut base_url = self.base_url.lock().unwrap();
        if base_url.len() == 0 {
            *base_url = url.to_string();
        }
        let (service, path, method) = match action {
            Service::UploadPackLs => ("upload-pack", "/info/refs?service=git-upload-pack", "GET"),
            Service::UploadPack => ("upload-pack", "/git-upload-pack", "POST"),
            Service::ReceivePackLs => {
                ("receive-pack", "/info/refs?service=git-receive-pack", "GET")
            }
            Service::ReceivePack => ("receive-pack", "/git-receive-pack", "POST"),
        };
        info!("action {} {}", service, path);
        Ok(Box::new(HyperSubtransport {
            handle: self.handle.clone(),
            service,
            url_path: path,
            base_url: self.base_url.clone(),
            method,
            reader: None,
            sent_request: false,
        }))
    }

    fn close(&self) -> Result<(), Error> {
        Ok(())
    }
}

impl HyperSubtransport {
    fn err<E: Into<Box<dyn error::Error + Send + Sync>>>(&self, err: E) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err)
    }

    fn execute(&mut self, data: &[u8]) -> io::Result<()> {
        if self.sent_request {
            return Err(self.err("already sent HTTP request"));
        }

        // FIXME: wrap a runtime here is definitely NOT a good idea
        let rt = tokio::runtime::Runtime::new().unwrap();

        let agent = format!("git/1.0 (git2-hyper {})", env!("CARGO_PKG_VERSION"));

        // Parse our input URL to figure out the host
        let url = format!("{}{}", self.base_url.lock().unwrap(), self.url_path);
        let parsed = Uri::from_str(&url).map_err(|_| self.err("invalid url, failed to parse"))?;
        let host = match parsed.host() {
            Some(host) => host,
            None => return Err(self.err("invalid url, did not have a host")),
        };

        // Prep the request
        debug!("request to {}", url);
        let client = self.handle.lock().unwrap();

        let method =
            Method::from_bytes(self.method.as_bytes()).map_err(|_| self.err("invalid method"))?;
        let request = Request::builder()
            .method(method)
            .uri(&url)
            .header(header::USER_AGENT, agent)
            .header(header::HOST, host)
            .header(header::EXPECT, "");

        let request = if data.is_empty() {
            request.header(header::ACCEPT, "*/*")
        } else {
            request
                .header(
                    header::ACCEPT,
                    format!("application/x-git-{}-result", self.service),
                )
                .header(
                    header::CONTENT_TYPE,
                    format!("application/x-git-{}-request", self.service),
                )
        };

        let request = request
            .body(Body::from(Vec::from(data)))
            .map_err(|_| self.err("invalid body"))?;

        let mut res = rt.block_on(client.request(request)).unwrap();
        let headers = res.headers();

        let content_type = match headers.get(header::CONTENT_TYPE) {
            Some(v) => Some(v.to_str().unwrap()),
            None => None,
        };

        let code = res.status();
        if code.as_u16() != 200 {
            return Err(self.err(
                &format!(
                    "failed to receive HTTP 200 response: \
                     got {}",
                    code
                )[..],
            ));
        }

        // Check returned headers
        let expected = match self.method {
            "GET" => format!("application/x-git-{}-advertisement", self.service),
            _ => format!("application/x-git-{}-result", self.service),
        };

        if let Some(content_type) = content_type {
            if content_type != expected {
                return Err(self.err(
                    &format!(
                        "expected a Content-Type header \
                         with `{}` but found `{}`",
                        expected, content_type
                    )[..],
                ));
            }
        } else {
            return Err(self.err(
                &format!(
                    "expected a Content-Type header \
                         with `{}` but didn't find one",
                    expected
                )[..],
            ));
        }

        // Ok, time to read off some data.
        let body = rt.block_on(res.body_mut().data());

        let body = match body {
            Some(b) => b,
            None => return Err(self.err("empty response body")),
        };

        let bytes = match body {
            Ok(b) => b,
            Err(_) => return Err(self.err("invalid response body")),
        };

        let mut chunks = vec![];
        for byte in bytes {
            chunks.push(byte);
        }
        self.reader = Some(Cursor::new(chunks));

        Ok(())
    }
}

impl Read for HyperSubtransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.reader.is_none() {
            self.execute(&[])?;
        }
        self.reader.as_mut().unwrap().read(buf)
    }
}

impl Write for HyperSubtransport {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        if self.reader.is_none() {
            self.execute(data)?;
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
