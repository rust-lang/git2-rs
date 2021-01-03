//! Bindings to libgit2's git_libgit2_opts function.

use std::ffi::CString;
use std::sync::Mutex;

use once_cell::sync::Lazy;

use crate::util::Binding;
use crate::{call, raw, Buf, ConfigLevel, Error, IntoCString};

static SEARCH_PATH: Lazy<Mutex<SearchPath>> = Lazy::new(|| {
    crate::init();
    Mutex::new(SearchPath)
});

struct SearchPath;

impl SearchPath {
    pub fn set<P>(&self, level: ConfigLevel, path: P) -> Result<(), Error>
    where
        P: IntoCString,
    {
        let path = path.into_c_string()?;
        unsafe {
            call::c_try(raw::git_libgit2_opts(
                raw::GIT_OPT_SET_SEARCH_PATH as libc::c_int,
                level as libc::c_int,
                path.as_ptr(),
            ))?;
        }
        Ok(())
    }

    pub fn reset(&self, level: ConfigLevel) -> Result<(), Error> {
        unsafe {
            call::c_try(raw::git_libgit2_opts(
                raw::GIT_OPT_SET_SEARCH_PATH as libc::c_int,
                level as libc::c_int,
                core::ptr::null::<u8>(),
            ))?;
        }
        Ok(())
    }

    pub fn get(&self, level: ConfigLevel) -> Result<CString, Error> {
        let buf = Buf::new();
        unsafe {
            call::c_try(raw::git_libgit2_opts(
                raw::GIT_OPT_GET_SEARCH_PATH as libc::c_int,
                level as libc::c_int,
                buf.raw(),
            ))?;
        }
        buf.into_c_string()
    }
}

/// Set the search path for a level of config data. The search path applied to
/// shared attributes and ignore files, too.
///
/// `level` must be one of [`ConfigLevel::System`], [`ConfigLevel::Global`],
/// [`ConfigLevel::XDG`], [`ConfigLevel::ProgramData`].
///
/// `path` lists directories delimited by `GIT_PATH_LIST_SEPARATOR`.
/// Use magic path `$PATH` to include the old value of the path
/// (if you want to prepend or append, for instance).
pub fn set_search_path<P>(level: ConfigLevel, path: P) -> Result<(), Error>
where
    P: IntoCString,
{
    SEARCH_PATH.lock().unwrap().set(level, path)
}

/// Reset the search path for a given level of config data to the default
/// (generally based on environment variables).
///
/// `level` must be one of [`ConfigLevel::System`], [`ConfigLevel::Global`],
/// [`ConfigLevel::XDG`], [`ConfigLevel::ProgramData`].
pub fn reset_search_path(level: ConfigLevel) -> Result<(), Error> {
    SEARCH_PATH.lock().unwrap().reset(level)
}

/// Get the search path for a given level of config data.
///
/// `level` must be one of [`ConfigLevel::System`], [`ConfigLevel::Global`],
/// [`ConfigLevel::XDG`], [`ConfigLevel::ProgramData`].
pub fn get_search_path(level: ConfigLevel) -> Result<CString, Error> {
    SEARCH_PATH.lock().unwrap().get(level)
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
    use std::env::join_paths;

    #[test]
    fn smoke() {
        strict_hash_verification(false);
    }

    #[test]
    fn search_path() -> Result<(), Box<dyn std::error::Error>> {
        let path = "fake_path";
        let original = get_search_path(ConfigLevel::Global);
        assert_ne!(original, Ok(path.into_c_string()?));

        // Set
        set_search_path(ConfigLevel::Global, &path)?;
        assert_eq!(
            get_search_path(ConfigLevel::Global),
            Ok(path.into_c_string()?)
        );

        // Append
        let paths = join_paths(["$PATH", path].iter())?;
        let expected_paths = join_paths([path, path].iter())?.into_c_string()?;
        set_search_path(ConfigLevel::Global, paths)?;
        assert_eq!(get_search_path(ConfigLevel::Global), Ok(expected_paths));

        // Reset
        reset_search_path(ConfigLevel::Global)?;
        assert_eq!(get_search_path(ConfigLevel::Global), original);

        Ok(())
    }
}
