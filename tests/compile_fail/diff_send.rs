// Regression tests for https://github.com/rust-lang/git2-rs/issues/1333

#[forbid(unsafe_code)]

use std::{fs, path::Path, thread};
use git2::{AttrCheckFlags, Repository};

fn main() {
    let dir = std::env::temp_dir().join("diff-send-demo");
    let _ = fs::remove_dir_all(&dir);
    let repo = Repository::init(&dir).unwrap();
    fs::write(dir.join(".git/info/attributes"), "*.c diff=cpp\n").unwrap();
    let tree = |content: &[u8]| {
        let blob = repo.blob(content).unwrap();
        let mut builder = repo.treebuilder(None).unwrap();
        builder.insert("a.c", blob, 0o100644).unwrap();
        repo.find_tree(builder.write().unwrap()).unwrap()
    };
    let (old, new) = (tree(b"int a;\n"), tree(b"int b;\n"));
    // Create the attribute cache and index up front, so the only shared state left is the driver registry.
    repo.get_attr(Path::new("a.c"), "diff", AttrCheckFlags::empty()).unwrap();
    repo.index().unwrap();

    let theirs = repo.diff_tree_to_tree(Some(&old), Some(&new), None).unwrap();
    let ours = repo.diff_tree_to_tree(Some(&old), Some(&new), None).unwrap();
    thread::scope(|s| {
        s.spawn(move || theirs.stats().unwrap().insertions());
        ours.stats().unwrap().insertions();
    });
}
