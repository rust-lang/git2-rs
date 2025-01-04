//! Test for `set_extensions`, which writes a global state maintained by libgit2

use git2::opts::{get_extensions, set_extensions};
use git2::Error;

#[test]
fn test_add_extensions() -> Result<(), Error> {
    unsafe {
        set_extensions(&["custom"])?;
    }

    let extensions = unsafe { get_extensions() }?;
    let extensions: Vec<_> = extensions.iter().collect();

    assert_eq!(
        extensions,
        [
            Some("custom"),
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
