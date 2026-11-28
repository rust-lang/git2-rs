// Regression tests for https://github.com/rust-lang/git2-rs/issues/1330

#![forbid(unsafe_code)]
use git2::Repository;

fn main() {
    let dir = std::env::temp_dir().join("git2-borrowed-strings");
    let mut repo = Repository::init(&dir).unwrap();

    let workdir = repo.workdir().unwrap();
    repo.set_workdir(&dir.join(".git"), false).unwrap();
    println!("{}", workdir.display());
}
