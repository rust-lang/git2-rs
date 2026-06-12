//! Test for `set_extensions`, which writes a global state maintained by libgit2

use git2::opts::{get_extensions, set_extensions};
use git2::Error;

#[test]
fn test_add_extensions() -> Result<(), Error> {
    unsafe {
        set_extensions(&["custom"])?;
    }

    let extensions = unsafe { get_extensions() }?;
    let extensions: Result<Vec<_>, Error> = extensions.iter().collect();
    let extensions = extensions.unwrap();

    assert_eq!(
        extensions,
        [
            "custom",
            "noop",
            // The objectformat extension was added in 1.6
            "objectformat",
            // The preciousobjects extension was added in 1.9
            "preciousobjects",
            // The relativeworktrees extension was added in 1.9.4
            "relativeworktrees",
            // The worktreeconfig extension was added in 1.8
            "worktreeconfig"
        ]
    );

    Ok(())
}
