use std::ffi::CString;
use std::marker;
use std::ptr;

use crate::raw;
use crate::util::Binding;

/// Options which can be specified to various fetch operations.
#[derive(Default)]
pub struct ProxyOptions<'a> {
    url: Option<CString>,
    proxy_kind: raw::git_proxy_t,
    _marker: marker::PhantomData<&'a i32>,
}

impl<'a> ProxyOptions<'a> {
    /// Creates a new set of proxy options ready to be configured.
    pub fn new() -> ProxyOptions<'a> {
        Default::default()
    }

    /// Try to auto-detect the proxy from the git configuration.
    ///
    /// Note that this will override `url` specified before.
    pub fn auto(&mut self) -> &mut Self {
        self.proxy_kind = raw::GIT_PROXY_AUTO;
        self
    }

    /// Specify the exact URL of the proxy to use.
    ///
    /// Note that this will override `auto` specified before.
    pub fn url(&mut self, url: &str) -> &mut Self {
        self.proxy_kind = raw::GIT_PROXY_SPECIFIED;
        self.url = Some(CString::new(url).unwrap());
        self
    }
}

impl<'a> Binding for ProxyOptions<'a> {
    type Raw = raw::git_proxy_options;
    unsafe fn from_raw(_raw: raw::git_proxy_options) -> ProxyOptions<'a> {
        panic!("can't create proxy from raw options")
    }

    fn raw(&self) -> raw::git_proxy_options {
        raw::git_proxy_options {
            version: raw::GIT_PROXY_OPTIONS_VERSION,
            kind: self.proxy_kind,
            url: self.url.as_ref().map(|s| s.as_ptr()).unwrap_or(ptr::null()),
            credentials: None,
            certificate_check: None,
            payload: ptr::null_mut(),
        }
    }
}
