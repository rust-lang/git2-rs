use crate::buf::Buf;
use crate::reference::Reference;
use crate::repo::Repository;
use crate::util::{self, Binding};
use crate::{call, raw, Error};
use std::mem;
use std::path::Path;
use std::ptr;
use std::str;

/// An owned git worktree
///
/// This structure corresponds to a `git_worktree` in libgit2.
//
pub struct Worktree {
    raw: *mut raw::git_worktree,
}

// It is the current belief that a `Worktree` can be sent among threads, or
// even shared among threads in a mutex
unsafe impl Send for Worktree {}

/// Options which can be used to configure how a repository is initialized
pub struct WorktreeAddOptions<'a> {
    lock: i32,
    reference: Option<Reference<'a>>,
}

/// Options to configure how worktree pruning is performed
pub struct WorktreePruneOptions {
    flags: u32,
}

/// Lock Status of a worktree
#[derive(PartialEq, Debug)]
pub enum WorktreeLockStatus {
    /// Worktree is Unlocked
    Unlocked,
    /// Worktree is locked with the optional message
    Locked(Option<String>),
}

impl Worktree {
    /// Retrieves the name of the worktree
    ///
    /// This is the name that can be passed to repo::Repository::worktree_lookup
    /// to reopen the worktree. This is also the name that would appear in the
    /// list returned by repo::Repository::worktrees
    pub fn name(&self) -> Option<&str> {
        unsafe {
            crate::opt_bytes(self, raw::git_worktree_name(self.raw))
                .and_then(|s| str::from_utf8(s).ok())
        }
    }

    /// Retrieves the path to the worktree
    ///
    /// This is the path to the top-level of the source and not the path to the
    /// .git file within the worktree. This path can be passed to
    /// repo::Repository::open.
    pub fn path(&self) -> &Path {
        unsafe {
            util::bytes2path(crate::opt_bytes(self, raw::git_worktree_path(self.raw)).unwrap())
        }
    }

    /// Validates the worktree
    ///
    /// This checks that it still exists on the
    /// filesystem and that the metadata is correct
    pub fn validate(&self) -> Result<(), Error> {
        unsafe {
            call::c_try(raw::git_worktree_validate(call::convert(&self.raw)))?;
        }
        Ok(())
    }

    /// Locks the worktree
    pub fn lock(&self, reason: Option<&str>) -> Result<(), Error> {
        let reason = crate::opt_cstr(reason)?;
        unsafe {
            try_call!(raw::git_worktree_lock(self.raw, reason));
        }
        Ok(())
    }

    /// Unlocks the worktree
    pub fn unlock(&self) -> Result<(), Error> {
        unsafe {
            call::c_try(raw::git_worktree_unlock(call::convert(&self.raw)))?;
        }
        Ok(())
    }

    /// Checks if worktree is locked
    pub fn is_locked(&self) -> Result<WorktreeLockStatus, Error> {
        let buf = Buf::new();
        unsafe {
            match try_call!(raw::git_worktree_is_locked(buf.raw(), self.raw)) {
                0 => Ok(WorktreeLockStatus::Unlocked),
                _ => {
                    println!("Buf: {}", buf.as_str().unwrap());
                    let v = buf.to_vec();
                    println!("Length of v: {}", v.len());
                    Ok(WorktreeLockStatus::Locked(match v.len() {
                        0 => None,
                        _ => String::from_utf8(v).ok(),
                    }))
                }
            }
        }
    }

    /// Prunes the worktree
    pub fn prune(&self, opts: WorktreePruneOptions) -> Result<(), Error> {
        // When successful the worktree should be removed however the backing structure
        // of the git_worktree should still be valid.
        unsafe {
            try_call!(raw::git_worktree_prune(self.raw, &mut opts.raw()));
        }
        Ok(())
    }

    /// Checks if the worktree is prunable
    pub fn is_prunable(&self, opts: WorktreePruneOptions) -> bool {
        unsafe {
            if call!(raw::git_worktree_is_prunable(self.raw, &mut opts.raw())) <= 0 {
                false
            } else {
                true
            }
        }
    }

    /// Opens the repository from the worktree
    pub fn open_repository(&self) -> Result<Repository, Error> {
        let mut ret = ptr::null_mut();
        unsafe {
            try_call!(raw::git_repository_open_from_worktree(&mut ret, self.raw));
            Ok(Binding::from_raw(ret))
        }
    }
}

impl<'a> WorktreeAddOptions<'a> {
    /// Creates a default set of add options.
    ///
    /// By default this will not lock the worktree
    pub fn new(reference: Option<Reference<'a>>) -> WorktreeAddOptions<'a> {
        WorktreeAddOptions {
            lock: 0,
            reference: reference,
        }
    }

    /// If enabled, this will cause the newly added worktree to be locked
    pub fn lock(&mut self, enabled: bool) -> &mut WorktreeAddOptions<'a> {
        self.lock = match enabled {
            true => 1,
            false => 0,
        };
        self
    }

    /// Creates a set of raw add options to be used with `git_worktree_add`
    ///
    /// This method is unsafe as the returned value may have pointers to the
    /// interior of this structure
    pub unsafe fn raw(&self) -> raw::git_worktree_add_options {
        let mut opts = mem::zeroed();
        assert_eq!(
            raw::git_worktree_add_options_init(&mut opts, raw::GIT_WORKTREE_ADD_OPTIONS_VERSION),
            0
        );

        opts.lock = self.lock;
        opts.reference = if let Some(ref gref) = self.reference {
            gref.raw()
        } else {
            ptr::null_mut()
        };

        opts
    }
}

