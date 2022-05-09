//! Bindings to libgit2's git_libgit2_opts function.

use std::ffi::CString;
use std::ptr;

use crate::string_array::StringArray;
use crate::util::Binding;
use crate::{raw, Buf, ConfigLevel, Error, IntoCString};

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
    try_call!(raw::git_libgit2_opts(
        raw::GIT_OPT_SET_SEARCH_PATH as libc::c_int,
        level as libc::c_int,
        path.into_c_string()?.as_ptr()
    ));
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
    try_call!(raw::git_libgit2_opts(
        raw::GIT_OPT_SET_SEARCH_PATH as libc::c_int,
        level as libc::c_int,
        core::ptr::null::<u8>()
    ));
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
    try_call!(raw::git_libgit2_opts(
        raw::GIT_OPT_GET_SEARCH_PATH as libc::c_int,
        level as libc::c_int,
        buf.raw() as *const _
    ));
    buf.into_c_string()
}

/// Controls whether or not libgit2 will cache loaded objects.  Enabled by
/// default, but disabling this can improve performance and memory usage if
/// loading a large number of objects that will not be referenced again.
/// Disabling this will cause repository objects to clear their caches when next
/// accessed.
pub fn enable_caching(enabled: bool) {
    let error = unsafe {
        raw::git_libgit2_opts(
            raw::GIT_OPT_ENABLE_CACHING as libc::c_int,
            enabled as libc::c_int,
        )
    };
    // This function cannot actually fail, but the function has an error return
    // for other options that can.
    debug_assert!(error >= 0);
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

/// Returns the list of git extensions that are supported. This is the list of
/// built-in extensions supported by libgit2 and custom extensions that have
/// been added with [`set_extensions`]. Extensions that have been negated will
/// not be returned.
///
/// # Safety
///
/// libgit2 stores user extensions in a static variable.
/// This function is effectively reading a `static mut` and should be treated as such
pub unsafe fn get_extensions() -> Result<StringArray, Error> {
    crate::init();

    let mut extensions = raw::git_strarray {
        strings: ptr::null_mut(),
        count: 0,
    };

    try_call!(raw::git_libgit2_opts(
        raw::GIT_OPT_GET_EXTENSIONS as libc::c_int,
        &mut extensions
    ));

    Ok(StringArray::from_raw(extensions))
}

/// Set that the given git extensions are supported by the caller. Extensions
/// supported by libgit2 may be negated by prefixing them with a `!`.
/// For example: setting extensions to `[ "!noop", "newext" ]` indicates that
/// the caller does not want to support repositories with the `noop` extension
/// but does want to support repositories with the `newext` extension.
///
/// # Safety
///
/// libgit2 stores user extensions in a static variable.
/// This function is effectively modifying a `static mut` and should be treated as such
pub unsafe fn set_extensions<E>(extensions: &[E]) -> Result<(), Error>
where
    for<'x> &'x E: IntoCString,
{
    crate::init();

    let extensions = extensions
        .iter()
        .map(|e| e.into_c_string())
        .collect::<Result<Vec<_>, _>>()?;

    let extension_ptrs = extensions.iter().map(|e| e.as_ptr()).collect::<Vec<_>>();

    try_call!(raw::git_libgit2_opts(
        raw::GIT_OPT_SET_EXTENSIONS as libc::c_int,
        extension_ptrs.as_ptr(),
        extension_ptrs.len() as libc::size_t
    ));

    Ok(())
}

/// Set wheter or not to verify ownership before performing a repository.
/// Enabled by default, but disabling this can lead to code execution vulnerabilities.
pub unsafe fn set_verify_owner_validation(enabled: bool) -> Result<(), Error> {
    let error = raw::git_libgit2_opts(
        raw::GIT_OPT_SET_OWNER_VALIDATION as libc::c_int,
        enabled as libc::c_int,
    );
    // This function cannot actually fail, but the function has an error return
    // for other options that can.
    debug_assert!(error >= 0);
    Ok(())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn smoke() {
        strict_hash_verification(false);
    }
}
