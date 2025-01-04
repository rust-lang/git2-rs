//! Test for `get_extensions`, which reads a global state maintained by libgit2

use git2::opts::get_extensions;
use git2::Error;

#[test]
fn test_get_extensions() -> Result<(), Error> {
    let extensions = unsafe { get_extensions() }?;
    let extensions: Vec<_> = extensions.iter().collect();

    assert_eq!(
        extensions,
        [
            Some("noop"),
            // The objectformat extension was added in 1.6
            Some("objectformat"),
            // The preciousobjects extension was added in 1.9
            Some("preciousobjects"),
            // The worktreeconfig extension was added in 1.8
            Some("worktreeconfig")
        ]
    );

    Ok(())
}
