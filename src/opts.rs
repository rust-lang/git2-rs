//! Bindings to libgit2's git_libgit2_opts function.

use std::ffi::CString;

use crate::util::Binding;
use crate::{call, raw, Buf, ConfigLevel, Error, IntoCString};

/// Set the search path for a level of config data. The search path applied to
/// shared attributes and ignore files, too.
///
/// `level` must be one of [`ConfigLevel::System`], [`ConfigLevel::Global`],
/// [`ConfigLevel::XDG`], [`ConfigLevel::ProgramData`].
///
/// `path` lists directories delimited by `GIT_PATH_LIST_SEPARATOR`.
/// Use magic path `$PATH` to include the old value of the path
/// (if you want to prepend or append, for instance).
///
/// This function is unsafe as it mutates the global state but cannot guarantee
/// thread-safety. It needs to be externally synchronized with calls to access
/// the global state.
pub unsafe fn set_search_path<P>(level: ConfigLevel, path: P) -> Result<(), Error>
where
    P: IntoCString,
{
    crate::init();
    call::c_try(raw::git_libgit2_opts(
        raw::GIT_OPT_SET_SEARCH_PATH as libc::c_int,
        level as libc::c_int,
        path.into_c_string()?.as_ptr(),
    ))?;
    Ok(())
}

/// Reset the search path for a given level of config data to the default
/// (generally based on environment variables).
///
/// `level` must be one of [`ConfigLevel::System`], [`ConfigLevel::Global`],
/// [`ConfigLevel::XDG`], [`ConfigLevel::ProgramData`].
///
/// This function is unsafe as it mutates the global state but cannot guarantee
/// thread-safety. It needs to be externally synchronized with calls to access
/// the global state.
pub unsafe fn reset_search_path(level: ConfigLevel) -> Result<(), Error> {
    crate::init();
    call::c_try(raw::git_libgit2_opts(
        raw::GIT_OPT_SET_SEARCH_PATH as libc::c_int,
        level as libc::c_int,
        core::ptr::null::<u8>(),
    ))?;
    Ok(())
}

/// Get the search path for a given level of config data.
///
/// `level` must be one of [`ConfigLevel::System`], [`ConfigLevel::Global`],
/// [`ConfigLevel::XDG`], [`ConfigLevel::ProgramData`].
///
/// This function is unsafe as it mutates the global state but cannot guarantee
/// thread-safety. It needs to be externally synchronized with calls to access
/// the global state.
pub unsafe fn get_search_path(level: ConfigLevel) -> Result<CString, Error> {
    crate::init();
    let buf = Buf::new();
    call::c_try(raw::git_libgit2_opts(
        raw::GIT_OPT_GET_SEARCH_PATH as libc::c_int,
        level as libc::c_int,
        buf.raw(),
    ))?;
    buf.into_c_string()
}

/// Controls whether or not libgit2 will verify when writing an object that all
/// objects it references are valid. Enabled by default, but disabling this can
/// significantly improve performance, at the cost of potentially allowing the
/// creation of objects that reference invalid objects (due to programming
/// error or repository corruption).
pub fn strict_object_creation(enabled: bool) {
    let error = unsafe {
        raw::git_libgit2_opts(
            raw::GIT_OPT_ENABLE_STRICT_OBJECT_CREATION as libc::c_int,
            enabled as libc::c_int,
        )
    };
    // This function cannot actually fail, but the function has an error return
    // for other options that can.
    debug_assert!(error >= 0);
}

/// Controls whether or not libgit2 will verify that objects loaded have the
/// expected hash. Enabled by default, but disabling this can significantly
/// improve performance, at the cost of relying on repository integrity
/// without checking it.
pub fn strict_hash_verification(enabled: bool) {
    let error = unsafe {
        raw::git_libgit2_opts(
            raw::GIT_OPT_ENABLE_STRICT_HASH_VERIFICATION as libc::c_int,
            enabled as libc::c_int,
        )
    };
    // This function cannot actually fail, but the function has an error return
    // for other options that can.
    debug_assert!(error >= 0);
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke() {
        strict_hash_verification(false);
    }
}
