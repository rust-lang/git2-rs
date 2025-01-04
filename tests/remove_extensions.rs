//! Test for `set_extensions`, which writes a global state maintained by libgit2

use git2::opts::{get_extensions, set_extensions};
use git2::Error;

#[test]
fn test_remove_extensions() -> Result<(), Error> {
    unsafe {
        set_extensions(&[
            "custom",
            "!ignore",
            "!noop",
            "!objectformat",
            "!preciousobjects",
            "!worktreeconfig",
            "other",
        ])?;
    }

    let extensions = unsafe { get_extensions() }?;
    let extensions: Vec<_> = extensions.iter().collect();

    assert_eq!(extensions, [Some("custom"), Some("other")]);

    Ok(())
}
