use libc::{c_char, c_int, c_uint, c_void};
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use std::slice;
use std::str;

use crate::cert::Cert;
use crate::util::Binding;
use crate::{panic, raw, Cred, CredentialType, Error, IndexerProgress, Oid, Progress};

/// A structure to contain the callbacks which are invoked when a repository is
/// being updated or downloaded.
///
/// These callbacks are used to manage facilities such as authentication,
/// transfer progress, etc.
pub struct RemoteCallbacks<'a> {
    progress: Option<Box<IndexerProgress<'a>>>,
    credentials: Option<Box<Credentials<'a>>>,
    sideband_progress: Option<Box<TransportMessage<'a>>>,
    update_tips: Option<Box<UpdateTips<'a>>>,
    certificate_check: Option<Box<CertificateCheck<'a>>>,
    push_update_reference: Option<Box<PushUpdateReference<'a>>>,
}

/// Callback used to acquire credentials for when a remote is fetched.
///
/// * `url` - the resource for which the credentials are required.
/// * `username_from_url` - the username that was embedded in the url, or `None`
///                         if it was not included.
/// * `allowed_types` - a bitmask stating which cred types are ok to return.
pub type Credentials<'a> =
    dyn FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> + 'a;

/// Callback for receiving messages delivered by the transport.
///
/// The return value indicates whether the network operation should continue.
pub type TransportMessage<'a> = dyn FnMut(&[u8]) -> bool + 'a;

/// Callback for whenever a reference is updated locally.
pub type UpdateTips<'a> = dyn FnMut(&str, Oid, Oid) -> bool + 'a;

/// Callback for a custom certificate check.
///
/// The first argument is the certificate receved on the connection.
/// Certificates are typically either an SSH or X509 certificate.
///
/// The second argument is the hostname for the connection is passed as the last
/// argument.
pub type CertificateCheck<'a> = dyn FnMut(&Cert<'_>, &str) -> bool + 'a;

/// Callback for each updated reference on push.
///
/// The first argument here is the `refname` of the reference, and the second is
/// the status message sent by a server. If the status is `Some` then the update
/// was rejected by the remote server with a reason why.
pub type PushUpdateReference<'a> = dyn FnMut(&str, Option<&str>) -> Result<(), Error> + 'a;

