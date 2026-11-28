// Regression tests for https://github.com/rust-lang/git2-rs/issues/1330

#![forbid(unsafe_code)]
use git2::Repository;

fn main() {
    let dir = std::env::temp_dir().join("git2-borrowed-strings");
    let mut repo = Repository::init(&dir).unwrap();

	repo.set_namespace_bytes(b"first").unwrap();
	let ns = repo.namespace_bytes().unwrap();
	repo.set_namespace_bytes(b"second").unwrap();
	println!("{ns:?}");
}
