use std::path::{Path, PathBuf};
use std::io;
#[cfg(unix)]
use std::ptr;
use tempdir::TempDir;
use url::Url;

use Repository;

macro_rules! t {
    ($e:expr) => (match $e {
        Ok(e) => e,
        Err(e) => panic!("{} failed with {}", stringify!($e), e),
    })
}

pub fn repo_init() -> (TempDir, Repository) {
    let td = TempDir::new("test").unwrap();
    let repo = Repository::init(td.path()).unwrap();
    {
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "name").unwrap();
        config.set_str("user.email", "email").unwrap();
        let mut index = repo.index().unwrap();
        let id = index.write_tree().unwrap();

        let tree = repo.find_tree(id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial",
                    &tree, &[]).unwrap();
    }
    (td, repo)
}

pub fn path2url(path: &Path) -> String {
    Url::from_file_path(path).unwrap().to_string()
}

#[cfg(windows)]
pub fn realpath(original: &Path) -> io::Result<PathBuf> {
    Ok(original.to_path_buf())
}
#[cfg(unix)]
pub fn realpath(original: &Path) -> io::Result<PathBuf> {
    use std::ffi::{CStr, OsString, CString};
    use std::os::unix::prelude::*;
    use libc::{self, c_char};
    extern {
        fn realpath(name: *const c_char, resolved: *mut c_char) -> *mut c_char;
    }
    unsafe {
        let cstr = try!(CString::new(original.as_os_str().as_bytes()));
        let ptr = realpath(cstr.as_ptr(), ptr::null_mut());
        if ptr.is_null() {
            return Err(io::Error::last_os_error())
        }
        let bytes = CStr::from_ptr(ptr).to_bytes().to_vec();
        libc::free(ptr as *mut _);
        Ok(PathBuf::from(OsString::from_vec(bytes)))
    }
}
