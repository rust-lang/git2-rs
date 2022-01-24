//! Test for `get_extensions`, which reads a global state maintained by libgit2

use git2::opts::get_extensions;
use git2::Error;

#[test]
fn test_get_extensions() -> Result<(), Error> {
    let extensions = unsafe { get_extensions() }?;

    assert_eq!(extensions.len(), 1);
    assert_eq!(extensions.get(0), Some("noop"));

    Ok(())
}
