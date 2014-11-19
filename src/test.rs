use std::io::TempDir;
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
