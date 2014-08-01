use {raw, Revspec, Error, doit, init, Object};

pub struct Repository {
    repo: *mut raw::git_repository,
}

impl Repository {
    pub fn open<T: ToCStr>(path: T) -> Result<Repository, Error> {
        init();
        let s = path.to_c_str();
        let mut ret = 0 as *mut raw::git_repository;
        try!(doit(|| unsafe {
            raw::git_repository_open(&mut ret, s.as_ptr())
        }));
        Ok(Repository { repo: ret })
    }

    pub fn revparse(&self, spec: &str) -> Result<Revspec, Error> {
        let s = spec.to_c_str();
        let mut spec = raw::git_revspec {
            from: 0 as *mut _,
            to: 0 as *mut _,
            flags: raw::git_revparse_mode_t::empty(),
        };
        try!(doit(|| unsafe {
            raw::git_revparse(&mut spec, self.repo, s.as_ptr())
        }));

        if spec.flags.contains(raw::GIT_REVPARSE_SINGLE) {
            assert!(spec.to.is_null());
            Ok(Revspec::from_objects(Some(unsafe { Object::from_raw(spec.from) }),
                                     None))
        } else {
            fail!()
        }
    }
}

impl Drop for Repository {
    fn drop(&mut self) {
        unsafe {
            raw::git_repository_free(self.repo);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::{TempDir, Command, File};
    use std::str;

    use super::Repository;

    macro_rules! git( ( $cwd:expr, $($arg:expr),*) => ({
        let out = Command::new("git").cwd($cwd) $(.arg($arg))* .output().unwrap();
        assert!(out.status.success());
        str::from_utf8(out.output.as_slice()).unwrap().trim().to_string()
    }) )

    #[test]
    fn smoke_open() {
        let td = TempDir::new("test").unwrap();
        let path = td.path();
        git!(td.path(), "init");

        Repository::open(path).unwrap();
    }

    #[test]
    fn smoke_revparse() {
        let td = TempDir::new("test").unwrap();
        git!(td.path(), "init");
        File::create(&td.path().join("foo")).write_str("foobar").unwrap();
        git!(td.path(), "add", ".");
        git!(td.path(), "commit", "-m", "foo");
        let expected_rev = git!(td.path(), "rev-parse", "HEAD");

        let repo = Repository::open(td.path()).unwrap();
        let actual_rev = repo.revparse("HEAD").unwrap();
        let from = actual_rev.from().unwrap();
        assert!(actual_rev.to().is_none());
        assert_eq!(expected_rev, from.id().to_string());
    }
}
