use std::io::TempDir;
use {Repository, Commit, Tree, Signature};

macro_rules! git( ( $cwd:expr, $($arg:expr),*) => ({
    use std::str;
    let mut cmd = ::std::io::Command::new("git");
    cmd.cwd($cwd)$(.arg($arg))*;
    let out = cmd.output().unwrap();
    if !out.status.success() {
        let err = str::from_utf8(out.error.as_slice()).unwrap_or("<not-utf8>");
        let out = str::from_utf8(out.output.as_slice()).unwrap_or("<not-utf8>");
        fail!("cmd failed: {}\n{}\n{}\n", cmd, out, err);
    }
    str::from_utf8(out.output.as_slice()).unwrap().trim().to_string()
}) )

pub fn repo_init() -> (TempDir, Repository) {
    let td = TempDir::new("test").unwrap();
    Repository::init(td.path(), false).unwrap();
    git!(td.path(), "config", "user.name", "name");
    git!(td.path(), "config", "user.email", "email");

    let repo = Repository::init(td.path(), false).unwrap();
    {
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();

        let tree = Tree::lookup(&repo, id).unwrap();
        let sig = Signature::default(&repo).unwrap();
        Commit::new(&repo, Some("HEAD"), &sig, &sig, "initial",
                    &tree, []).unwrap();
    }
    (td, repo)
}
