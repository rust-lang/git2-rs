use git2::{ErrorCode, RefdbType, Repository, RepositoryInitOptions, Signature};

#[test]
fn default_init_does_not_enable_reftable() {
    let directory = tempfile::tempdir().unwrap();
    let repo = Repository::init(directory.path()).unwrap();
    assert_eq!(
        repo.config()
            .unwrap()
            .get_string("extensions.refstorage")
            .unwrap_err()
            .code(),
        ErrorCode::NotFound
    );
    assert!(!repo.path().join("reftable").exists());
}

#[test]
fn reference_formats_roundtrip() {
    for (format, other_format) in [
        (RefdbType::Files, RefdbType::Reftable),
        (RefdbType::Reftable, RefdbType::Files),
    ] {
        let directory = tempfile::tempdir().unwrap();
        let repo = Repository::init_opts(
            directory.path(),
            RepositoryInitOptions::new()
                .initial_head("main")
                .refdb_type(format),
        )
        .unwrap();
        let signature = Signature::now("Nobody", "nobody@example.com").unwrap();
        let head = {
            let tree_id = repo.index().unwrap().write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &signature, &signature, "initial", &tree, &[])
                .unwrap()
        };
        repo.reference("refs/tags/test", head, false, "create tag")
            .unwrap();
        repo.branch("topic", &repo.find_commit(head).unwrap(), false)
            .unwrap();
        repo.refdb().unwrap().compress().unwrap();
        assert_eq!(repo.refname_to_id("refs/tags/test").unwrap(), head);
        assert_eq!(repo.branches(None).unwrap().count(), 2);
        assert!(!repo.reflog("HEAD").unwrap().is_empty());
        if format == RefdbType::Reftable {
            assert_eq!(
                repo.config()
                    .unwrap()
                    .get_string("extensions.refStorage")
                    .unwrap(),
                "reftable"
            );
            assert!(repo.path().join("reftable").is_dir());
        }
        drop(repo);

        let error = Repository::init_opts(
            directory.path(),
            RepositoryInitOptions::new()
                .refdb_type(other_format)
                .no_reinit(false),
        )
        .err()
        .unwrap();
        assert_eq!(error.code(), ErrorCode::Exists);

        let repo = Repository::init(directory.path()).unwrap();
        assert_eq!(repo.refname_to_id("refs/heads/topic").unwrap(), head);
        let config = repo.config().unwrap();
        let storage = config.get_string("extensions.refstorage");
        if format == RefdbType::Reftable {
            assert_eq!(storage.unwrap(), "reftable");
        } else {
            assert_eq!(storage.unwrap_err().code(), ErrorCode::NotFound);
        }
        drop(config);
        drop(repo);

        let repo = Repository::open(directory.path()).unwrap();
        assert_eq!(repo.head().unwrap().target(), Some(head));
        assert_eq!(repo.refname_to_id("refs/heads/topic").unwrap(), head);
        assert_eq!(repo.refname_to_id("refs/tags/test").unwrap(), head);
    }
}
