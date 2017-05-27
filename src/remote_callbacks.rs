use std::ffi::CStr;
use std::marker;
use std::mem;
use std::slice;
use std::ptr;
use std::str;
use libc::{c_void, c_int, c_char, c_uint};

use {raw, panic, Error, Cred, CredentialType, Oid};
use cert::Cert;
use util::Binding;

/// A structure to contain the callbacks which are invoked when a repository is
/// being updated or downloaded.
///
/// These callbacks are used to manage facilities such as authentication,
/// transfer progress, etc.
pub struct RemoteCallbacks<'a> {
    progress: Option<Box<TransferProgress<'a>>>,
    credentials: Option<Box<Credentials<'a>>>,
    sideband_progress: Option<Box<TransportMessage<'a>>>,
    update_tips: Option<Box<UpdateTips<'a>>>,
    certificate_check: Option<Box<CertificateCheck<'a>>>,
    push_update_reference: Option<Box<PushUpdateReference<'a>>>,
}

/// Struct representing the progress by an in-flight transfer.
pub struct Progress<'a> {
    raw: ProgressState,
    _marker: marker::PhantomData<&'a raw::git_transfer_progress>,
}

enum ProgressState {
    Borrowed(*const raw::git_transfer_progress),
    Owned(raw::git_transfer_progress),
}

/// Callback used to acquire credentials for when a remote is fetched.
///
/// * `url` - the resource for which the credentials are required.
/// * `username_from_url` - the username that was embedded in the url, or `None`
///                         if it was not included.
/// * `allowed_types` - a bitmask stating which cred types are ok to return.
pub type Credentials<'a> = FnMut(&str, Option<&str>, CredentialType)
                                 -> Result<Cred, Error> + 'a;

/// Callback to be invoked while a transfer is in progress.
///
/// This callback will be periodically called with updates to the progress of
/// the transfer so far. The return value indicates whether the transfer should
/// continue. A return value of `false` will cancel the transfer.
///
/// * `progress` - the progress being made so far.
pub type TransferProgress<'a> = FnMut(Progress) -> bool + 'a;

/// Callback for receiving messages delivered by the transport.
///
/// The return value indicates whether the network operation should continue.
pub type TransportMessage<'a> = FnMut(&[u8]) -> bool + 'a;

/// Callback for whenever a reference is updated locally.
pub type UpdateTips<'a> = FnMut(&str, Oid, Oid) -> bool + 'a;

/// Callback for a custom certificate check.
///
/// The first argument is the certificate receved on the connection.
/// Certificates are typically either an SSH or X509 certificate.
///
/// The second argument is the hostname for the connection is passed as the last
/// argument.
pub type CertificateCheck<'a> = FnMut(&Cert, &str) -> bool + 'a;

/// Callback for each updated reference on push.
///
/// The first argument here is the `refname` of the reference, and the second is
/// the status message sent by a server. If the status is `Some` then the update
/// was rejected by the remote server with a reason why.
pub type PushUpdateReference<'a> = FnMut(&str, Option<&str>) -> Result<(), Error> + 'a;

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
                          where F: FnMut(&str, Option<&str>, CredentialType)
                                         -> Result<Cred, Error> + 'a
    {
        self.credentials = Some(Box::new(cb) as Box<Credentials<'a>>);
        self
    }

    /// The callback through which progress is monitored.
    pub fn transfer_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                                where F: FnMut(Progress) -> bool + 'a {
        self.progress = Some(Box::new(cb) as Box<TransferProgress<'a>>);
        self
    }

    /// Textual progress from the remote.
    ///
    /// Text sent over the progress side-band will be passed to this function
    /// (this is the 'counting objects' output.
    pub fn sideband_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                                where F: FnMut(&[u8]) -> bool + 'a {
        self.sideband_progress = Some(Box::new(cb) as Box<TransportMessage<'a>>);
        self
    }

    /// Each time a reference is updated locally, the callback will be called
    /// with information about it.
    pub fn update_tips<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                          where F: FnMut(&str, Oid, Oid) -> bool + 'a {
        self.update_tips = Some(Box::new(cb) as Box<UpdateTips<'a>>);
        self
    }

    /// If certificate verification fails, then this callback will be invoked to
    /// let the caller make the final decision of whether to allow the
    /// connection to proceed.
    pub fn certificate_check<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
        where F: FnMut(&Cert, &str) -> bool + 'a
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
        where F: FnMut(&str, Option<&str>) -> Result<(), Error> + 'a,
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
            assert_eq!(raw::git_remote_init_callbacks(&mut callbacks,
                                        raw::GIT_REMOTE_CALLBACKS_VERSION), 0);
            if self.progress.is_some() {
                let f: raw::git_transfer_progress_cb = transfer_progress_cb;
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
                let f: raw::git_transport_certificate_check_cb =
                        certificate_check_cb;
                callbacks.certificate_check = Some(f);
            }
            if self.push_update_reference.is_some() {
                let f: extern fn(_, _, _) -> c_int = push_update_reference_cb;
                callbacks.push_update_reference = Some(f);
            }
            if self.update_tips.is_some() {
                let f: extern fn(*const c_char, *const raw::git_oid,
                                 *const raw::git_oid, *mut c_void) -> c_int
                                = update_tips_cb;
                callbacks.update_tips = Some(f);
            }
            callbacks.payload = self as *const _ as *mut _;
            return callbacks;
        }
    }
}

