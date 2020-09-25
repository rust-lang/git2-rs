//! A crate for using rustls as a backend for HTTP git requests with git2-rs.
//!
//! > **NOTE**: At this time this crate likely does not support a `git push`
//! >           operation, only clones.

use std::error;
use std::io::{self, ErrorKind::ConnectionAborted, ErrorKind::InvalidInput, Read, Write};
use std::net;
use std::str;
use std::sync::{Arc, Mutex, Once};

use chunked_transfer::Decoder as ChunkDecoder;
use git2::transport::SmartSubtransportStream;
use git2::transport::{Service, SmartSubtransport, Transport};
use git2::Error;
use log::{debug, info};
use url::Url;

struct RustlsTransport {
    /// The URL of the remote server, e.g. "https://github.com/user/repo"
    ///
    /// This is an empty string until the first action is performed.
    /// If there is an HTTP redirect, this will be updated with the new URL.
    base_url: Arc<Mutex<String>>,
    rustls_config: Arc<rustls::ClientConfig>,
}

struct RustlsSubtransport {
    service: &'static str,
    url_path: &'static str,
    base_url: Arc<Mutex<String>>,
    method: &'static str,
    sent_request: bool,
    rustls_config: Arc<rustls::ClientConfig>,
    stream: Option<Box<dyn Read + Send + Sync>>,
}

/// Register the rustls backend for HTTP requests made by libgit2.
///
/// # Safety
///
/// This function is unsafe largely for the same reasons as
/// `git2::transport::register`:
///
/// * The function needs to be synchronized against all other creations of
///   transport (any API calls to libgit2).
/// * The function will leak `handle` as once registered it is not currently
///   possible to unregister the backend.
pub unsafe fn register() {
    static INIT: Once = Once::new();

    INIT.call_once(move || {
        let mut config = rustls::ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let config = Arc::new(config);
        let config2 = config.clone();
        git2::transport::register("http", move |remote| factory(remote, config.clone())).unwrap();
        git2::transport::register("https", move |remote| factory(remote, config2.clone())).unwrap();
    });
}

fn factory(
    remote: &git2::Remote<'_>,
    rustls_config: Arc<rustls::ClientConfig>,
) -> Result<Transport, Error> {
    Transport::smart(
        remote,
        true,
        RustlsTransport {
            base_url: Arc::new(Mutex::new(String::new())),
            rustls_config,
        },
    )
}

impl SmartSubtransport for RustlsTransport {
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
        Ok(Box::new(RustlsSubtransport {
            service,
            url_path: path,
            base_url: self.base_url.clone(),
            method,
            sent_request: false,
            rustls_config: self.rustls_config.clone(),
            stream: None,
        }))
    }

    fn close(&self) -> Result<(), Error> {
        Ok(()) // do nothing
    }
}

fn read_line<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut buf = Vec::new();

    loop {
        let mut one_byte = [0_u8];
        match reader.read(&mut one_byte[..])? {
            0 => return Err(io::Error::new(ConnectionAborted, "unexpected EOF")),
            _ if one_byte[0] == b'\n' => break,
            _ => buf.push(one_byte[0]),
        }
    }

    if matches!(buf.last(), Some(b'\r')) {
        buf.pop(); // remove the '\r'
        match String::from_utf8(buf) {
            Ok(line) => {
                debug!("read header: {}", &line);
                Ok(line)
            }
            Err(_) => Err(io::Error::new(InvalidInput, "header is not in ASCII")),
        }
    } else {
        Err(io::Error::new(InvalidInput, "unexpected newline"))
    }
}

impl RustlsSubtransport {
    fn err<E: Into<Box<dyn error::Error + Send + Sync>>>(&self, err: E) -> io::Error {
        io::Error::new(io::ErrorKind::Other, err)
    }

    fn execute(&mut self, data: &[u8]) -> io::Result<()> {
        if self.sent_request {
            return Err(self.err("already sent HTTP request"));
        }

        let url = format!("{}{}", self.base_url.lock().unwrap(), self.url_path);
        let parsed = Url::parse(&url).map_err(|_| self.err("invalid url, failed to parse"))?;
        let host = parsed
            .host_str()
            .ok_or_else(|| self.err("invalid url, did not have a host"))?;

        // Prep the request
        debug!("request to {}", url);
        let name_ref =
            webpki::DNSNameRef::try_from_ascii_str(host).map_err(|_| self.err("invalid host"))?;
        let client_session = rustls::ClientSession::new(&self.rustls_config, name_ref);

        let addrs = parsed.socket_addrs(|| None).unwrap();
        let sock = net::TcpStream::connect(&*addrs)?;
        let mut stream = rustls::StreamOwned::new(client_session, sock);

        // Note: User-Agent must start with "git/" in order to trigger GitHub's
        // smart transport when used with https://github.com/example/example URLs
        // (as opposed to https://github.com/example/example.git).
        let agent = format!("git/1.0 (git2-rustls {})", env!("CARGO_PKG_VERSION"));
        let path = parsed.path();
        if let Some(query) = parsed.query() {
            write!(stream, "{} {}?{} HTTP/1.0\r\n", self.method, path, query)?;
        } else {
            write!(stream, "{} {} HTTP/1.0\r\n", self.method, path)?;
        }
        write!(stream, "Host: {}\r\n", host)?;
        write!(stream, "User-Agent: {}\r\n", agent)?;
        if data.len() > 0 {
            assert_eq!(self.method, "POST", "wrong method for write");
            let pre = "application/x-git-";
            write!(stream, "Accept: {}{}-result\r\n", pre, self.service)?;
            write!(stream, "Content-Type: {}{}-request\r\n", pre, self.service)?;
            write!(stream, "Content-Length: {}\r\n", data.len())?;
            write!(stream, "\r\n")?; // Done with headers
            stream.write_all(data)?;
        } else {
            write!(stream, "Accept: */*\r\n")?;
            write!(stream, "\r\n")?; // Done with headers
        }

        let status_line = read_line(&mut stream)?;
        let mut headers = vec![];
        loop {
            let line = read_line(&mut stream)?;
            if line.len() == 0 {
                break;
            } else {
                headers.push(line);
            }
        }

        let status_code = status_line.splitn(3, ' ').nth(1);
        if status_code != Some("200") {
            return Err(self.err(format!("HTTP status {}", status_code.unwrap_or("999"))));
        }

        // If there was a redirect, update with the new base.
        let location = headers.iter().find_map(|h| h.strip_prefix("Location: "));
        if let Some(location) = location {
            let new_base = if location.ends_with(self.url_path) {
                // Strip the action from the end.
                &location[..location.len() - self.url_path.len()]
            } else {
                // I'm not sure if this code path makes sense, but it's what
                // libgit does.
                location
            };
            *self.base_url.lock().unwrap() = new_base.to_string();
        }

        let transfer_encoding = headers
            .iter()
            .find_map(|h| h.strip_prefix("Transfer-Encoding: "));
        if let Some(transfer_encoding) = transfer_encoding {
            if transfer_encoding == "chunked" {
                self.stream = Some(Box::new(ChunkDecoder::new(stream)));
            } else {
                self.stream = Some(Box::new(stream));
            }
        } else {
            self.stream = Some(Box::new(stream));
        }

        Ok(())
    }
}

impl Read for RustlsSubtransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.stream.is_none() {
            self.execute(&[])?;
        }
        let stream = self.stream.as_mut().expect("stream was none after execute");
        stream.read(buf)
    }
}

impl Write for RustlsSubtransport {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        assert!(
            self.stream.is_none(),
            "attempted to write on an already-open stream?"
        );
        self.execute(data)?;
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
