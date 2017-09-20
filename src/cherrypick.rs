use std::mem;

use build::CheckoutBuilder;
use merge::MergeOptions;
use raw;
use std::ptr;

/// Options to specify when cherry picking
pub struct CherrypickOptions<'cb> {
    mainline: u32,
    checkout_builder: Option<CheckoutBuilder<'cb>>,
    merge_opts: Option<MergeOptions>
}

impl<'cb> CherrypickOptions<'cb> {
    /// Creates a default set of cherrypick options
    pub fn new() -> CherrypickOptions<'cb> {
        let opts = CherrypickOptions {
            mainline: 1,
            checkout_builder: None,
            merge_opts: None
        };
        opts
    }

    /// Set the checkout builder
    pub fn checkout_builder(&mut self, cb: CheckoutBuilder<'cb>) -> &mut Self {
        self.checkout_builder = Some(cb);
        self
    }

    /// Set the merge options
    pub fn merge_opts(&mut self, merge_opts: MergeOptions) -> &mut Self {
        self.merge_opts = Some(merge_opts);
        self
    }

    /// Obtain the raw pointer
    pub fn raw(&mut self) -> *const raw::git_cherrypick_options {
        unsafe {
            let mut checkout_opts: raw::git_checkout_options = mem::zeroed();
            raw::git_checkout_init_options(&mut checkout_opts, raw::GIT_CHECKOUT_OPTIONS_VERSION);
            if let Some(ref mut cb) = self.checkout_builder {
                cb.configure(&mut checkout_opts);
            }
            let mut merge_opts = mem::zeroed();
            if let Some(ref opts) = self.merge_opts {
                ptr::copy(opts.raw(), &mut merge_opts, 1);
            }
            let opts = raw::git_cherrypick_options {
                version: 1,
                mainline: self.mainline,
                checkout_opts: checkout_opts,
                merge_opts: merge_opts
            };
            &opts as * const _
        }
    }
}
