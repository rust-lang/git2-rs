#![cfg(feature = "unstable-sha256")]

//! A safe API must not hand libgit2 an `Oid` whose hash type differs from the
//! repository's. Its loose backend sizes the object path by the repository's
//! format but formats it with the id's own format, which overflows the path
//! buffer (SHA-256 id in a SHA-1 repository) or reads uninitialised bytes
//! (SHA-1 id in a SHA-256 repository).

use std::path::Path;

use git2::build::TreeUpdateBuilder;
use git2::{
    BlameOptions, ErrorClass, ErrorCode, FileMode, IndexEntry, IndexTime, ObjectFormat,
    OdbLookupFlags, Oid, Repository, RepositoryInitOptions, Signature,
};
use tempfile::TempDir;

fn init(format: ObjectFormat) -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let mut opts = RepositoryInitOptions::new();
    opts.object_format(format);
    let repo = Repository::init_opts(td.path(), &opts).unwrap();
    (td, repo)
}

fn init_with_commit(format: ObjectFormat) -> (TempDir, Repository, Oid, Oid, Oid) {
    let (td, repo) = init(format);
    let blob = repo.blob(b"data\n").unwrap();
    let mut tree_builder = repo.treebuilder(None).unwrap();
    tree_builder.insert("file", blob, 0o100644).unwrap();
    let tree_oid = tree_builder.write().unwrap();
    drop(tree_builder);
    let tree = repo.find_tree(tree_oid).unwrap();
    let sig = Signature::now("a", "a@b").unwrap();
    let commit_oid = repo
        .commit(Some("HEAD"), &sig, &sig, "msg", &tree, &[])
        .unwrap();
    drop(tree);
    (td, repo, commit_oid, tree_oid, blob)
}

fn sha1_oid() -> Oid {
    Oid::from_bytes(&[0xab; 20]).unwrap()
}

fn sha256_oid() -> Oid {
    Oid::from_bytes(&[0xab; 32]).unwrap()
}

fn index_entry(id: Oid) -> IndexEntry {
    IndexEntry {
        ctime: IndexTime::new(0, 0),
        mtime: IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: 0o100644,
        uid: 0,
        gid: 0,
        file_size: 0,
        id,
        flags: 0,
        flags_extended: 0,
        path: b"file".to_vec(),
    }
}

fn assert_invalid<T>(result: Result<T, git2::Error>) {
    match result {
        Ok(_) => panic!("expected an Invalid error"),
        Err(err) => {
            assert_eq!(err.code(), ErrorCode::Invalid, "{err:?}");
            assert_eq!(err.class(), ErrorClass::Invalid, "{err:?}");
            assert_eq!(
                err.message(),
                "object ID type does not match repository object format"
            );
        }
    }
}

fn assert_code<T>(result: Result<T, git2::Error>, code: ErrorCode) {
    match result {
        Ok(_) => panic!("expected {code:?}"),
        Err(err) => assert_eq!(err.code(), code, "{err:?}"),
    }
}

#[test]
fn repository_lookups_reject_a_foreign_oid() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let wide = sha256_oid();

    assert_invalid(repo.find_blob(wide));
    assert_invalid(repo.find_commit(wide));
    assert_invalid(repo.find_tree(wide));
    assert_invalid(repo.find_tag(wide));
    assert_invalid(repo.find_object(wide, None));
    assert_invalid(repo.find_annotated_commit(wide));
}

#[test]
fn matching_format_lookups_keep_their_errors() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    assert_code(repo.find_blob(sha1_oid()), ErrorCode::NotFound);

    let (_td, repo) = init(ObjectFormat::Sha256);
    assert_code(repo.find_blob(sha256_oid()), ErrorCode::NotFound);
}

#[test]
fn odb_rejects_a_foreign_oid() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let wide = sha256_oid();
    let odb = repo.odb().unwrap();

    assert!(!odb.exists(wide));
    assert!(!odb.exists_ext(wide, OdbLookupFlags::empty()));
    assert_invalid(odb.read(wide));
    assert_invalid(odb.read_header(wide));
    assert_invalid(odb.reader(wide));
    assert_invalid(odb.exists_prefix(wide, 1));
}

#[test]
fn builders_and_walks_reject_a_foreign_oid() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let wide = sha256_oid();

    let mut tree = repo.treebuilder(None).unwrap();
    assert_invalid(tree.insert("file", wide, 0o100644));
    assert_eq!(tree.len(), 0);

    let mut walk = repo.revwalk().unwrap();
    assert_invalid(walk.push(wide));
    assert_invalid(walk.hide(wide));

    let mut pack = repo.packbuilder().unwrap();
    assert_invalid(pack.insert_object(wide, None));
    assert_invalid(pack.insert_tree(wide));
    assert_invalid(pack.insert_commit(wide));
    assert_invalid(pack.insert_recursive(wide, None));
}

