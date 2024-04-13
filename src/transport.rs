//! Interfaces for adding custom transports to libgit2

use libc::{c_char, c_int, c_uint, c_void, size_t};
use std::ffi::{CStr, CString};
use std::io;
use std::io::prelude::*;
use std::mem;
use std::ptr;
use std::slice;
use std::str;

use crate::util::Binding;
use crate::{panic, raw, Error, Remote};

/// A transport is a structure which knows how to transfer data to and from a
/// remote.
///
/// This transport is a representation of the raw transport underneath it, which
/// is similar to a trait object in Rust.
#[allow(missing_copy_implementations)]
pub struct Transport {
    raw: *mut raw::git_transport,
    owned: bool,
}

/// Interface used by smart transports.
///
/// The full-fledged definition of transports has to deal with lots of
/// nitty-gritty details of the git protocol, but "smart transports" largely
/// only need to deal with read() and write() of data over a channel.
///
/// A smart subtransport is contained within an instance of a smart transport
/// and is delegated to in order to actually conduct network activity to push or
/// pull data from a remote.
pub trait SmartSubtransport: Send + 'static {
    /// Indicates that this subtransport will be performing the specified action
    /// on the specified URL.
    ///
    /// This function is responsible for making any network connections and
    /// returns a stream which can be read and written from in order to
    /// negotiate the git protocol.
    fn action(&self, url: &str, action: Service)
        -> Result<Box<dyn SmartSubtransportStream>, Error>;

    /// Terminates a connection with the remote.
    ///
    /// Each subtransport is guaranteed a call to close() between calls to
    /// action(), except for the following two natural progressions of actions
    /// against a constant URL.
    ///
    /// 1. UploadPackLs -> UploadPack
    /// 2. ReceivePackLs -> ReceivePack
    fn close(&self) -> Result<(), Error>;
}

/// Actions that a smart transport can ask a subtransport to perform
#[derive(Copy, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum Service {
    UploadPackLs,
    UploadPack,
    ReceivePackLs,
    ReceivePack,
}

/// An instance of a stream over which a smart transport will communicate with a
/// remote.
///
/// Currently this only requires the standard `Read` and `Write` traits. This
/// trait also does not need to be implemented manually as long as the `Read`
/// and `Write` traits are implemented.
pub trait SmartSubtransportStream: Read + Write + Send + 'static {}

impl<T: Read + Write + Send + 'static> SmartSubtransportStream for T {}

