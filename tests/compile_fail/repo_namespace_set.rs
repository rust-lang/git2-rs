// Regression tests for https://github.com/rust-lang/git2-rs/issues/1330

#![forbid(unsafe_code)]
use git2::Repository;

fn main() {
    let dir = std::env::temp_dir().join("git2-borrowed-strings");
    let mut repo = Repository::init(&dir).unwrap();

	repo.set_namespace("first").unwrap();
	let ns = repo.namespace().unwrap().unwrap();
	repo.set_namespace("second").unwrap();
	println!("{ns}");
}
