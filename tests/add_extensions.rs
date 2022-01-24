//! Test for `set_extensions`, which writes a global state maintained by libgit2

use git2::opts::{get_extensions, set_extensions};
use git2::Error;

#[test]
fn test_add_extensions() -> Result<(), Error> {
    unsafe {
        set_extensions(&["custom"])?;
    }

    let extensions = unsafe { get_extensions() }?;

    assert_eq!(extensions.len(), 2);
    assert_eq!(extensions.get(0), Some("noop"));
    assert_eq!(extensions.get(1), Some("custom"));

    Ok(())
}
