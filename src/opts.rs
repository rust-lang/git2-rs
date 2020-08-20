//! Bindings to libgit2's git_libgit2_opts function.

use crate::raw;

/// Controls whether or not libgit2 will verify that objects loaded have the
/// expected hash. Enabled by default, but disabling this can significantly
/// improve performance, at the cost of relying on repository integrity
/// without checking it.
pub fn strict_hash_verification(enabled: bool) {
    let error = unsafe {
        raw::git_libgit2_opts(
            raw::GIT_OPT_ENABLE_STRICT_HASH_VERIFICATION as libc::c_int,
            if enabled { 1 } else { 0 } as libc::c_int,
        )
    };
    // This function cannot actually fail, but the function has an error return
    // for other options that can.
    debug_assert!(error >= 0);
}

#[cfg(test)]
mod test {
    #[test]
    fn smoke() {
        super::strict_hash_verification(false);
    }
}
