use libc::{c_char, c_int, c_uint, c_void, size_t};
use std::ffi::{CStr, CString};
use std::mem;
use std::ptr;
use std::slice;
use std::str;

use crate::cert::Cert;
use crate::cred::CredInner;
use crate::util::Binding;
use crate::{
    panic, raw, Cred, CredentialType, Error, IndexerProgress, Oid, PackBuilderStage, Progress,
};

/// A structure to contain the callbacks which are invoked when a repository is
/// being updated or downloaded.
///
/// These callbacks are used to manage facilities such as authentication,
/// transfer progress, etc.
pub struct RemoteCallbacks<'a> {
    push_progress: Option<Box<PushTransferProgress<'a>>>,
    progress: Option<Box<IndexerProgress<'a>>>,
    pack_progress: Option<Box<PackProgress<'a>>>,
    credentials: Option<Box<Credentials<'a>>>,
    sideband_progress: Option<Box<TransportMessage<'a>>>,
    update_tips: Option<Box<UpdateTips<'a>>>,
    certificate_check: Option<Box<CertificateCheck<'a>>>,
    push_update_reference: Option<Box<PushUpdateReference<'a>>>,

    #[cfg(feature = "ssh")]
    ssh_interactive: Option<Box<SshInteractiveCallback<'a>>>,
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

/// Callback for push transfer progress
///
/// Parameters:
///     * current
///     * total
///     * bytes
pub type PushTransferProgress<'a> = dyn FnMut(usize, usize, usize) + 'a;

/// Callback for pack progress
///
/// Parameters:
///     * stage
///     * current
///     * total
pub type PackProgress<'a> = dyn FnMut(PackBuilderStage, usize, usize) + 'a;

