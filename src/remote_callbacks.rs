use std::c_str::CString;
use std::mem;
use std::slice;
use libc;

use {raw, Error, Cred, CredentialType};

/// A structure to contain the callbacks which are invoked when a repository is
/// being updated or downloaded.
///
/// These callbacks are used to manage facilities such as authentication,
/// transfer progress, etc.
pub struct RemoteCallbacks<'a> {
    progress: Option<TransferProgress<'a>>,
    credentials: Option<Credentials<'a>>,
    sideband_progress: Option<TransportMessage<'a>>,
}

/// Struct representing the progress by an in-flight transfer.
#[deriving(Copy)]
pub struct Progress {
    /// Number of objects in the packfile being downloaded
    pub total_objects: uint,
    /// Received objects that have been hashed
    pub indexed_objects: uint,
    /// Objects which have been downloaded
    pub received_objects: uint,
    /// Locally-available objects that have been injected in order to fix a thin
    /// pack.
    pub local_objects: uint,
    /// Number of deltas in the packfile being downloaded
    pub total_deltas: uint,
    /// Received deltas that have been hashed.
    pub indexed_deltas: uint,
    /// Size of the packfile received up to now
    pub received_bytes: uint,
}

/// Callback used to acquire credentials for when a remote is fetched.
///
/// * `url` - the resource for which the credentials are required.
/// * `username_from_url` - the username that was embedded in the url, or `None`
///                         if it was not included.
/// * `allowed_types` - a bitmask stating which cred types are ok to return.
pub type Credentials<'a> = |url: &str,
                            username_from_url: Option<&str>,
                            allowed_types: CredentialType|: 'a
                           -> Result<Cred, Error>;

/// Callback to be invoked while a transfer is in progress.
///
/// This callback will be periodically called with updates to the progress of
/// the transfer so far. The return value indicates whether the transfer should
/// continue. A return value of `false` will cancel the transfer.
///
/// * `progress` - the progress being made so far.
pub type TransferProgress<'a> = |progress: Progress|: 'a -> bool;

/// Callback for receiving messages delivered by the transport.
///
/// The return value indicates whether the network operation should continue.
pub type TransportMessage<'a> = |message: &[u8]|: 'a -> bool;

impl<'a> RemoteCallbacks<'a> {
    /// Creates a new set of empty callbacks
    pub fn new() -> RemoteCallbacks<'a> {
        RemoteCallbacks {
            credentials: None,
            progress: None,
            sideband_progress: None,
        }
    }

    /// The callback through which to fetch credentials if required.
    ///
    /// It is strongly recommended to audit the `credentials` callback for
    /// failure as it will likely leak resources if it fails.
    pub fn credentials(mut self, cb: Credentials<'a>) -> RemoteCallbacks<'a> {
        self.credentials = Some(cb);
        self
    }

    /// The callback through which progress is monitored.
    ///
    /// It is strongly recommended to audit the `progress` callback for
    /// failure as it will likely leak resources if it fails.
    pub fn transfer_progress(mut self, cb: TransferProgress<'a>)
                             -> RemoteCallbacks<'a> {
        self.progress = Some(cb);
        self
    }

    /// Textual progress from the remote.
    ///
    /// Text sent over the progress side-band will be passed to this function
    /// (this is the 'counting objects' output.
    pub fn sideband_progress(mut self, cb: TransportMessage<'a>)
                             -> RemoteCallbacks<'a> {
        self.sideband_progress = Some(cb);
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
            callbacks.transfer_progress = Some(transfer_progress_cb);
        }
        if self.credentials.is_some() {
            callbacks.credentials = Some(credentials_cb);
        }
        if self.sideband_progress.is_some() {
            callbacks.sideband_progress = Some(sideband_progress_cb);
        }
        callbacks.payload = self as *mut _ as *mut _;
        return callbacks;
    }
}

extern fn credentials_cb(ret: *mut *mut raw::git_cred,
                         url: *const libc::c_char,
                         username_from_url: *const libc::c_char,
                         allowed_types: libc::c_uint,
                         payload: *mut libc::c_void) -> libc::c_int {
    unsafe {
        let payload: &mut RemoteCallbacks = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.credentials {
            Some(ref mut c) => c,
            None => return raw::GIT_PASSTHROUGH as libc::c_int,
        };
        *ret = 0 as *mut raw::git_cred;
        let url = CString::new(url, false);
        let url = match url.as_str()  {
            Some(url) => url,
            None => return raw::GIT_PASSTHROUGH as libc::c_int,
        };
        let username_from_url = if username_from_url.is_null() {
            None
        } else {
            Some(CString::new(username_from_url, false))
        };
        let username_from_url = match username_from_url {
            Some(ref username) => match username.as_str() {
                Some(s) => Some(s),
                None => return raw::GIT_PASSTHROUGH as libc::c_int,
            },
            None => None,
        };

        let cred_type = CredentialType::from_bits_truncate(allowed_types as uint);
        match (*callback)(url, username_from_url, cred_type) {
            Ok(cred) => {
                // Turns out it's a memory safety issue if we pass through any
                // and all credentials into libgit2
                if allowed_types & (cred.credtype() as libc::c_uint) != 0 {
                    *ret = cred.unwrap();
                    0
                } else {
                    raw::GIT_PASSTHROUGH as libc::c_int
                }
            }
            Err(e) => e.raw_code() as libc::c_int,
        }
    }
}

extern fn transfer_progress_cb(stats: *const raw::git_transfer_progress,
                               payload: *mut libc::c_void) -> libc::c_int {
    unsafe {
        let payload: &mut RemoteCallbacks = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.progress {
            Some(ref mut c) => c,
            None => return 0,
        };
        let progress = Progress {
            total_objects: (*stats).total_objects as uint,
            indexed_objects: (*stats).indexed_objects as uint,
            received_objects: (*stats).received_objects as uint,
            local_objects: (*stats).local_objects as uint,
            total_deltas: (*stats).total_deltas as uint,
            indexed_deltas: (*stats).indexed_deltas as uint,
            received_bytes: (*stats).received_bytes as uint,
        };
        if (*callback)(progress) {0} else {-1}
    }
}

extern fn sideband_progress_cb(str: *const libc::c_char,
                               len: libc::c_int,
                               payload: *mut libc::c_void) -> libc::c_int {
    unsafe {
        let payload: &mut RemoteCallbacks = &mut *(payload as *mut RemoteCallbacks);
        let callback = match payload.sideband_progress {
            Some(ref mut c) => c,
            None => return 0,
        };
        let ptr = str as *const u8;
        let buf = slice::from_raw_buf(&ptr, len as uint);
        let r = (*callback)(buf);
        if r {0} else {-1}
    }
}
