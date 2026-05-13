//! Tests for some end-to-end logic about certain operations
use git2::{
    Error, ReferenceType, Repository, RepositoryInitOptions, StashFlags, Status, StatusOptions,
};

use libgit2_sys as raw;
use std::ffi::{CString, OsString};
use std::fs;
use std::path::Path;
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

#[test]
fn repo_status() {
    let td = TempDir::new().unwrap();
    let path = td.path();

    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(path, &opts).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();

    // Create some files
    fs::write(path.join("BothModified"), "BothModified").unwrap();
    fs::write(path.join("IndexModified"), "IndexModified").unwrap();
    fs::write(path.join("IndexDeleted"), "IndexDeleted").unwrap();
    fs::write(path.join("IndexRenamed"), "IndexRenamed").unwrap();
    fs::write(path.join("IndexTypechange"), "IndexTypechange").unwrap();
    fs::write(path.join("WorktreeDeleted"), "WorktreeDeleted").unwrap();
    fs::write(path.join("WorktreeModified"), "WorktreeModified").unwrap();
    fs::write(path.join("WorktreeTypechange"), "WorktreeTypechange").unwrap();
    fs::write(path.join("WorktreeRenamed"), "WorktreeRenamed").unwrap();
    fs::write(path.join("Unchanged"), "Unchanged").unwrap();
    fs::write(path.join(".gitignore"), "ignored-*").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(&Path::new("BothModified")).unwrap();
    index.add_path(&Path::new("IndexModified")).unwrap();
    index.add_path(&Path::new("IndexDeleted")).unwrap();
    index.add_path(&Path::new("IndexRenamed")).unwrap();
    index.add_path(&Path::new("IndexTypechange")).unwrap();
    index.add_path(&Path::new("Unchanged")).unwrap();
    index.add_path(&Path::new("WorktreeDeleted")).unwrap();
    index.add_path(&Path::new("WorktreeModified")).unwrap();
    index.add_path(&Path::new("WorktreeRenamed")).unwrap();
    index.add_path(&Path::new("WorktreeTypechange")).unwrap();
    index.add_path(&Path::new(".gitignore")).unwrap();

    let id = index.write_tree().unwrap();

    let tree = repo.find_tree(id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial files", &tree, &[])
        .unwrap();

    // Modify some files that will differ between HEAD and index
    fs::write(path.join("BothModified"), "Modified in index").unwrap();
    fs::write(path.join("IndexModified"), "IndexModified-content2").unwrap();
    fs::write(path.join("IndexNew"), "IndexNew").unwrap();
    fs::remove_file(path.join("IndexDeleted")).unwrap();
    fs::remove_file(path.join("IndexTypechange")).unwrap();

    fn create_symlink(to: &Path, from: &Path) {
        #[cfg(unix)]
        std::os::unix::fs::symlink(to, from).unwrap();

        #[cfg(windows)]
        std::os::windows::fs::symlink_file(to, from).unwrap();
    }
    create_symlink(&path.join("Unchanged"), &path.join("IndexTypechange"));

    fs::rename(path.join("IndexRenamed"), path.join("IndexRenamed-new")).unwrap();

    index.add_path(&Path::new("BothModified")).unwrap();
    index.remove_path(&Path::new("IndexDeleted")).unwrap();
    index.add_path(&Path::new("IndexModified")).unwrap();
    index.add_path(&Path::new("IndexNew")).unwrap();
    index.remove_path(&Path::new("IndexRenamed")).unwrap();
    index.add_path(&Path::new("IndexRenamed-new")).unwrap();
    index.add_path(&Path::new("IndexTypechange")).unwrap();

    // And between index and worktree
    fs::write(path.join("ignored-random"), "ignored-random").unwrap();
    fs::write(path.join("BothModified"), "Modified in worktree").unwrap();
    fs::remove_file(path.join("WorktreeDeleted")).unwrap();
    fs::write(path.join("WorktreeModified"), "New content").unwrap();
    fs::write(path.join("WorktreeNew"), "WorktreeNew").unwrap();
    fs::remove_file(path.join("WorktreeTypechange")).unwrap();
    fs::rename(
        path.join("WorktreeRenamed"),
        path.join("WorktreeRenamed-new"),
    )
    .unwrap();

    create_symlink(&path.join("Unchanged"), &path.join("WorktreeTypechange"));

    let mut opts = StatusOptions::new();
    opts.renames_head_to_index(true);
    opts.renames_index_to_workdir(true);
    opts.include_untracked(true);
    opts.include_unmodified(true);
    opts.include_ignored(true);
    let status = repo.statuses(Some(&mut opts)).unwrap();

    #[derive(PartialEq, Eq, PartialOrd, Ord, Debug)]
    struct SimpleEntry {
        path: String,
        status: String,
    }
    let mut entries: Vec<SimpleEntry> = vec![];
    for entry in status.iter() {
        entries.push({
            SimpleEntry {
                path: entry.path().unwrap().to_string(),
                status: format!("{:?}", entry.status()),
            }
        });
    }
    entries.sort();
    macro_rules! expected {
        ($($path:literal -> $status:expr,)+) => {
            vec![
                $(SimpleEntry { path: $path.to_string(), status: concat!("Status(", stringify!($status), ")").to_string() }),+
            ]
        }
    }
    // Does not cover WT_UNREADABLE which is rare, or CONFLICTED which is for
    // in-progress conflicts
    // Doesn't show all combinations of index and worktree changes, just a few
    assert_eq!(
        expected!(
            // Status::CURRENT
            ".gitignore" -> 0x0,
            "BothModified" -> INDEX_MODIFIED | WT_MODIFIED,
            "IndexDeleted" -> INDEX_DELETED,
            "IndexModified" -> INDEX_MODIFIED,
            "IndexNew" -> INDEX_NEW,
            "IndexRenamed" -> INDEX_RENAMED,
            "IndexTypechange" -> INDEX_TYPECHANGE,
            // Status::CURRENT
            "Unchanged" -> 0x0,
            "WorktreeDeleted" -> WT_DELETED,
            "WorktreeModified" -> WT_MODIFIED,
            "WorktreeNew" -> WT_NEW,
            "WorktreeRenamed" -> WT_RENAMED,
            "WorktreeTypechange" -> WT_TYPECHANGE,
            "ignored-random" -> IGNORED,
        ),
        entries
    );
}

#[test]
fn repo_status_clean() {
    let td = TempDir::new().unwrap();
    let path = td.path();

    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(path, &opts).unwrap();

    let mut config = repo.config().unwrap();
    config.set_str("user.name", "name").unwrap();
    config.set_str("user.email", "email").unwrap();

    // Create some files
    fs::write(path.join("MyFile"), "content").unwrap();

    let mut index = repo.index().unwrap();
    index.add_path(&Path::new("MyFile")).unwrap();

    let id = index.write_tree().unwrap();

    let tree = repo.find_tree(id).unwrap();
    let sig = repo.signature().unwrap();
    repo.commit(Some("HEAD"), &sig, &sig, "Initial files", &tree, &[])
        .unwrap();

    // A repo status is "clean" if
    // - there are no untracked files
    // - there are no modified files, in either the index or worktree
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);

    // Repo is clean:
    {
        let status = repo.statuses(Some(&mut opts)).unwrap();
        assert_eq!(0, status.len());
    }

    fs::write(path.join("OtherFile"), "content").unwrap();

    // Repo is dirty due to the untracked file
    {
        let status = repo.statuses(Some(&mut opts)).unwrap();
        assert_eq!(1, status.len());
        let entry = status.get(0).unwrap();
        assert_eq!(Some("OtherFile"), entry.path());
        assert_eq!(Status::WT_NEW, entry.status());
    }

    // Add it to the index
    index.add_path(&Path::new("OtherFile")).unwrap();

    // Still dirty
    {
        let status = repo.statuses(Some(&mut opts)).unwrap();
        assert_eq!(1, status.len());
        let entry = status.get(0).unwrap();
        assert_eq!(Some("OtherFile"), entry.path());
        assert_eq!(Status::INDEX_NEW, entry.status());
    }

    // Remove the file,
    fs::remove_file(path.join("OtherFile")).unwrap();

    // Still dirty because it is in the index
    {
        let status = repo.statuses(Some(&mut opts)).unwrap();
        assert_eq!(1, status.len());
        let entry = status.get(0).unwrap();
        assert_eq!(Some("OtherFile"), entry.path());
        assert_eq!(Status::INDEX_NEW | Status::WT_DELETED, entry.status());
    }

    // After removing from the index, it should be clean again
    index.remove_path(&Path::new("OtherFile")).unwrap();

    {
        let status = repo.statuses(Some(&mut opts)).unwrap();
        assert_eq!(0, status.len());
    }
}