#[test]
fn matching_format_builders_keep_their_errors() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let mut tree = repo.treebuilder(None).unwrap();
    let err = match tree.insert("file", sha1_oid(), 0o100644) {
        Ok(_) => panic!("an absent object was accepted"),
        Err(err) => err,
    };
    assert_ne!(err.code(), ErrorCode::Invalid, "{err:?}");
}

#[test]
fn merge_and_graph_apis_reject_foreign_oids() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let one = sha256_oid();
    let two = Oid::from_bytes(&[0xcd; 32]).unwrap();

    assert_invalid(repo.merge_base(one, two));
    assert_invalid(repo.merge_base_many(&[one, two]));
    assert_invalid(repo.merge_base_octopus(&[one, two]));
    assert_invalid(repo.graph_ahead_behind(one, two));
    assert_invalid(repo.graph_descendant_of(one, two));
}

#[test]
fn refs_reject_a_foreign_oid() {
    let (_td, repo, commit, _, _) = init_with_commit(ObjectFormat::Sha1);
    let wide = sha256_oid();

    assert_invalid(repo.reference("refs/heads/x", wide, true, "create"));
    assert_invalid(repo.set_head_detached(wide));

    let mut reference = repo
        .reference("refs/heads/x", commit, true, "create")
        .unwrap();
    assert_invalid(reference.set_target(wide, "set"));
}

#[test]
fn index_rejects_a_foreign_oid() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let entry = index_entry(sha256_oid());
    let mut index = repo.index().unwrap();

    assert_invalid(index.add(&entry));

    // `add_frombuffer` hashes the buffer and ignores the entry's id, so a
    // placeholder id of another format is harmless there.
    index.add_frombuffer(&entry, b"data").unwrap();
}

#[test]
fn index_keeps_the_error_of_a_matching_oid() {
    let (_td, repo) = init(ObjectFormat::Sha1);
    let entry = index_entry(sha1_oid());
    let mut index = repo.index().unwrap();

    match index.add(&entry) {
        Ok(_) => panic!("an absent object was accepted"),
        Err(err) => assert_ne!(err.code(), ErrorCode::Invalid, "{err:?}"),
    }
}

#[test]
fn blame_rejects_foreign_bound_commits() {
    let (_td, repo, commit, _, _) = init_with_commit(ObjectFormat::Sha1);
    let wide = sha256_oid();

    let mut opts = BlameOptions::new();
    opts.newest_commit(wide);
    opts.oldest_commit(wide);
    assert_invalid(repo.blame_file(Path::new("file"), Some(&mut opts)));

    let mut opts = BlameOptions::new();
    opts.newest_commit(commit);
    opts.oldest_commit(commit);
    let blame = repo.blame_file(Path::new("file"), Some(&mut opts)).unwrap();
    assert_eq!(blame.len(), 1);
}

#[test]
fn tree_update_builder_rejects_a_foreign_oid() {
    let (_td, repo, _, tree_oid, blob) = init_with_commit(ObjectFormat::Sha1);
    let tree = repo.find_tree(tree_oid).unwrap();

    let mut update = TreeUpdateBuilder::new();
    update.upsert("file2", sha256_oid(), FileMode::Blob);
    assert_invalid(update.create_updated(&repo, &tree));

    let mut update = TreeUpdateBuilder::new();
    update.upsert("file2", blob, FileMode::Blob);
    update.create_updated(&repo, &tree).unwrap();
}

#[test]
fn sha256_repository_controls_still_work() {
    let (_td, repo, commit, tree_oid, blob) = init_with_commit(ObjectFormat::Sha256);

    assert_eq!(repo.find_blob(blob).unwrap().id(), blob);
    repo.set_head_detached(commit).unwrap();
    let mut reference = repo
        .reference("refs/heads/x", commit, true, "create")
        .unwrap();
    reference.set_target(commit, "set").unwrap();
    drop(reference);

    let mut entry = index_entry(blob);
    entry.path = b"blob".to_vec();
    repo.index().unwrap().add(&entry).unwrap();

    let mut opts = BlameOptions::new();
    opts.newest_commit(commit);
    opts.oldest_commit(commit);
    assert_eq!(
        repo.blame_file(Path::new("file"), Some(&mut opts))
            .unwrap()
            .len(),
        1
    );

    let tree = repo.find_tree(tree_oid).unwrap();
    let mut update = TreeUpdateBuilder::new();
    update.upsert("file2", blob, FileMode::Blob);
    update.create_updated(&repo, &tree).unwrap();
}
