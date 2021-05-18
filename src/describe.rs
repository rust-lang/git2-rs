use std::ffi::CString;
use std::marker;
use std::mem;
use std::ptr;

use libc::{c_int, c_uint};

use crate::util::Binding;
use crate::{raw, Buf, Error, Repository};

/// The result of a `describe` operation on either an `Describe` or a
/// `Repository`.
pub struct Describe<'repo> {
    raw: *mut raw::git_describe_result,
    _marker: marker::PhantomData<&'repo Repository>,
}

/// Options which indicate how a `Describe` is created.
pub struct DescribeOptions {
    raw: raw::git_describe_options,
    pattern: CString,
}

/// Options which can be used to customize how a description is formatted.
pub struct DescribeFormatOptions {
    raw: raw::git_describe_format_options,
    dirty_suffix: CString,
}

impl<'repo> Describe<'repo> {
    /// Prints this describe result, returning the result as a string.
    pub fn format(&self, opts: Option<&DescribeFormatOptions>) -> Result<String, Error> {
        let buf = Buf::new();
        let opts = opts.map(|o| &o.raw as *const _).unwrap_or(ptr::null());
        unsafe {
            try_call!(raw::git_describe_format(buf.raw(), self.raw, opts));
        }
        Ok(String::from_utf8(buf.to_vec()).unwrap())
    }
}

impl<'repo> Binding for Describe<'repo> {
    type Raw = *mut raw::git_describe_result;

    unsafe fn from_raw(raw: *mut raw::git_describe_result) -> Describe<'repo> {
        Describe {
            raw,
            _marker: marker::PhantomData,
        }
    }
    fn raw(&self) -> *mut raw::git_describe_result {
        self.raw
    }
}

impl<'repo> Drop for Describe<'repo> {
    fn drop(&mut self) {
        unsafe { raw::git_describe_result_free(self.raw) }
    }
}

impl Default for DescribeFormatOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DescribeFormatOptions {
    /// Creates a new blank set of formatting options for a description.
    pub fn new() -> DescribeFormatOptions {
        let mut opts = DescribeFormatOptions {
            raw: unsafe { mem::zeroed() },
            dirty_suffix: CString::new(Vec::new()).unwrap(),
        };
        opts.raw.version = 1;
        opts.raw.abbreviated_size = 7;
        opts
    }

    /// Sets the size of the abbreviated commit id to use.
    ///
    /// The value is the lower bound for the length of the abbreviated string,
    /// and the default is 7.
    pub fn abbreviated_size(&mut self, size: u32) -> &mut Self {
        self.raw.abbreviated_size = size as c_uint;
        self
    }

    /// Sets whether or not the long format is used even when a shorter name
    /// could be used.
    pub fn always_use_long_format(&mut self, long: bool) -> &mut Self {
        self.raw.always_use_long_format = long as c_int;
        self
    }

    /// If the workdir is dirty and this is set, this string will be appended to
    /// the description string.
    pub fn dirty_suffix(&mut self, suffix: &str) -> &mut Self {
        self.dirty_suffix = CString::new(suffix).unwrap();
        self.raw.dirty_suffix = self.dirty_suffix.as_ptr();
        self
    }
}

impl Default for DescribeOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl DescribeOptions {
    /// Creates a new blank set of formatting options for a description.
    pub fn new() -> DescribeOptions {
        let mut opts = DescribeOptions {
            raw: unsafe { mem::zeroed() },
            pattern: CString::new(Vec::new()).unwrap(),
        };
        opts.raw.version = 1;
        opts.raw.max_candidates_tags = 10;
        opts
    }

    #[allow(missing_docs)]
    pub fn max_candidates_tags(&mut self, max: u32) -> &mut Self {
        self.raw.max_candidates_tags = max as c_uint;
        self
    }

    /// Sets the reference lookup strategy
    ///
    /// This behaves like the `--tags` option to git-describe.
    pub fn describe_tags(&mut self) -> &mut Self {
        self.raw.describe_strategy = raw::GIT_DESCRIBE_TAGS as c_uint;
        self
    }

    /// Sets the reference lookup strategy
    ///
    /// This behaves like the `--all` option to git-describe.
    pub fn describe_all(&mut self) -> &mut Self {
        self.raw.describe_strategy = raw::GIT_DESCRIBE_ALL as c_uint;
        self
    }

    /// Indicates when calculating the distance from the matching tag or
    /// reference whether to only walk down the first-parent ancestry.
    pub fn only_follow_first_parent(&mut self, follow: bool) -> &mut Self {
        self.raw.only_follow_first_parent = follow as c_int;
        self
    }

    /// If no matching tag or reference is found whether a describe option would
    /// normally fail. This option indicates, however, that it will instead fall
    /// back to showing the full id of the commit.
    pub fn show_commit_oid_as_fallback(&mut self, show: bool) -> &mut Self {
        self.raw.show_commit_oid_as_fallback = show as c_int;
        self
    }

    #[allow(missing_docs)]
    pub fn pattern(&mut self, pattern: &str) -> &mut Self {
        self.pattern = CString::new(pattern).unwrap();
        self.raw.pattern = self.pattern.as_ptr();
        self
    }
}

impl Binding for DescribeOptions {
    type Raw = *mut raw::git_describe_options;

    unsafe fn from_raw(_raw: *mut raw::git_describe_options) -> DescribeOptions {
        panic!("unimplemened")
    }
    fn raw(&self) -> *mut raw::git_describe_options {
        &self.raw as *const _ as *mut _
    }
}

#[cfg(test)]
mod tests {
    use crate::DescribeOptions;

    #[test]
    fn smoke() {
        let (_td, repo) = crate::test::repo_init();
        let head = t!(repo.head()).target().unwrap();

        let d = t!(repo.describe(DescribeOptions::new().show_commit_oid_as_fallback(true)));
        let id = head.to_string();
        assert_eq!(t!(d.format(None)), &id[..7]);

        let obj = t!(repo.find_object(head, None));
        let sig = t!(repo.signature());
        t!(repo.tag("foo", &obj, &sig, "message", true));
        let d = t!(repo.describe(&DescribeOptions::new()));
        assert_eq!(t!(d.format(None)), "foo");

        let d = t!(obj.describe(&DescribeOptions::new()));
        assert_eq!(t!(d.format(None)), "foo");
    }
}
