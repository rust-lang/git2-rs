extern crate git2;
extern crate curl;

use std::cell::RefCell;
use std::io::{IoResult, MemReader};
use std::sync::{Once, ONCE_INIT};

use curl::http::{Handle, Request};
use curl::http::handle::Method;
use git2::Error;
use git2::transport::{Transport, SmartSubtransport, Service};
use git2::transport::{SmartSubtransportStream};

struct CurlTransport {
    handle: RefCell<Handle>,
}

struct CurlSubtransport {
    service: &'static str,
    url_path: &'static str,
    base_url: String,
    method: Method,
    reader: Option<MemReader>,
}

unsafe impl Send for CurlTransport {} // Handle is not send...

pub fn register() {
    static INIT: Once = ONCE_INIT;

    INIT.call_once(|| unsafe {
        git2::transport::register("http", factory).unwrap();
        git2::transport::register("https", factory).unwrap();
    });
}

fn factory(remote: &git2::Remote) -> Result<Transport, Error> {
    Transport::smart(remote, true, CurlTransport {
        handle: RefCell::new(Handle::new()),
    })
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
        Ok(Box::new(CurlSubtransport {
            service: service,
            url_path: path,
            base_url: url.to_string(),
            method: method,
            reader: None,
        }) as Box<SmartSubtransportStream>)
    }

    fn close(&self) -> Result<(), Error> {
        Ok(()) // ...
    }
}

impl Reader for CurlSubtransport {
    fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
        println!("{} {} {}", self.service, self.url_path, self.base_url);
        match self.reader {
            Some(ref mut r) => return r.read(buf),
            None => {}
        }
        let url = format!("{}{}", self.base_url, self.url_path);
        println!("{}", url);
        let mut h = Handle::new();
        let resp = Request::new(&mut h, self.method).uri(url)
                           .header("User-Agent", "git/1.0 (git2-rs 0.1.0)")
                           .header("Host", "github.com")
                           .header("Accept", "*/*")
                           .exec().unwrap();


        assert_eq!(resp.get_code(), 200);
        let mut rdr = MemReader::new(resp.move_body());
        let ret = rdr.read(buf);
        self.reader = Some(rdr);
        return ret;
    }
}

impl Writer for CurlSubtransport {
    fn write(&mut self, data: &[u8]) -> IoResult<()> {
        panic!()
    }
}

#[test]
fn foo() {
    use git2::Repository;

    register();
    let td = std::io::TempDir::new("wut").unwrap();
    Repository::clone("https://github.com/alexcrichton/git2-rs",
                      td.path()).unwrap();
}
