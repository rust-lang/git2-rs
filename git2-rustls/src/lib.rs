//! A crate for using rustls as a backend for HTTP git requests with git2-rs.
//!
//! > **NOTE**: At this time this crate likely does not support a `git push`
//! >           operation, only clones.

use rustls;
use std::error;
use std::io::{self, Read, Write};
use std::net;
use std::str;
use std::sync::{Arc, Mutex, Once};

use git2::transport::SmartSubtransportStream;
use git2::transport::{Service, SmartSubtransport, Transport};
use git2::Error;
use log::{debug, info};
use url::Url;
use webpki;
use webpki_roots;

struct RustlsTransport {
    /// The URL of the remote server, e.g. "https://github.com/user/repo"
    ///
    /// This is an empty string until the first action is performed.
    /// If there is an HTTP redirect, this will be updated with the new URL.
    base_url: Arc<Mutex<String>>,
}

struct RustlsSubtransport {
    service: &'static str,
    url_path: &'static str,
    base_url: Arc<Mutex<String>>,
    method: &'static str,
    sent_request: bool,
    stream: Option<rustls::StreamOwned<rustls::ClientSession, net::TcpStream>>,
}

/// Register the rustls backend for HTTP requests made by libgit2.
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
        git2::transport::register("http", move |remote| factory(remote)).unwrap();
        git2::transport::register("https", move |remote| factory(remote)).unwrap();
    });
}

fn factory(remote: &git2::Remote<'_>) -> Result<Transport, Error> {
    Transport::smart(
        remote,
        true,
        RustlsTransport {
            base_url: Arc::new(Mutex::new(String::new())),
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
            service: service,
            url_path: path,
            base_url: self.base_url.clone(),
            method: method,
            sent_request: false,
            stream: None,
        }))
    }

    fn close(&self) -> Result<(), Error> {
        Ok(()) // ...
    }
}

fn read_line<R: Read>(reader: &mut R) -> io::Result<String> {
    let line = read_line2(reader).expect("wtf");
    debug!("received line: {}", line);
    Ok(line)
}

fn read_line2<R: Read>(reader: &mut R) -> io::Result<String> {
    let mut buf = Vec::new();

    loop {
        let mut one_byte = [0_u8];
        let amt = reader.read(&mut one_byte[..])?;

        if amt == 0 {
            return Err(io::Error::new(
                io::ErrorKind::ConnectionAborted,
                "unexpected EOF",
            ));
        }

        if one_byte[0] == b'\n' {
            if matches!(buf.last(), Some(b'\r')) {
                buf.pop(); // remove the '\r'
                return String::from_utf8(buf).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidInput, "header is not in ASCII")
                });
            } else {
                /*debug!("input thus far {}", String::from_utf8(buf.clone()).unwrap());
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "unexpected newline",
                ));
                */
            }
        }

        buf.push(one_byte[0]);
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

        // Parse our input URL to figure out the host
        let url = format!("{}{}", self.base_url.lock().unwrap(), self.url_path);
        let parsed = Url::parse(&url).map_err(|_| self.err("invalid url, failed to parse"))?;
        let host = parsed
            .host_str()
            .ok_or(self.err("invalid url, did not have a host"))?;
        let default_port = match parsed.scheme() {
            "http" => Ok(80),
            "https" => Ok(443),
            _ => Err(self.err("unknown scheme")),
        }?;
        let port = parsed.port().unwrap_or(default_port);

        // Prep the request
        debug!("request to {}", url);
        let mut config = rustls::ClientConfig::new();
        config
            .root_store
            .add_server_trust_anchors(&webpki_roots::TLS_SERVER_ROOTS);
        let name_ref = webpki::DNSNameRef::try_from_ascii_str(host).unwrap();
        let client_session = rustls::ClientSession::new(&Arc::new(config), name_ref);

        let addr = format!("{}:{}", host, port);
        // TODO(jsha): Use connect_timeout (requires doing ToSocketAddr explicitly)
        let sock = net::TcpStream::connect(&addr)?;
        let mut stream = rustls::StreamOwned::new(client_session, sock);

        let agent = format!("git/1.0 (git2-rustls {})", env!("CARGO_PKG_VERSION"));
        let path = parsed.path();
        if let Some(query) = parsed.query() {
            write!(stream, "{} {}?{} HTTP/1.1\r\n", self.method, path, query)?;
        } else {
            write!(stream, "{} {} HTTP/1.1\r\n", self.method, path)?;
        }
        write!(stream, "Host: {}\r\n", host)?;
        write!(stream, "User-Agent: {}\r\n", agent)?;
        //write!(stream, "Expect:\r\n")?; // TODO(jsha): This was in git2-curl. Figure out if it's needed.
        if data.len() > 0 {
            write!(
                stream,
                "Accept: application/x-git-{}-result\r\n",
                self.service
            )?;
            write!(
                stream,
                "Content-Type: application/x-git-{}-request\r\n",
                self.service
            )?;
            stream.write(data)?;
        } else {
            write!(stream, "Accept: */*\r\n")?;
        }
        write!(stream, "\r\n")?; // Done with headers

        let status_line = read_line(&mut stream)?;
        let mut headers = vec![];
        loop {
            headers.push(read_line(&mut stream)?);
            if headers.last() == Some(&"".to_string()) {
                headers.pop();
                break;
            }
        }

        let status_code = status_line
            .splitn(3, ' ')
            .nth(1)
            .ok_or(self.err("bad status line"))?;
        if status_code != "200" {
            return Err(self.err(format!("HTTP status {}", status_code)));
        }

        // If there was a redirect, update the `RustlsTransport` with the new base.
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

        self.stream = Some(stream);
        Ok(())
    }
}

impl Read for RustlsSubtransport {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        debug!("read {}", buf.len());
        if !self.stream.is_some() {
            self.execute(&[])?;
        }
        match self.stream.as_mut().unwrap().read(buf) {
            Ok(size) => Ok(size),
            Err(ref e) if is_close_notify(e) => Ok(0),
            Err(e) => {
                debug!("returning error from read");
                Err(e)
            }
        }
    }
}

fn is_close_notify(e: &io::Error) -> bool {
    if e.kind() != io::ErrorKind::ConnectionAborted {
        return false;
    }

    if let Some(msg) = e.get_ref() {
        return msg.to_string().contains("CloseNotify");
    }

    false
}

impl Write for RustlsSubtransport {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        debug!("write {}", data.len());

        if self.stream.is_none() {
            self.execute(data)?;
        } else {
            panic!("attempted to write on an already-open stream?");
        }
        Ok(data.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