impl<'a> Default for RemoteCallbacks<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> RemoteCallbacks<'a> {
    /// Creates a new set of empty callbacks
    pub fn new() -> RemoteCallbacks<'a> {
        RemoteCallbacks {
            credentials: None,
            progress: None,
            sideband_progress: None,
            update_tips: None,
            certificate_check: None,
            push_update_reference: None,
        }
    }

    /// The callback through which to fetch credentials if required.
    pub fn credentials<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(&str, Option<&str>, CredentialType) -> Result<Cred, Error> + 'a,
    {
        self.credentials = Some(Box::new(cb) as Box<Credentials<'a>>);
        self
    }

    /// The callback through which progress is monitored.
    pub fn transfer_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(Progress<'_>) -> bool + 'a,
    {
        self.progress = Some(Box::new(cb) as Box<IndexerProgress<'a>>);
        self
    }

    /// Textual progress from the remote.
    ///
    /// Text sent over the progress side-band will be passed to this function
    /// (this is the 'counting objects' output).
    pub fn sideband_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(&[u8]) -> bool + 'a,
    {
        self.sideband_progress = Some(Box::new(cb) as Box<TransportMessage<'a>>);
        self
    }

    /// Each time a reference is updated locally, the callback will be called
    /// with information about it.
    pub fn update_tips<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(&str, Oid, Oid) -> bool + 'a,
    {
        self.update_tips = Some(Box::new(cb) as Box<UpdateTips<'a>>);
        self
    }

    /// If certificate verification fails, then this callback will be invoked to
    /// let the caller make the final decision of whether to allow the
    /// connection to proceed.
    pub fn certificate_check<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(&Cert<'_>, &str) -> bool + 'a,
    {
        self.certificate_check = Some(Box::new(cb) as Box<CertificateCheck<'a>>);
        self
    }

    /// Set a callback to get invoked for each updated reference on a push.
    ///
    /// The first argument to the callback is the name of the reference and the
    /// second is a status message sent by the server. If the status is `Some`
    /// then the push was rejected.
    pub fn push_update_reference<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(&str, Option<&str>) -> Result<(), Error> + 'a,
    {
        self.push_update_reference = Some(Box::new(cb) as Box<PushUpdateReference<'a>>);
        self
    }
}

impl<'a> Binding for RemoteCallbacks<'a> {
    type Raw = raw::git_remote_callbacks;
    unsafe fn from_raw(_raw: raw::git_remote_callbacks) -> RemoteCallbacks<'a> {
        panic!("unimplemented");
    }

    fn raw(&self) -> raw::git_remote_callbacks {
        unsafe {
            let mut callbacks: raw::git_remote_callbacks = mem::zeroed();
            assert_eq!(
                raw::git_remote_init_callbacks(&mut callbacks, raw::GIT_REMOTE_CALLBACKS_VERSION),
                0
            );
            if self.progress.is_some() {
                let f: raw::git_indexer_progress_cb = transfer_progress_cb;
                callbacks.transfer_progress = Some(f);
            }
            if self.credentials.is_some() {
                let f: raw::git_cred_acquire_cb = credentials_cb;
                callbacks.credentials = Some(f);
            }
            if self.sideband_progress.is_some() {
                let f: raw::git_transport_message_cb = sideband_progress_cb;
                callbacks.sideband_progress = Some(f);
            }
            if self.certificate_check.is_some() {
                let f: raw::git_transport_certificate_check_cb = certificate_check_cb;
                callbacks.certificate_check = Some(f);
            }
            if self.push_update_reference.is_some() {
                let f: extern "C" fn(_, _, _) -> c_int = push_update_reference_cb;
                callbacks.push_update_reference = Some(f);
            }
            if self.update_tips.is_some() {
                let f: extern "C" fn(
                    *const c_char,
                    *const raw::git_oid,
                    *const raw::git_oid,
                    *mut c_void,
                ) -> c_int = update_tips_cb;
                callbacks.update_tips = Some(f);
            }
            callbacks.payload = self as *const _ as *mut _;
            callbacks
        }
    }
}

extern "C" fn credentials_cb(
    ret: *mut *mut raw::git_cred,
    url: *const c_char,
    username_from_url: *const c_char,
    allowed_types: c_uint,
    payload: *mut c_void,
) -> c_int {
    unsafe {
        let ok = panic::wrap(|| {
            let payload = &mut *(payload as *mut RemoteCallbacks<'_>);
            let callback = payload
                .credentials
                .as_mut()
                .ok_or(raw::GIT_PASSTHROUGH as c_int)?;
            *ret = ptr::null_mut();
            let url = str::from_utf8(CStr::from_ptr(url).to_bytes())
                .map_err(|_| raw::GIT_PASSTHROUGH as c_int)?;
            let username_from_url = match crate::opt_bytes(&url, username_from_url) {
                Some(username) => {
                    Some(str::from_utf8(username).map_err(|_| raw::GIT_PASSTHROUGH as c_int)?)
                }
                None => None,
            };

            let cred_type = CredentialType::from_bits_truncate(allowed_types as u32);

            callback(url, username_from_url, cred_type).map_err(|e| {
                let s = CString::new(e.to_string()).unwrap();
                raw::git_error_set_str(e.raw_code() as c_int, s.as_ptr());
                e.raw_code() as c_int
            })
        });
        match ok {
            Some(Ok(cred)) => {
                // Turns out it's a memory safety issue if we pass through any
                // and all credentials into libgit2
                if allowed_types & (cred.credtype() as c_uint) != 0 {
                    *ret = cred.unwrap();
                    0
                } else {
                    raw::GIT_PASSTHROUGH as c_int
                }
            }
            Some(Err(e)) => e,
            None => -1,
        }
    }
}

extern "C" fn transfer_progress_cb(
    stats: *const raw::git_indexer_progress,
    payload: *mut c_void,
) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(payload as *mut RemoteCallbacks<'_>);
        let callback = match payload.progress {
            Some(ref mut c) => c,
            None => return true,
        };
        let progress = Binding::from_raw(stats);
        callback(progress)
    });
    if ok == Some(true) {
        0
    } else {
        -1
    }
}

extern "C" fn sideband_progress_cb(str: *const c_char, len: c_int, payload: *mut c_void) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(payload as *mut RemoteCallbacks<'_>);
        let callback = match payload.sideband_progress {
            Some(ref mut c) => c,
            None => return true,
        };
        let buf = slice::from_raw_parts(str as *const u8, len as usize);
        callback(buf)
    });
    if ok == Some(true) {
        0
    } else {
        -1
    }
}

extern "C" fn update_tips_cb(
    refname: *const c_char,
    a: *const raw::git_oid,
    b: *const raw::git_oid,
    data: *mut c_void,
) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks<'_>);
        let callback = match payload.update_tips {
            Some(ref mut c) => c,
            None => return true,
        };
        let refname = str::from_utf8(CStr::from_ptr(refname).to_bytes()).unwrap();
        let a = Binding::from_raw(a);
        let b = Binding::from_raw(b);
        callback(refname, a, b)
    });
    if ok == Some(true) {
        0
    } else {
        -1
    }
}

extern "C" fn certificate_check_cb(
    cert: *mut raw::git_cert,
    _valid: c_int,
    hostname: *const c_char,
    data: *mut c_void,
) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks<'_>);
        let callback = match payload.certificate_check {
            Some(ref mut c) => c,
            None => return true,
        };
        let cert = Binding::from_raw(cert);
        let hostname = str::from_utf8(CStr::from_ptr(hostname).to_bytes()).unwrap();
        callback(&cert, hostname)
    });
    if ok == Some(true) {
        0
    } else {
        -1
    }
}

extern "C" fn push_update_reference_cb(
    refname: *const c_char,
    status: *const c_char,
    data: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks<'_>);
        let callback = match payload.push_update_reference {
            Some(ref mut c) => c,
            None => return 0,
        };
        let refname = str::from_utf8(CStr::from_ptr(refname).to_bytes()).unwrap();
        let status = if status.is_null() {
            None
        } else {
            Some(str::from_utf8(CStr::from_ptr(status).to_bytes()).unwrap())
        };
        match callback(refname, status) {
            Ok(()) => 0,
            Err(e) => e.raw_code(),
        }
    })
    .unwrap_or(-1)
}
