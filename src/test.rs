use std::path::{Path, PathBuf};
use std::io;
use tempdir::TempDir;
use url::Url;

use Repository;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {}", stringify!($e), e),
    })
}

pub fn repo_init() -> (TempDir, Repository) {
    let td = TempDir::new("test").unwrap();
    let repo = Repository::init(td.path()).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "name").unwrap();
        config.set_str("user.email", "email").unwrap();
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();

        let tree = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial",
                    &tree, &[]).unwrap();
    }
    (td, repo)
}

pub fn path2url(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}

pub fn realpath(original: &Path) -> io::Result<PathBuf> {
    // TODO: implement this
    Ok(original.to_path_buf())
}