#[cfg(feature = "ssh")]
/// Callback for push transfer progress
///
/// Parameters:
///     * name
///     * instruction
///     * prompts
///     * responses
pub type SshInteractiveCallback<'a> =
    dyn FnMut(&str, &str, &[crate::cred::SshInteractivePrompt<'a>], &mut [String]) + 'a;

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
            pack_progress: None,
            sideband_progress: None,
            update_tips: None,
            certificate_check: None,
            push_update_reference: None,
            push_progress: None,

            #[cfg(feature = "ssh")]
            ssh_interactive: None,
        }
    }

    /// The callback through which to fetch credentials if required.
    ///
    /// # Example
    ///
    /// Prepare a callback to authenticate using the `$HOME/.ssh/id_rsa` SSH key, and
    /// extracting the username from the URL (i.e. git@github.com:rust-lang/git2-rs.git):
    ///
    /// ```no_run
    /// use git2::{Cred, RemoteCallbacks};
    /// use std::env;
    ///
    /// let mut callbacks = RemoteCallbacks::new();
    /// callbacks.credentials(|_url, username_from_url, _allowed_types| {
    ///   Cred::ssh_key(
    ///     username_from_url.unwrap(),
    ///     None,
    ///     std::path::Path::new(&format!("{}/.ssh/id_rsa", env::var("HOME").unwrap())),
    ///     None,
    ///   )
    /// });
    /// ```
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

    /// The callback through which progress of push transfer is monitored
    pub fn push_transfer_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(usize, usize, usize) + 'a,
    {
        self.push_progress = Some(Box::new(cb) as Box<PushTransferProgress<'a>>);
        self
    }

    /// Function to call with progress information during pack building.
    /// Be aware that this is called inline with pack building operations,
    /// so performance may be affected.
    pub fn pack_progress<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(PackBuilderStage, usize, usize) + 'a,
    {
        self.pack_progress = Some(Box::new(cb) as Box<PackProgress<'a>>);
        self
    }

    #[cfg(any(doc, feature = "ssh"))]
    /// Function to call with SSH interactive prompts to write the responses
    /// into the given mutable [String] slice
    ///
    /// Callback parameters:
    /// - name
    /// - instruction
    /// - prompts
    /// - responses
    pub fn ssh_interactive<F>(&mut self, cb: F) -> &mut RemoteCallbacks<'a>
    where
        F: FnMut(&str, &str, &[crate::cred::SshInteractivePrompt<'a>], &mut [String]) + 'a,
    {
        self.ssh_interactive = Some(Box::new(cb) as Box<SshInteractiveCallback<'a>>);
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
                callbacks.transfer_progress = Some(transfer_progress_cb);
            }
            if self.credentials.is_some() {
                callbacks.credentials = Some(credentials_cb);
            }
            if self.sideband_progress.is_some() {
                callbacks.sideband_progress = Some(sideband_progress_cb);
            }
            if self.certificate_check.is_some() {
                callbacks.certificate_check = Some(certificate_check_cb);
            }
            if self.push_update_reference.is_some() {
                callbacks.push_update_reference = Some(push_update_reference_cb);
            }
            if self.push_progress.is_some() {
                callbacks.push_transfer_progress = Some(push_transfer_progress_cb);
            }
            if self.pack_progress.is_some() {
                callbacks.pack_progress = Some(pack_progress_cb);
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
    c_payload: *mut c_void,
) -> c_int {
    unsafe {
        let ok = panic::wrap(|| {
            let payload = &mut *(c_payload as *mut RemoteCallbacks<'_>);
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

            callback(url, username_from_url, cred_type)
                .and_then(|cred| match cred.unwrap_inner() {
                    CredInner::Cred(raw) => Ok(Cred::from_raw(raw)),

                    #[cfg(feature = "ssh")]
                    CredInner::Interactive { username } => {
                        let username = CString::new(username)?;
                        let mut out = ptr::null_mut();
                        try_call!(raw::git_cred_ssh_interactive_new(
                            &mut out,
                            username,
                            Some(ssh_interactive_cb as _),
                            c_payload
                        ));

                        Ok(Cred::from_raw(out))
                    }
                })
                .map_err(|e| {
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

extern "C" fn push_transfer_progress_cb(
    progress: c_uint,
    total: c_uint,
    bytes: size_t,
    data: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks<'_>);
        let callback = match payload.push_progress {
            Some(ref mut c) => c,
            None => return 0,
        };

        callback(progress as usize, total as usize, bytes as usize);

        0
    })
    .unwrap_or(-1)
}

extern "C" fn pack_progress_cb(
    stage: raw::git_packbuilder_stage_t,
    current: c_uint,
    total: c_uint,
    data: *mut c_void,
) -> c_int {
    panic::wrap(|| unsafe {
        let payload = &mut *(data as *mut RemoteCallbacks<'_>);
        let callback = match payload.pack_progress {
            Some(ref mut c) => c,
            None => return 0,
        };

        let stage = Binding::from_raw(stage);

        callback(stage, current as usize, total as usize);

        0
    })
    .unwrap_or(-1)
}

#[cfg(feature = "ssh")]
extern "C" fn ssh_interactive_cb(
    name: *const c_char,
    name_len: c_int,
    instruction: *const c_char,
    instruction_len: c_int,
    num_prompts: c_int,
    prompts: *const raw::LIBSSH2_USERAUTH_KBDINT_PROMPT,
    responses: *mut raw::LIBSSH2_USERAUTH_KBDINT_RESPONSE,
    payload: *mut *mut c_void,
) {
    use libc::malloc;

    panic::wrap(|| unsafe {
        let prompts = prompts as *const libssh2_sys::LIBSSH2_USERAUTH_KBDINT_PROMPT;
        let responses = responses as *mut libssh2_sys::LIBSSH2_USERAUTH_KBDINT_RESPONSE;

        let name =
            String::from_utf8_lossy(slice::from_raw_parts(name as *const u8, name_len as usize));
        let instruction = String::from_utf8_lossy(slice::from_raw_parts(
            instruction as *const u8,
            instruction_len as usize,
        ));

        let mut wrapped_prompts = Vec::with_capacity(num_prompts as usize);
        for i in 0..num_prompts {
            let prompt = &*prompts.offset(i as isize);
            wrapped_prompts.push(crate::cred::SshInteractivePrompt {
                text: String::from_utf8_lossy(slice::from_raw_parts(
                    prompt.text as *const u8,
                    prompt.length as usize,
                )),
                echo: prompt.echo != 0,
            });
        }

        let mut wrapped_responses = vec![String::new(); num_prompts as usize];

        let payload = &mut *(payload as *mut Box<RemoteCallbacks<'_>>);
        if let Some(callback) = &mut payload.ssh_interactive {
            callback(
                name.as_ref(),
                instruction.as_ref(),
                &wrapped_prompts[..],
                &mut wrapped_responses[..],
            );
        }

        for i in 0..num_prompts {
            let response = &mut *responses.offset(i as isize);
            let response_bytes = wrapped_responses[i as usize].as_bytes();

            // libgit2 frees returned strings
            let text = malloc(response_bytes.len());
            response.text = text as *mut c_char;
            response.length = response_bytes.len() as u32;
        }
    });
}
