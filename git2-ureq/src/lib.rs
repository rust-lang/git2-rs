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
use log::info;

struct UreqTransport {
    /// The URL of the remote server, e.g. "https://github.com/user/repo"
    ///
    /// This is an empty string until the first action is performed.
    /// If there is an HTTP redirect, this will be updated with the new URL.
    base_url: Arc<Mutex<String>>,
    agent: Arc<ureq::Agent>,
}

impl UreqTransport {
    fn new_get(&self, service: Service) -> Box<dyn SmartSubtransportStream> {
        Box::new(GetSubTransport {
            service,
            parent: self.clone(),
            stream: None,
        })
    }

    fn new_post(&self, service: Service) -> Box<dyn SmartSubtransportStream> {
        Box::new(PostSubTransport {
            service,
            parent: self.clone(),
            stream: None,
        })
    }

    fn make_url(&self, url_path: &str) -> String {
        format!("{}{}", self.base_url.lock().unwrap(), url_path)
    }

    // Set base_url if it hasn't already been set. This function only sets once,
    // to allow redirects to take precedence.
    fn set_base(&self, url: &str) {
        let mut base = self.base_url.lock().unwrap();
        if *base == "" {
            *base = url.to_string();
        }
    }

    // If last_url is different from url, update the base_url accordingly.
    // Used to persist redirects, which is necessary for, e.g. GitLab.
    fn update_base(&self, url: &str, last_url: &str, url_path: &str) {
        // If there was a redirect, update with the new base.
        if last_url != url {
            let new_base = strip_suffix(last_url, url_path);
            // If redirect target doesn't end in url_path, set  base_url
            // to the whole target. Not clear that this makes sense but
            // it's what libgit does.
            let new_base = new_base.unwrap_or(last_url);
            info!("got redirect. updating base url to {}", new_base);
            *self.base_url.lock().unwrap() = new_base.to_string();
        }
    }
}

impl Clone for UreqTransport {
    fn clone(&self) -> UreqTransport {
        UreqTransport {
            base_url: self.base_url.clone(),
            agent: self.agent.clone(),
        }
    }
}

struct GetSubTransport {
    service: Service,
    parent: UreqTransport,
    stream: Option<Box<dyn Read + Send>>,
}

struct PostSubTransport {
    service: Service,
    parent: UreqTransport,
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
        self.set_base(url);
        let subtransport = match action {
            Service::UploadPackLs => self.new_get(action),
            Service::UploadPack => self.new_post(action),
            Service::ReceivePackLs => self.new_get(action),
            Service::ReceivePack => self.new_post(action),
        };
        Ok(subtransport)
    }

    fn close(&self) -> Result<(), Error> {
        Ok(()) // do nothing
    }
}

fn err<E: Into<Box<dyn error::Error + Send + Sync>>>(err: E) -> io::Error {
    io::Error::new(io::ErrorKind::Other, err)
}

fn user_agent() -> String {
    // Note: User-Agent must start with "git/" in order to trigger GitHub's
    // smart transport when used with https://github.com/example/example URLs
    // (as opposed to https://github.com/example/example.git).
    format!("git/1.0 (git2-ureq {})", env!("CARGO_PKG_VERSION"))
}

fn resp_error(resp: &ureq::Response) -> io::Result<()> {
    match resp.synthetic_error() {
        Some(error) => Err(err(format!("HTTP request failed: {}", error))),
        _ if !resp.ok() => Err(err(format!(
            "HTTP request failed: status {}",
            resp.status()
        ))),
        _ => Ok(()),
    }
}

fn service_name(service: Service) -> String {
    match service {
        Service::UploadPackLs | Service::UploadPack => "upload-pack",
        Service::ReceivePackLs | Service::ReceivePack => "receive-pack",
    }
    .to_string()
}

impl Read for GetSubTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if let Some(ref mut stream) = self.stream {
            return stream.read(buf);
        }
        let url_path = format!("/info/refs?service=git-{}", service_name(self.service));
        let url = self.parent.make_url(&url_path);
        let agent = &self.parent.agent;
        let resp = agent
            .get(&url)
            .set("User-Agent", &user_agent())
            .set("Accept", "*/*")
            .call();
        resp_error(&resp)?;
        self.parent.update_base(&url, resp.get_url(), &url_path);

        let mut stream = resp.into_reader();
        let n = stream.read(buf)?;

        self.stream = Some(Box::new(stream));
        Ok(n)
    }
}

impl Write for GetSubTransport {
    fn write(&mut self, _data: &[u8]) -> io::Result<usize> {
        Err(err("write not implemented for GetSubTransport"))
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl Read for PostSubTransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match &mut self.stream {
            Some(stream) => stream.read(buf),
            None => Err(err("PostSubTransport got read before write")),
        }
    }
}

impl Write for PostSubTransport {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        let url_path = format!("/git-{}", service_name(self.service));
        let url = self.parent.make_url(&url_path);
        let pre = format!("application/x-git-{}", service_name(self.service));
        let agent = &self.parent.agent;
        let resp = agent
            .post(&url)
            .set("User-Agent", &user_agent())
            .set("Accept", format!("{}-result", pre).as_str())
            .set("Content-Type", format!("{}-request", pre).as_str())
            .send_bytes(data);
        resp_error(&resp)?;
        self.parent.update_base(&url, resp.get_url(), &url_path);

        self.stream = Some(Box::new(resp.into_reader()));

        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

// String.strip_suffix is new in Rust 1.45: https://doc.rust-lang.org/std/string/struct.String.html#method.strip_suffix
// Use our own implementation for now so we don't need a nightly to build.
fn strip_suffix<'a>(source: &'a str, suffix: &str) -> Option<&'a str> {
    if suffix.len() > source.len() {
        return None;
    }
    let delta = source.len() - suffix.len();
    let potential_match = &source[delta..];
    if potential_match == suffix {
        Some(&source[..delta])
    } else {
        None
    }
}
