use std::io::TempDir;
use {Repository, Commit, Tree, Signature};

pub fn repo_init() -> (TempDir, Repository) {
    let td = TempDir::new("test").unwrap();
    let repo = Repository::init(td.path(), false).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "name").unwrap();
        config.set_str("user.email", "email").unwrap();
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();

        let tree = Tree::lookup(&repo, id).unwrap();
        let sig = Signature::default(&repo).unwrap();
        Commit::new(&repo, Some("HEAD"), &sig, &sig, "initial",
                    &tree, []).unwrap();
    }
    (td, repo)
}
