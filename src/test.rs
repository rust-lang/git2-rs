use std::fs::File;
use std::io;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::ptr;
use tempfile::TempDir;
use url::Url;

use crate::{Branch, Oid, Repository, RepositoryInitOptions};

macro_rules! t {
    ($e:expr) => {
        match $e {
            Ok(e) => e,
            Err(e) => panic!("{} failed with {}", stringify!($e), e),
        }
    };
}

pub fn repo_init() -> (TempDir, Repository) {
    let td = TempDir::new().unwrap();
    let mut opts = RepositoryInitOptions::new();
    opts.initial_head("main");
    let repo = Repository::init_opts(td.path(), &opts).unwrap();
    {
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
    (td, repo)
}

pub fn commit(repo: &Repository) -> (Oid, Oid) {
    let mut index = t!(repo.index());
    let root = repo.path().parent().unwrap();
    t!(File::create(&root.join("foo")));
    t!(index.add_path(Path::new("foo")));

    let tree_id = t!(index.write_tree());
    let tree = t!(repo.find_tree(tree_id));
    let sig = t!(repo.signature());
    let head_id = t!(repo.refname_to_id("HEAD"));
    let parent = t!(repo.find_commit(head_id));
    let commit = t!(repo.commit(Some("HEAD"), &sig, &sig, "commit", &tree, &[&parent]));
    (commit, tree_id)
}

pub fn path2url(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}

pub fn worktrees_env_init(repo: &Repository) -> (TempDir, Branch<'_>) {
    let oid = repo.head().unwrap().target().unwrap();
    let commit = repo.find_commit(oid).unwrap();
    let branch = repo.branch("wt-branch", &commit, true).unwrap();
    let wtdir = TempDir::new().unwrap();
    (wtdir, branch)
}

#[cfg(windows)]
pub fn realpath(original: &Path) -> io::Result<PathBuf> {
    Ok(original.canonicalize()?.to_path_buf())
}
#[cfg(unix)]
pub fn realpath(original: &Path) -> io::Result<PathBuf> {
    use libc::c_char;
    use std::ffi::{CStr, CString, OsString};
    use std::os::unix::prelude::*;
    extern "C" {
        fn realpath(name: *const c_char, resolved: *mut c_char) -> *mut c_char;
    }
    unsafe {
        let cstr = CString::new(original.as_os_str().as_bytes())?;
        let ptr = realpath(cstr.as_ptr(), ptr::null_mut());
        if ptr.is_null() {
            return Err(io::Error::last_os_error());
        }
        let bytes = CStr::from_ptr(ptr).to_bytes().to_vec();
        libc::free(ptr as *mut _);
        Ok(PathBuf::from(OsString::from_vec(bytes)))
    }
}
