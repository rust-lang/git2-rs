use std::old_io::TempDir;
use std::old_io::{self, fs};
use std::env;
use Repository;

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

// Copied from rustc
pub fn realpath(original: &Path) -> old_io::IoResult<Path> {
    static MAX_LINKS_FOLLOWED: u32 = 256;
    let original = env::current_dir().unwrap().join(original);
    // Right now lstat on windows doesn't work quite well
    if cfg!(windows) {
        return Ok(original)
    }
    let result = original.root_path();
    let mut result = result.expect("make_absolute has no root_path");
    let mut followed = 0;
    for part in original.components() {
        result.push(part);
        loop {
            if followed == MAX_LINKS_FOLLOWED {
                return Err(old_io::standard_error(old_io::InvalidInput))
            }
            match fs::lstat(&result) {
                Err(..) => break,
                Ok(ref stat) if stat.kind != old_io::FileType::Symlink => break,
                Ok(..) => {
                    followed += 1;
                    let path = try!(fs::readlink(&result));
                    result.pop();
                    result.push(path);
                }
            }
        }
    }
    return Ok(result);
}
