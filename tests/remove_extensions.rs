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
            "!worktreeconfig",
            "other",
        ])?;
    }

    let extensions = unsafe { get_extensions() }?;

    assert_eq!(extensions.len(), 2);
    assert_eq!(extensions.get(0), Some("custom"));
    assert_eq!(extensions.get(1), Some("other"));

    Ok(())
}