impl<'a> Progress<'a> {
    /// Number of objects in the packfile being downloaded
    pub fn total_objects(&self) -> usize {
        unsafe { (*self.raw()).total_objects as usize }
    }
    /// Received objects that have been hashed
    pub fn indexed_objects(&self) -> usize {
        unsafe { (*self.raw()).indexed_objects as usize }
    }
    /// Objects which have been downloaded
    pub fn received_objects(&self) -> usize {
        unsafe { (*self.raw()).received_objects as usize }
    }
    /// Locally-available objects that have been injected in order to fix a thin
    /// pack.
    pub fn local_objects(&self) -> usize {
        unsafe { (*self.raw()).local_objects as usize }
    }
    /// Number of deltas in the packfile being downloaded
    pub fn total_deltas(&self) -> usize {
        unsafe { (*self.raw()).total_deltas as usize }
    }
    /// Received deltas that have been hashed.
    pub fn indexed_deltas(&self) -> usize {
        unsafe { (*self.raw()).indexed_deltas as usize }
    }
    /// Size of the packfile received up to now
    pub fn received_bytes(&self) -> usize {
        unsafe { (*self.raw()).received_bytes as usize }
    }

    /// Convert this to an owned version of `Progress`.
    pub fn to_owned(&self) -> Progress<'static> {
        Progress {
            raw: ProgressState::Owned(unsafe { *self.raw() }),
            _marker: marker::PhantomData,
        }
    }
}

impl<'a> Binding for Progress<'a> {
    type Raw = *const raw::git_transfer_progress;
    unsafe fn from_raw(raw: *const raw::git_transfer_progress)
                           -> Progress<'a> {
        Progress {
            raw: ProgressState::Borrowed(raw),
            _marker: marker::PhantomData,
        }
    }

    fn raw(&self) -> *const raw::git_transfer_progress {
        match self.raw {
            ProgressState::Borrowed(raw) => raw,
            ProgressState::Owned(ref raw) => raw as *const _,
        }
    }
}

extern fn credentials_cb(ret: *mut *mut raw::git_cred,
                         url: *const c_char,
                         username_from_url: *const c_char,
                         allowed_types: c_uint,
                         payload: *mut c_void) -> c_int {
    unsafe {
        let ok = panic::wrap(|| {
            let payload = &mut *(payload as *mut RemoteCallbacks);
            let callback = try!(payload.credentials.as_mut()
                                       .ok_or(raw::GIT_PASSTHROUGH as c_int));
            *ret = ptr::null_mut();
            let url = try!(str::from_utf8(CStr::from_ptr(url).to_bytes())
                              .map_err(|_| raw::GIT_PASSTHROUGH as c_int));
            let username_from_url = match ::opt_bytes(&url, username_from_url) {
                Some(username) => {
                    Some(try!(str::from_utf8(username)
                                 .map_err(|_| raw::GIT_PASSTHROUGH as c_int)))
                }
                None => None,
            };

            let cred_type = CredentialType::from_bits_truncate(allowed_types as u32);

            callback(url, username_from_url, cred_type).map_err(|e| {
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

extern fn transfer_progress_cb(stats: *const raw::git_transfer_progress,
                               payload: *mut c_void) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.progress {
            Some(ref mut c) => c,
            None => return true,
        };
        let progress = Binding::from_raw(stats);
        callback(progress)
    });
    if ok == Some(true) {0} else {-1}
}

extern fn sideband_progress_cb(str: *const c_char,
                               len: c_int,
                               payload: *mut c_void) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.sideband_progress {
            Some(ref mut c) => c,
            None => return true,
        };
        let buf = slice::from_raw_parts(str as *const u8, len as usize);
        callback(buf)
    });
    if ok == Some(true) {0} else {-1}
}

extern fn update_tips_cb(refname: *const c_char,
                         a: *const raw::git_oid,
                         b: *const raw::git_oid,
                         data: *mut c_void) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks);
        let callback = match payload.update_tips {
            Some(ref mut c) => c,
            None => return true,
        };
        let refname = str::from_utf8(CStr::from_ptr(refname).to_bytes())
                          .unwrap();
        let a = Binding::from_raw(a);
        let b = Binding::from_raw(b);
        callback(refname, a, b)
    });
    if ok == Some(true) {0} else {-1}
}

extern fn certificate_check_cb(cert: *mut raw::git_cert,
                               _valid: c_int,
                               hostname: *const c_char,
                               data: *mut c_void) -> c_int {
    let ok = panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks);
        let callback = match payload.certificate_check {
            Some(ref mut c) => c,
            None => return true,
        };
        let cert = Binding::from_raw(cert);
        let hostname = str::from_utf8(CStr::from_ptr(hostname).to_bytes())
                           .unwrap();
        callback(&cert, hostname)
    });
    if ok == Some(true) {0} else {-1}
}

extern fn push_update_reference_cb(refname: *const c_char,
                                   status: *const c_char,
                                   data: *mut c_void) -> c_int {
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks);
        let callback = match payload.push_update_reference {
            Some(ref mut c) => c,
            None => return 0,
        };
        let refname = str::from_utf8(CStr::from_ptr(refname).to_bytes())
                           .unwrap();
        let status = if status.is_null() {
            None
        } else {
            Some(str::from_utf8(CStr::from_ptr(status).to_bytes()).unwrap())
        };
        match callback(refname, status) {
            Ok(()) => 0,
            Err(e) => e.raw_code(),
        }
    }).unwrap_or(-1)
}