impl WorktreePruneOptions {
    /// Creates a default set of pruning options
    ///
    /// By defaults this will prune only worktrees that are no longer valid
    /// unlocked and not checked out
    pub fn new() -> WorktreePruneOptions {
        WorktreePruneOptions { flags: 0 }
    }

    /// Controls whether valid (still existing on the filesystem) worktrees
    /// will be pruned
    ///
    /// Defaults to false
    pub fn valid(&mut self, valid: bool) -> &mut WorktreePruneOptions {
        self.flag(raw::GIT_WORKTREE_PRUNE_VALID, valid)
    }

    /// Controls whether locked worktrees will be pruned
    ///
    /// Defaults to false
    pub fn locked(&mut self, locked: bool) -> &mut WorktreePruneOptions {
        self.flag(raw::GIT_WORKTREE_PRUNE_LOCKED, locked)
    }

    /// Controls whether the actual working tree on the fs is recursively removed
    ///
    /// Defaults to false
    pub fn working_tree(&mut self, working_tree: bool) -> &mut WorktreePruneOptions {
        self.flag(raw::GIT_WORKTREE_PRUNE_WORKING_TREE, working_tree)
    }

    fn flag(&mut self, flag: raw::git_worktree_prune_t, on: bool) -> &mut WorktreePruneOptions {
        if on {
            self.flags |= flag as u32;
        } else {
            self.flags &= !(flag as u32);
        }
        self
    }
    /// Creates a set of raw prune options to be used with `git_worktree_prune`
    ///
    /// This method is unsafe as the returned value may have pointers to the
    /// interior of this structure
    pub unsafe fn raw(&self) -> raw::git_worktree_prune_options {
        let mut opts = mem::zeroed();
        assert_eq!(
            raw::git_worktree_prune_options_init(
                &mut opts,
                raw::GIT_WORKTREE_PRUNE_OPTIONS_VERSION
            ),
            0
        );

        opts.flags = self.flags;
        opts
    }
}

impl Binding for Worktree {
    type Raw = *mut raw::git_worktree;
    unsafe fn from_raw(ptr: *mut raw::git_worktree) -> Worktree {
        Worktree { raw: ptr }
    }
    fn raw(&self) -> *mut raw::git_worktree {
        self.raw
    }
}

impl Drop for Worktree {
    fn drop(&mut self) {
        unsafe { raw::git_worktree_free(self.raw) }
    }
}

#[cfg(test)]
mod tests {
    use crate::WorktreeAddOptions;
    use crate::WorktreeLockStatus;
    use tempfile::TempDir;

    #[test]
    fn smoke_add_no_ref() {
        let (_td, repo) = crate::test::repo_init();
        let wtdir = TempDir::new().unwrap();
        let wt_path = wtdir.path().join("tree-no-ref-dir");
        let opts = WorktreeAddOptions::new(None);

        let wt = repo.worktree_add("tree-no-ref", &wt_path, &opts).unwrap();
        assert_eq!(wt.name(), Some("tree-no-ref"));
        assert_eq!(
            wt.path().canonicalize().unwrap(),
            wt_path.canonicalize().unwrap()
        );
        let status = wt.is_locked().unwrap();
        assert_eq!(status, WorktreeLockStatus::Unlocked);
    }

    #[test]
    fn smoke_add_locked() {
        let (_td, repo) = crate::test::repo_init();
        let wtdir = TempDir::new().unwrap();
        let wt_path = wtdir.path().join("locked-tree");
        let mut opts = WorktreeAddOptions::new(None);
        opts.lock(true);

        let wt = repo.worktree_add("locked-tree", &wt_path, &opts).unwrap();
        // shouldn't be able to lock a worktree that was created locked
        assert!(wt.lock(Some("my reason")).is_err());
        assert_eq!(wt.name(), Some("locked-tree"));
        assert_eq!(
            wt.path().canonicalize().unwrap(),
            wt_path.canonicalize().unwrap()
        );
        assert_eq!(wt.is_locked().unwrap(), WorktreeLockStatus::Locked(None));
        assert!(wt.unlock().is_ok());
        assert!(wt.lock(Some("my reason")).is_ok());
        assert_eq!(
            wt.is_locked().unwrap(),
            WorktreeLockStatus::Locked(Some("my reason".to_string()))
        );
    }

    #[test]
    fn smoke_add_from_branch() {
        let (_td, repo) = crate::test::repo_init();
        let (wt_top, branch) = crate::test::worktrees_env_init(&repo);
        let wt_path = wt_top.path().join("test");
        let opts = WorktreeAddOptions::new(Some(branch.into_reference()));

        let wt = repo.worktree_add("test-worktree", &wt_path, &opts).unwrap();
        assert_eq!(wt.name(), Some("test-worktree"));
        assert_eq!(
            wt.path().canonicalize().unwrap(),
            wt_path.canonicalize().unwrap()
        );
        let status = wt.is_locked().unwrap();
        assert_eq!(status, WorktreeLockStatus::Unlocked);
    }
}
