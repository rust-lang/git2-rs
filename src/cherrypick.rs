use std::mem;

use raw;

/// Options to specify when cherry picking
pub struct CherrypickOptions {
    raw_opts: raw::git_cherrypick_options
}

impl CherrypickOptions {
    /// Creates a default set of cherrypick options
    pub fn new() -> CherrypickOptions {
        let mut opts = CherrypickOptions {
            raw_opts: unsafe { mem::zeroed() }
        };
        assert_eq!(unsafe {
            raw::git_cherrypick_init_options(&mut opts.raw_opts, 1)
        }, 0);
        opts
    }

    /// Acquire a pointer to the underlying raw options
    pub unsafe fn raw(&self) -> *const raw::git_cherrypick_options {
        &self.raw_opts as *const _
    }
}
