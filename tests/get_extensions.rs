//! Test for `get_extensions`, which reads a global state maintained by libgit2

use git2::opts::get_extensions;
use git2::Error;

#[test]
fn test_get_extensions() -> Result<(), Error> {
    let extensions = unsafe { get_extensions() }?;

    assert_eq!(extensions.len(), 3);
    assert_eq!(extensions.get(0), Some("noop"));
    // The objectformat extension was added in 1.6
    assert_eq!(extensions.get(1), Some("objectformat"));
    // The worktreeconfig extension was added in 1.8
    assert_eq!(extensions.get(2), Some("worktreeconfig"));

    Ok(())
}
