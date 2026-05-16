//! Test for `get_extensions`, which reads a global state maintained by libgit2

use git2::opts::get_extensions;
use git2::Error;

#[test]
fn test_get_extensions() -> Result<(), Error> {
    let extensions = unsafe { get_extensions() }?;
    let extensions: Result<Vec<_>, Error> = extensions.iter().collect();
    let extensions = extensions.unwrap();

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
