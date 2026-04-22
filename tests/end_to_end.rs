//! Tests for some end-to-end logic about certain operations
use git2::{Error, Repository, RepositoryInitOptions};

use libgit2_sys as raw;
use std::ffi::{CString, OsStr};
use std::ptr;

use tempfile::TempDir;

#[test]
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
        use std::os::unix::prelude::*;
        let os_path: &OsStr = path.as_ref();
        let path = CString::new(os_path.as_bytes()).unwrap();
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
            assert_eq!(0, result);
        }

        // Reference::peel()
        let mut peeled = ptr::null_mut();
        unsafe {
            let result =
                raw::git_reference_peel(&mut peeled, head_reference, raw::GIT_OBJECT_COMMIT);
            assert_eq!(0, result);
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
            assert_eq!(0, result);
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