type TransportFactory = dyn Fn(&Remote<'_>) -> Result<Transport, Error> + Send + Sync + 'static;

/// Boxed data payload used for registering new transports.
///
/// Currently only contains a field which knows how to create transports.
struct TransportData {
    factory: Box<TransportFactory>,
}

/// Instance of a `git_smart_subtransport`, must use `#[repr(C)]` to ensure that
/// the C fields come first.
#[repr(C)]
struct RawSmartSubtransport {
    raw: raw::git_smart_subtransport,
    stream: Option<*mut raw::git_smart_subtransport_stream>,
    rpc: bool,
    obj: Box<dyn SmartSubtransport>,
}

/// Instance of a `git_smart_subtransport_stream`, must use `#[repr(C)]` to
/// ensure that the C fields come first.
#[repr(C)]
struct RawSmartSubtransportStream {
    raw: raw::git_smart_subtransport_stream,
    obj: Box<dyn SmartSubtransportStream>,
}

/// Add a custom transport definition, to be used in addition to the built-in
/// set of transports that come with libgit2.
///
/// This function is unsafe as it needs to be externally synchronized with calls
/// to creation of other transports.
pub unsafe fn register<F>(prefix: &str, factory: F) -> Result<(), Error>
where
    F: Fn(&Remote<'_>) -> Result<Transport, Error> + Send + Sync + 'static,
{
    crate::init();
    let mut data = Box::new(TransportData {
        factory: Box::new(factory),
    });
    let prefix = CString::new(prefix)?;
    let datap = (&mut *data) as *mut TransportData as *mut c_void;
    let factory: raw::git_transport_cb = Some(transport_factory);
    try_call!(raw::git_transport_register(prefix, factory, datap));
    mem::forget(data);
    Ok(())
}

impl Transport {
    /// Creates a new transport which will use the "smart" transport protocol
    /// for transferring data.
    ///
    /// A smart transport requires a *subtransport* over which data is actually
    /// communicated, but this subtransport largely just needs to be able to
    /// read() and write(). The subtransport provided will be used to make
    /// connections which can then be read/written from.
    ///
    /// The `rpc` argument is `true` if the protocol is stateless, false
    /// otherwise. For example `http://` is stateless but `git://` is not.
    pub fn smart<S>(remote: &Remote<'_>, rpc: bool, subtransport: S) -> Result<Transport, Error>
    where
        S: SmartSubtransport,
    {
        let mut ret = ptr::null_mut();

        let mut raw = Box::new(RawSmartSubtransport {
            raw: raw::git_smart_subtransport {
                action: Some(subtransport_action),
                close: Some(subtransport_close),
                free: Some(subtransport_free),
            },
            stream: None,
            rpc,
            obj: Box::new(subtransport),
        });
        let mut defn = raw::git_smart_subtransport_definition {
            callback: Some(smart_factory),
            rpc: rpc as c_uint,
            param: &mut *raw as *mut _ as *mut _,
        };

        // Currently there's no way to pass a payload via the
        // git_smart_subtransport_definition structure, but it's only used as a
        // configuration for the initial creation of the smart transport (verified
        // by reading the current code, hopefully it doesn't change!).
        //
        // We, however, need some state (gotta pass in our
        // `RawSmartSubtransport`). This also means that this block must be
        // entirely synchronized with a lock (boo!)
        unsafe {
            try_call!(raw::git_transport_smart(
                &mut ret,
                remote.raw(),
                &mut defn as *mut _ as *mut _
            ));
            mem::forget(raw); // ownership transport to `ret`
        }
        return Ok(Transport {
            raw: ret,
            owned: true,
        });

        extern "C" fn smart_factory(
            out: *mut *mut raw::git_smart_subtransport,
            _owner: *mut raw::git_transport,
            ptr: *mut c_void,
        ) -> c_int {
            unsafe {
                *out = ptr as *mut raw::git_smart_subtransport;
                0
            }
        }
    }
}

impl Drop for Transport {
    fn drop(&mut self) {
        if self.owned {
            unsafe { (*self.raw).free.unwrap()(self.raw) }
        }
    }
}

// callback used by register() to create new transports
extern "C" fn transport_factory(
    out: *mut *mut raw::git_transport,
    owner: *mut raw::git_remote,
    param: *mut c_void,
) -> c_int {
    struct Bomb<'a> {
        remote: Option<Remote<'a>>,
    }
    impl<'a> Drop for Bomb<'a> {
        fn drop(&mut self) {
            // TODO: maybe a method instead?
            mem::forget(self.remote.take());
        }
    }

    panic::wrap(|| unsafe {
        let remote = Bomb {
            remote: Some(Binding::from_raw(owner)),
        };
        let data = &mut *(param as *mut TransportData);
        match (data.factory)(remote.remote.as_ref().unwrap()) {
            Ok(mut transport) => {
                *out = transport.raw;
                transport.owned = false;
                0
            }
            Err(e) => e.raw_code() as c_int,
        }
    })
    .unwrap_or(-1)
}

// callback used by smart transports to delegate an action to a
// `SmartSubtransport` trait object.
extern "C" fn subtransport_action(
    stream: *mut *mut raw::git_smart_subtransport_stream,
    raw_transport: *mut raw::git_smart_subtransport,
    url: *const c_char,
    action: raw::git_smart_service_t,
) -> c_int {
    panic::wrap(|| unsafe {
        let url = CStr::from_ptr(url).to_bytes();
        let url = match str::from_utf8(url).ok() {
            Some(s) => s,
            None => return -1,
        };
        let action = match action {
            raw::GIT_SERVICE_UPLOADPACK_LS => Service::UploadPackLs,
            raw::GIT_SERVICE_UPLOADPACK => Service::UploadPack,
            raw::GIT_SERVICE_RECEIVEPACK_LS => Service::ReceivePackLs,
            raw::GIT_SERVICE_RECEIVEPACK => Service::ReceivePack,
            n => panic!("unknown action: {}", n),
        };

        let transport = &mut *(raw_transport as *mut RawSmartSubtransport);
        // Note: we only need to generate if rpc is on. Else, for receive-pack and upload-pack
        // libgit2 reuses the stream generated for receive-pack-ls or upload-pack-ls.
        let generate_stream =
            transport.rpc || action == Service::UploadPackLs || action == Service::ReceivePackLs;
        if generate_stream {
            let obj = match transport.obj.action(url, action) {
                Ok(s) => s,
                Err(e) => return e.raw_set_git_error(),
            };
            *stream = mem::transmute(Box::new(RawSmartSubtransportStream {
                raw: raw::git_smart_subtransport_stream {
                    subtransport: raw_transport,
                    read: Some(stream_read),
                    write: Some(stream_write),
                    free: Some(stream_free),
                },
                obj,
            }));
            transport.stream = Some(*stream);
        } else {
            if transport.stream.is_none() {
                return -1;
            }
            *stream = transport.stream.unwrap();
        }
        0
    })
    .unwrap_or(-1)
}

// callback used by smart transports to close a `SmartSubtransport` trait
// object.
extern "C" fn subtransport_close(transport: *mut raw::git_smart_subtransport) -> c_int {
    let ret = panic::wrap(|| unsafe {
        let transport = &mut *(transport as *mut RawSmartSubtransport);
        transport.obj.close()
    });
    match ret {
        Some(Ok(())) => 0,
        Some(Err(e)) => e.raw_code() as c_int,
        None => -1,
    }
}

// callback used by smart transports to free a `SmartSubtransport` trait
// object.
extern "C" fn subtransport_free(transport: *mut raw::git_smart_subtransport) {
    let _ = panic::wrap(|| unsafe {
        mem::transmute::<_, Box<RawSmartSubtransport>>(transport);
    });
}

// callback used by smart transports to read from a `SmartSubtransportStream`
// object.
extern "C" fn stream_read(
    stream: *mut raw::git_smart_subtransport_stream,
    buffer: *mut c_char,
    buf_size: size_t,
    bytes_read: *mut size_t,
) -> c_int {
    let ret = panic::wrap(|| unsafe {
        let transport = &mut *(stream as *mut RawSmartSubtransportStream);
        let buf = slice::from_raw_parts_mut(buffer as *mut u8, buf_size as usize);
        match transport.obj.read(buf) {
            Ok(n) => {
                *bytes_read = n as size_t;
                Ok(n)
            }
            e => e,
        }
    });
    match ret {
        Some(Ok(_)) => 0,
        Some(Err(e)) => unsafe {
            set_err_io(&e);
            -2
        },
        None => -1,
    }
}

// callback used by smart transports to write to a `SmartSubtransportStream`
// object.
extern "C" fn stream_write(
    stream: *mut raw::git_smart_subtransport_stream,
    buffer: *const c_char,
    len: size_t,
) -> c_int {
    let ret = panic::wrap(|| unsafe {
        let transport = &mut *(stream as *mut RawSmartSubtransportStream);
        let buf = slice::from_raw_parts(buffer as *const u8, len as usize);
        transport.obj.write_all(buf)
    });
    match ret {
        Some(Ok(())) => 0,
        Some(Err(e)) => unsafe {
            set_err_io(&e);
            -2
        },
        None => -1,
    }
}

unsafe fn set_err_io(e: &io::Error) {
    let s = CString::new(e.to_string()).unwrap();
    raw::git_error_set_str(raw::GIT_ERROR_NET as c_int, s.as_ptr());
}

// callback used by smart transports to free a `SmartSubtransportStream`
// object.
extern "C" fn stream_free(stream: *mut raw::git_smart_subtransport_stream) {
    let _ = panic::wrap(|| unsafe {
        mem::transmute::<_, Box<RawSmartSubtransportStream>>(stream);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ErrorClass, ErrorCode};
    use std::sync::Once;

    struct DummyTransport;

    // in lieu of lazy_static
    fn dummy_error() -> Error {
        Error::new(ErrorCode::Ambiguous, ErrorClass::Net, "bleh")
    }

    impl SmartSubtransport for DummyTransport {
        fn action(
            &self,
            _url: &str,
            _service: Service,
        ) -> Result<Box<dyn SmartSubtransportStream>, Error> {
            Err(dummy_error())
        }

        fn close(&self) -> Result<(), Error> {
            Ok(())
        }
    }

    #[test]
    fn transport_error_propagates() {
        static INIT: Once = Once::new();

        unsafe {
            INIT.call_once(|| {
                register("dummy", move |remote| {
                    Transport::smart(&remote, true, DummyTransport)
                })
                .unwrap();
            })
        }

        let (_td, repo) = crate::test::repo_init();
        t!(repo.remote("origin", "dummy://ball"));

        let mut origin = t!(repo.find_remote("origin"));

        match origin.fetch(&["main"], None, None) {
            Ok(()) => unreachable!(),
            Err(e) => assert_eq!(e, dummy_error()),
        }
    }
}
