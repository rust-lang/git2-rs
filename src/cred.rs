use std::mem;

use {raw, Error};

/// A structure to represent git credentials in libgit2.
pub struct Cred {
    raw: *mut raw::git_cred,
    credtype: raw::git_credtype_t,
}

impl Cred {
    /// Create a new credential object from its raw component.
    ///
    /// This method is unsafe as there is no guarantee that `raw` is a valid
    /// pointer.
    pub unsafe fn from_raw(raw: *mut raw::git_cred,
                           credtype: raw::git_credtype_t) -> Cred {
        Cred { raw: raw, credtype: credtype }
    }

    /// Create a "default" credential usable for Negotiate mechanisms like NTLM
    /// or Kerberos authentication.
    pub fn default() -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_default_new(&mut out));
            Ok(Cred::from_raw(out, raw::GIT_CREDTYPE_DEFAULT))
        }
    }

    /// Create a new ssh key credential object used for querying an ssh-agent.
    ///
    /// The username specified is the username to authenticate.
    pub fn ssh_key_from_agent(username: &str) -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_ssh_key_from_agent(&mut out,
                                                       username.to_c_str()));
            Ok(Cred::from_raw(out, raw::GIT_CREDTYPE_SSH_KEY))
        }
    }

    /// Create a new passphrase-protected ssh key credential object.
    pub fn ssh_key(username: &str,
                   publickey: Option<&Path>,
                   privatekey: &Path,
                   passphrase: Option<&str>) -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_ssh_key_new(&mut out,
                                                username.to_c_str(),
                                                publickey.map(|s| s.to_c_str()),
                                                privatekey.to_c_str(),
                                                passphrase.map(|s| s.to_c_str())));
            Ok(Cred::from_raw(out, raw::GIT_CREDTYPE_SSH_KEY))
        }
    }

    /// Create a new plain-text username and password credential object.
    pub fn userpass_plaintext(username: &str,
                              password: &str) -> Result<Cred, Error> {
        ::init();
        let mut out = 0 as *mut raw::git_cred;
        unsafe {
            try_call!(raw::git_cred_userpass_plaintext_new(&mut out,
                                                           username.to_c_str(),
                                                           password.to_c_str()));
            Ok(Cred::from_raw(out, raw::GIT_CREDTYPE_USERPASS_PLAINTEXT))
        }
    }

    /// Check whether a credential object contains username information.
    pub fn has_username(&self) -> bool {
        unsafe { raw::git_cred_has_username(self.raw) == 1 }
    }

    /// Gain access to the underlying raw credential pointer.
    pub fn raw(&self) -> *mut raw::git_cred { self.raw }

    /// Return the type of credentials that this object represents.
    pub fn credtype(&self) -> raw::git_credtype_t { self.credtype }

    /// Unwrap access to the underlying raw pointer, canceling the destructor
    pub unsafe fn unwrap(mut self) -> *mut raw::git_cred {
        mem::replace(&mut self.raw, 0 as *mut raw::git_cred)
    }
}

impl Drop for Cred {
    fn drop(&mut self) {
        if !self.raw.is_null() {
            unsafe { ((*self.raw).free)(self.raw) }
        }
    }
}

#[cfg(test)]
mod test {
    use super::Cred;

    #[test]
    fn smoke() {
        Cred::default().unwrap();
    }
}
