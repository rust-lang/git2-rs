//! Test for some global state set up by libgit2's `git_libgit2_init` function
//! that need to be synchronized within a single process.

use git2::opts;
use git2::{ConfigLevel, IntoCString};

// Test for mutating configuration file search path which is set during
// initialization in libgit2's `git_sysdir_global_init` function.
#[test]
fn search_path() -> Result<(), Box<dyn std::error::Error>> {
    use std::env::join_paths;

    let path = "fake_path";
    let original = unsafe { opts::get_search_path(ConfigLevel::Global) };
    assert_ne!(original, Ok(path.into_c_string()?));

    // Set
    unsafe {
        opts::set_search_path(ConfigLevel::Global, &path)?;
    }
    assert_eq!(
        unsafe { opts::get_search_path(ConfigLevel::Global) },
        Ok(path.into_c_string()?)
    );

    // Append
    let paths = join_paths(["$PATH", path].iter())?;
    let expected_paths = join_paths([path, path].iter())?.into_c_string()?;
    unsafe {
        opts::set_search_path(ConfigLevel::Global, paths)?;
    }
    assert_eq!(
        unsafe { opts::get_search_path(ConfigLevel::Global) },
        Ok(expected_paths)
    );

    // Reset
    unsafe {
        opts::reset_search_path(ConfigLevel::Global)?;
    }
    assert_eq!(
        unsafe { opts::get_search_path(ConfigLevel::Global) },
        original
    );

    Ok(())
}
