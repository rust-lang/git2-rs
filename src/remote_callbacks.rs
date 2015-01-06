use std::c_str::CString;
use std::kinds::marker;
use std::mem;
use std::slice;
use libc::{c_void, c_int, c_char, c_uint};

use {raw, panic, Error, Cred, CredentialType, Oid};

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
}

/// Struct representing the progress by an in-flight transfer.
pub struct Progress<'a> {
    raw: ProgressState,
    marker: marker::ContravariantLifetime<'a>,
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
// FIXME: the second &str should be Option<&str> but that currently ICEs
pub type Credentials<'a> = FnMut(&str, &str, CredentialType)
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

impl<'a> RemoteCallbacks<'a> {
    /// Creates a new set of empty callbacks
    pub fn new() -> RemoteCallbacks<'a> {
        RemoteCallbacks {
            credentials: None,
            progress: None,
            sideband_progress: None,
            update_tips: None,
        }
    }

    /// The callback through which to fetch credentials if required.
    pub fn credentials<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                          where F: FnMut(&str, &str, CredentialType)
                                         -> Result<Cred, Error> + 'a
    {
        self.credentials = Some(box cb as Box<Credentials<'a>>);
        self
    }

    /// The callback through which progress is monitored.
    pub fn transfer_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                                where F: FnMut(Progress) -> bool + 'a {
        self.progress = Some(box cb as Box<TransferProgress<'a>>);
        self
    }

    /// Textual progress from the remote.
    ///
    /// Text sent over the progress side-band will be passed to this function
    /// (this is the 'counting objects' output.
    pub fn sideband_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                                where F: FnMut(&[u8]) -> bool + 'a {
        self.sideband_progress = Some(box cb as Box<TransportMessage<'a>>);
        self
    }

    /// Each time a reference is updated locally, the callback will be called
    /// with information about it.
    pub fn update_tips<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
                          where F: FnMut(&str, Oid, Oid) -> bool + 'a {
        self.update_tips = Some(box cb as Box<UpdateTips<'a>>);
        self
    }

    /// Convert this set of callbacks to a raw callbacks structure.
    ///
    /// This function is unsafe as the callbacks returned have a reference to
    /// this object and are only valid while the object is alive.
    pub unsafe fn raw(&mut self) -> raw::git_remote_callbacks {
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
        if self.update_tips.is_some() {
            let f: extern fn(*const c_char, *const raw::git_oid,
                             *const raw::git_oid, *mut c_void) -> c_int
                            = update_tips_cb;
            callbacks.update_tips = Some(f);
        }
        callbacks.payload = self as *mut _ as *mut _;
        return callbacks;
    }
}

impl<'a> Progress<'a> {
    /// Creates a new progress structure from its raw counterpart.
    ///
    /// This function is unsafe as there is no anchor for the returned lifetime
    /// and the validity of the pointer cannot be guaranteed.
    pub unsafe fn from_raw(raw: *const raw::git_transfer_progress)
                           -> Progress<'a> {
        Progress {
            raw: ProgressState::Borrowed(raw),
            marker: marker::ContravariantLifetime,
        }
    }

    /// Number of objects in the packfile being downloaded
    pub fn total_objects(&self) -> uint {
        unsafe { (*self.raw()).total_objects as uint }
    }
    /// Received objects that have been hashed
    pub fn indexed_objects(&self) -> uint {
        unsafe { (*self.raw()).indexed_objects as uint }
    }
    /// Objects which have been downloaded
    pub fn received_objects(&self) -> uint {
        unsafe { (*self.raw()).received_objects as uint }
    }
    /// Locally-available objects that have been injected in order to fix a thin
    /// pack.
    pub fn local_objects(&self) -> uint {
        unsafe { (*self.raw()).local_objects as uint }
    }
    /// Number of deltas in the packfile being downloaded
    pub fn total_deltas(&self) -> uint {
        unsafe { (*self.raw()).total_deltas as uint }
    }
    /// Received deltas that have been hashed.
    pub fn indexed_deltas(&self) -> uint {
        unsafe { (*self.raw()).indexed_deltas as uint }
    }
    /// Size of the packfile received up to now
    pub fn received_bytes(&self) -> uint {
        unsafe { (*self.raw()).received_bytes as uint }
    }

    /// Convert this to an owned version of `Progress`.
    pub fn to_owned(&self) -> Progress<'static> {
        Progress {
            raw: ProgressState::Owned(unsafe { *self.raw() }),
            marker: marker::ContravariantLifetime,
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
        let payload: &mut RemoteCallbacks = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.credentials {
            Some(ref mut c) => c,
            None => return raw::GIT_PASSTHROUGH as c_int,
        };
        *ret = 0 as *mut raw::git_cred;
        let url = CString::new(url, false);
        let url = match url.as_str()  {
            Some(url) => url,
            None => return raw::GIT_PASSTHROUGH as c_int,
        };
        let username_from_url = if username_from_url.is_null() {
            None
        } else {
            Some(CString::new(username_from_url, false))
        };
        let username_from_url = match username_from_url {
            Some(ref username) => match username.as_str() {
                Some(s) => Some(s),
                None => return raw::GIT_PASSTHROUGH as c_int,
            },
            None => None,
        };
        let username_from_url = username_from_url.unwrap_or("");

        let cred_type = CredentialType::from_bits_truncate(allowed_types as uint);
        match panic::wrap(|| {
            callback(url, username_from_url, cred_type)
        }) {
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
            Some(Err(e)) => e.raw_code() as c_int,
            None => -1,
        }
    }
}

extern fn transfer_progress_cb(stats: *const raw::git_transfer_progress,
                               payload: *mut c_void) -> c_int {
    unsafe {
        let payload: &mut RemoteCallbacks = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.progress {
            Some(ref mut c) => c,
            None => return 0,
        };
        let progress = Progress::from_raw(stats);
        let ok = panic::wrap(move || {
            callback(progress)
        }).unwrap_or(false);
        if ok {0} else {-1}
    }
}

extern fn sideband_progress_cb(str: *const c_char,
                               len: c_int,
                               payload: *mut c_void) -> c_int {
    unsafe {
        let payload: &mut RemoteCallbacks = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.sideband_progress {
            Some(ref mut c) => c,
            None => return 0,
        };
        let ptr = str as *const u8;
        let buf = slice::from_raw_buf(&ptr, len as uint);
        let ok = panic::wrap(|| {
            callback(buf)
        }).unwrap_or(false);
        if ok {0} else {-1}
    }
}

extern fn update_tips_cb(refname: *const c_char,
                         a: *const raw::git_oid,
                         b: *const raw::git_oid,
                         data: *mut c_void) -> c_int {
    unsafe {
        let payload: &mut RemoteCallbacks = &mut *(data as *mut RemoteCallbacks);
        let callback = match payload.update_tips {
            Some(ref mut c) => c,
            None => return 0,
        };
        let refname = CString::new(refname, false);
        let refname = refname.as_str().unwrap();
        let a = Oid::from_raw(a);
        let b = Oid::from_raw(b);
        let ok = panic::wrap(|| {
            callback(refname, a, b)
        }).unwrap_or(false);
        if ok {0} else {-1}
    }
}
