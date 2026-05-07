//! Tests for some end-to-end logic about certain operations
use git2::{Error, ReferenceType, Repository, RepositoryInitOptions, StashFlags};

use libgit2_sys as raw;
use std::ffi::{CString, OsString};
use std::fs;
use std::ptr;

use tempfile::TempDir;

// Skip on MacOS, where git cannot even create a branch with a non-UTF8 name
// Same on Windows

#[test]
#[cfg_attr(any(windows, target_os = "macos"), ignore)]
fn non_utf8_branch() {
    let td = TempDir::new().unwrap();
    let path = td.path();
    {
        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = Repository::init_opts(path, &opts).unwrap();

        let mut config = repo.config().unwrap();
        config.set_str("user.name", "name").unwrap();
        config.set_str("user.email", "email").unwrap();
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();

        let tree = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial\n\nbody", &tree, &[])
            .unwrap();
    }

    // Create a branch with a non-UTF8 name
    // Since that cannot be done via the git2-rs interface, call the raw
    // underlying binding
    // For that we also have to recreate the underlying `git_repository` pointer
    {
        macro_rules! check_result {
            ($result:ident) => {
                if $result != 0 {
                    let e = Error::last_error($result);
                    // Show the error details in the assertion failure
                    assert_eq!("", format!("{:?}", e));
                }
            };
        }
        // based on util.rs IntoCString for OsString
        // Need cfg() guards since the file is compiled on Windows even when
        // the test is skipped
        #[cfg(unix)]
        fn ostr_to_cstr(s: OsString) -> CString {
            use std::ffi::OsStr;
            use std::os::unix::prelude::*;
            let s: &OsStr = s.as_ref();
            CString::new(s.as_bytes()).unwrap()
        }
        #[cfg(windows)]
        fn ostr_to_cstr(s: OsString) -> CString {
            panic!("Test is skipped on Windows");
        }

        let path = ostr_to_cstr(path.into());
        let mut repo = ptr::null_mut();
        unsafe {
            let result = raw::git_repository_open(&mut repo, path.as_ptr());
            assert_eq!(0, result);
        }
        // We now have a git_repository pointer in `repo`
        // Get a reference to the latest commit

        // Repo::head()
        let mut head_reference = ptr::null_mut();
        unsafe {
            let result = raw::git_repository_head(&mut head_reference, repo);
            check_result!(result);
        }

        // Reference::peel()
        let mut peeled = ptr::null_mut();
        unsafe {
            let result =
                raw::git_reference_peel(&mut peeled, head_reference, raw::GIT_OBJECT_COMMIT);
            check_result!(result);
            assert_eq!(raw::GIT_OBJECT_COMMIT, raw::git_object_type(&*peeled));
        }

        // Object::cast_or_panic(), already confirmed to be a commit
        let as_commit = peeled as *mut raw::git_commit;

        let branch_name = CString::new(vec![b'f', 0xff, 0xC0, b'o', b'o']).unwrap();
        let mut branch_reference = ptr::null_mut();
        unsafe {
            let result = raw::git_branch_create(
                &mut branch_reference,
                repo,
                branch_name.as_ptr(),
                as_commit,
                0,
            );
            check_result!(result);
        }

        unsafe {
            // impl Drop for Reference
            raw::git_reference_free(branch_reference);

            // impl Drop for Commit
            raw::git_commit_free(as_commit);

            // impl Drop for Object
            raw::git_object_free(peeled);

            // impl Drop for Reference
            raw::git_reference_free(head_reference);

            // impl Drop for Repository
            raw::git_repository_free(repo);
        }
    }

    // Now, get the repo again
    let repo = Repository::open(path).expect("created above");
    let mut refs = repo.references().expect("references");
    let mut names = refs.names();

    assert_eq!(
        Some(Err(Error::from_str(
            "invalid utf-8 sequence of 1 bytes from index 12"
        ))),
        names.next()
    );
    assert_eq!(Some(Ok("refs/heads/main")), names.next());
    assert_eq!(None, names.next());
}

#[test]
fn stash_length() {
    // Test that reflog() and len() allow tracking the number of stashed entries
    let td = TempDir::new().unwrap();
    let path = td.path();

    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let mut repo = Repository::init_opts(path, &opts).unwrap();

    let initial_reflog = repo.reflog("refs/stash").expect("Should work");
    assert_eq!(0, initial_reflog.len());

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();
    let sig = repo.signature().unwrap();

    // Need an initial commit before changes can be stashed
    let mut index = repo.index().unwrap();
    let id = index.write_tree().unwrap();
    {
        let tree = repo.find_tree(id).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial\n\nbody", &tree, &[])
            .unwrap();
    }

    fs::write(path.join("README.md"), "README content").unwrap();
    repo.stash_save(&sig, "Stashed", Some(StashFlags::INCLUDE_UNTRACKED))
        .unwrap();

    // Existing reflog references are not updated
    assert_eq!(0, initial_reflog.len());

    let after_stash1 = repo.reflog("refs/stash").expect("Should work");
    assert_eq!(1, after_stash1.len());

    fs::write(path.join("README.md2"), "README content").unwrap();
    repo.stash_save(&sig, "Stashed2", Some(StashFlags::INCLUDE_UNTRACKED))
        .unwrap();

    assert_eq!(0, initial_reflog.len());
    assert_eq!(1, after_stash1.len());

    let after_stash2 = repo.reflog("refs/stash").expect("Should work");
    assert_eq!(2, after_stash2.len());

    repo.stash_drop(1).expect("Should succeed");

    assert_eq!(0, initial_reflog.len());
    assert_eq!(1, after_stash1.len());
    assert_eq!(2, after_stash2.len());
    let after_drop1 = repo.reflog("refs/stash").expect("Should work");
    assert_eq!(1, after_drop1.len());

    repo.stash_drop(0).expect("Should succeed");

    assert_eq!(0, initial_reflog.len());
    assert_eq!(1, after_stash1.len());
    assert_eq!(2, after_stash2.len());
    assert_eq!(1, after_drop1.len());
    let after_drop2 = repo.reflog("refs/stash").expect("Should work");
    assert_eq!(0, after_drop2.len());
}

#[test]
fn branch_name_on_init() {
    // Confirm that the branch name is available via find_reference() even when
    // no commits are made yet and the branch doesn't exist
    // Test with "main"
    {
        let td = TempDir::new().unwrap();
        let path = td.path();

        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("main");
        let repo = Repository::init_opts(path, &opts).unwrap();

        let head_ref = repo.find_reference("HEAD").unwrap();
        assert_eq!(Some(ReferenceType::Symbolic), head_ref.kind());
        assert_eq!(Some("HEAD"), head_ref.name());

        let target = head_ref.symbolic_target();
        assert_eq!(Some("refs/heads/main"), target);
    }
    // Test with "somerandombranchnamehere"
    {
        let td = TempDir::new().unwrap();
        let path = td.path();

        let mut opts = RepositoryInitOptions::new();
        opts.initial_head("somerandombranchnamehere");
        let repo = Repository::init_opts(path, &opts).unwrap();

        let head_ref = repo.find_reference("HEAD").unwrap();
        assert_eq!(Some(ReferenceType::Symbolic), head_ref.kind());
        assert_eq!(Some("HEAD"), head_ref.name());

        let target = head_ref.symbolic_target();
        assert_eq!(Some("refs/heads/somerandombranchnamehere"), target);
    }
}
