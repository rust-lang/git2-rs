//! A crate for using ureq and rustls as a backend for HTTP git requests with git2-rs.
//!
//! > **NOTE**: At this time this crate likely does not support a `git push`
//! >           operation, only clones.

use std::error;
use std::io::{self, Read, Write};
use std::str;
use std::sync::{Arc, Mutex, Once};

use git2::transport::SmartSubtransportStream;
use git2::transport::{Service, SmartSubtransport, Transport};
use git2::Error;
use log::{debug, info};

struct UreqTransport {
    /// The URL of the remote server, e.g. "https://github.com/user/repo"
    ///
    /// This is an empty string until the first action is performed.
    /// If there is an HTTP redirect, this will be updated with the new URL.
    base_url: Arc<Mutex<String>>,
    agent: Arc<ureq::Agent>,
}

struct UreqSubtransport {
    service: &'static str,
    url_path: &'static str,
    base_url: Arc<Mutex<String>>,
    method: &'static str,
    agent: Arc<ureq::Agent>,
    stream: Option<Box<dyn Read + Send>>,
}

/// Register the ureq backend for HTTP requests made by libgit2.
///
/// # Safety
///
/// This function is unsafe largely for the same reasons as
/// `git2::transport::register`:
///
/// * The function needs to be synchronized against all other creations of
///   transport (any API calls to libgit2).
/// * The function will leak `agent` as once registered it is not currently
///   possible to unregister the backend.
pub unsafe fn register(agent: Arc<ureq::Agent>) {
    static INIT: Once = Once::new();

    INIT.call_once(move || {
        let agent2 = agent.clone();
        git2::transport::register("http", move |remote| factory(remote, agent.clone())).unwrap();
        git2::transport::register("https", move |remote| factory(remote, agent2.clone())).unwrap();
    });
}

fn factory(remote: &git2::Remote<'_>, agent: Arc<ureq::Agent>) -> Result<Transport, Error> {
    Transport::smart(
        remote,
        true,
        UreqTransport {
            base_url: Arc::new(Mutex::new(String::new())),
            agent,
        },
    )
}

impl SmartSubtransport for UreqTransport {
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
        Ok(Box::new(UreqSubtransport {
            service,
            url_path: path,
            base_url: self.base_url.clone(),
            method,
            agent: self.agent.clone(),
            stream: None,
        }))
    }

    fn close(&self) -> Result<(), Error> {
        Ok(()) // do nothing
    }
}

impl UreqSubtransport {
    fn err<E: Into<Box<dyn error::Error + Send + Sync>>>(&self, err: E) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err)
    }

    fn execute(&mut self, data: &[u8]) -> io::Result<()> {
        if self.stream.is_some() {
            return Err(self.err("already sent HTTP request"));
        }

        let url = format!("{}{}", self.base_url.lock().unwrap(), self.url_path);

        // Prep the request
        debug!("request to {}", url);
        let mut req = match self.method {
            "GET" => self.agent.get(url.as_str()),
            "POST" => self.agent.post(url.as_str()),
            _ => return Err(self.err("invalid HTTP method")),
        };

        // Note: User-Agent must start with "git/" in order to trigger GitHub's
        // smart transport when used with https://github.com/example/example URLs
        // (as opposed to https://github.com/example/example.git).
        req.set(
            "User-Agent",
            format!("git/1.0 (git2-rustls {})", env!("CARGO_PKG_VERSION")).as_str(),
        );
        let resp = if data.len() > 0 {
            assert_eq!(self.method, "POST", "wrong method for write");
            let pre = format!("application/x-git-{}", self.service);
            req.set("Accept", format!("{}-result", pre).as_str());
            req.set("Content-Type", format!("{}-request", pre).as_str());
            req.send_bytes(data)
        } else {
            req.set("Accept", "*/*");
            req.call()
        };

        if let Some(error) = resp.synthetic_error() {
            return Err(self.err(format!("HTTP request failed: {}", error)));
        } else if !resp.ok() {
            return Err(self.err(format!("HTTP request failed: status {}", resp.status())));
        }

        // If there was a redirect, update with the new base.
        let last_url = resp.get_url();
        if last_url != url {
            debug!("redirect from {} to {}, updating base URL", url, last_url);
            let new_base = if last_url.ends_with(self.url_path) {
                // Strip the action from the end.
                &last_url[..last_url.len() - self.url_path.len()]
            } else {
                // I'm not sure if this code path makes sense, but it's what
                // libgit does.
                last_url
            };
            *self.base_url.lock().unwrap() = new_base.to_string();
        }

        self.stream = Some(Box::new(resp.into_reader()));
        Ok(())
    }
}

impl Read for UreqSubtransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.stream.is_none() {
            self.execute(&[])?;
        }
        let stream = self.stream.as_mut().expect("stream was none after execute");
        stream.read(buf)
    }
}

impl Write for UreqSubtransport {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        self.execute(data)?;
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
