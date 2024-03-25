use crate::buf::Buf;
use crate::reference::Reference;
use crate::repo::Repository;
use crate::util::{self, Binding};
use crate::{raw, Error};
use std::os::raw::c_int;
use std::path::Path;
use std::ptr;
use std::str;
use std::{marker, mem};

/// An owned git worktree
///
/// This structure corresponds to a `git_worktree` in libgit2.
//
pub struct Worktree {
    raw: *mut raw::git_worktree,
}

/// Options which can be used to configure how a worktree is initialized
pub struct WorktreeAddOptions<'a> {
    raw: raw::git_worktree_add_options,
    _marker: marker::PhantomData<Reference<'a>>,
}

/// Options to configure how worktree pruning is performed
pub struct WorktreePruneOptions {
    raw: raw::git_worktree_prune_options,
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
    /// Open a worktree of a the repository
    ///
    /// If a repository is not the main tree but a worktree, this
    /// function will look up the worktree inside the parent
    /// repository and create a new `git_worktree` structure.
    pub fn open_from_repository(repo: &Repository) -> Result<Worktree, Error> {
        let mut raw = ptr::null_mut();
        unsafe {
            try_call!(raw::git_worktree_open_from_repository(&mut raw, repo.raw()));
            Ok(Binding::from_raw(raw))
        }
    }

    /// Retrieves the name of the worktree
    ///
    /// This is the name that can be passed to repo::Repository::find_worktree
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
            try_call!(raw::git_worktree_validate(self.raw));
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
            try_call!(raw::git_worktree_unlock(self.raw));
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
                    let v = buf.to_vec();
                    Ok(WorktreeLockStatus::Locked(match v.len() {
                        0 => None,
                        _ => Some(String::from_utf8(v).unwrap()),
                    }))
                }
            }
        }
    }

    /// Prunes the worktree
    pub fn prune(&self, opts: Option<&mut WorktreePruneOptions>) -> Result<(), Error> {
        // When successful the worktree should be removed however the backing structure
        // of the git_worktree should still be valid.
        unsafe {
            try_call!(raw::git_worktree_prune(self.raw, opts.map(|o| o.raw())));
        }
        Ok(())
    }

    /// Checks if the worktree is prunable
    pub fn is_prunable(&self, opts: Option<&mut WorktreePruneOptions>) -> Result<bool, Error> {
        unsafe {
            let rv = try_call!(raw::git_worktree_is_prunable(
                self.raw,
                opts.map(|o| o.raw())
            ));
            Ok(rv != 0)
        }
    }
}

impl<'a> WorktreeAddOptions<'a> {
    /// Creates a default set of add options.
    ///
    /// By default this will not lock the worktree
    pub fn new() -> WorktreeAddOptions<'a> {
        unsafe {
            let mut raw = mem::zeroed();
            assert_eq!(
                raw::git_worktree_add_options_init(&mut raw, raw::GIT_WORKTREE_ADD_OPTIONS_VERSION),
                0
            );
            WorktreeAddOptions {
                raw,
                _marker: marker::PhantomData,
            }
        }
    }

    /// If enabled, this will cause the newly added worktree to be locked
    pub fn lock(&mut self, enabled: bool) -> &mut WorktreeAddOptions<'a> {
        self.raw.lock = enabled as c_int;
        self
    }

    /// If enabled, this will checkout the existing branch matching the worktree name.
    pub fn checkout_existing(&mut self, enabled: bool) -> &mut WorktreeAddOptions<'a> {
        self.raw.checkout_existing = enabled as c_int;
        self
    }

    /// reference to use for the new worktree HEAD
    pub fn reference(
        &mut self,
        reference: Option<&'a Reference<'_>>,
    ) -> &mut WorktreeAddOptions<'a> {
        self.raw.reference = if let Some(reference) = reference {
            reference.raw()
        } else {
            ptr::null_mut()
        };
        self
    }

    /// Get a set of raw add options to be used with `git_worktree_add`
    pub fn raw(&self) -> *const raw::git_worktree_add_options {
        &self.raw
    }
}

impl WorktreePruneOptions {
    /// Creates a default set of pruning options
    ///
    /// By defaults this will prune only worktrees that are no longer valid
    /// unlocked and not checked out
    pub fn new() -> WorktreePruneOptions {
        unsafe {
            let mut raw = mem::zeroed();
            assert_eq!(
                raw::git_worktree_prune_options_init(
                    &mut raw,
                    raw::GIT_WORKTREE_PRUNE_OPTIONS_VERSION
                ),
                0
            );
            WorktreePruneOptions { raw }
        }
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

    /// Controls whether the actual working tree on the filesystem is recursively removed
    ///
    /// Defaults to false
    pub fn working_tree(&mut self, working_tree: bool) -> &mut WorktreePruneOptions {
        self.flag(raw::GIT_WORKTREE_PRUNE_WORKING_TREE, working_tree)
    }

    fn flag(&mut self, flag: raw::git_worktree_prune_t, on: bool) -> &mut WorktreePruneOptions {
        if on {
            self.raw.flags |= flag as u32;
        } else {
            self.raw.flags &= !(flag as u32);
        }
        self
    }

    /// Get a set of raw prune options to be used with `git_worktree_prune`
    pub fn raw(&mut self) -> *mut raw::git_worktree_prune_options {
        &mut self.raw
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
        let opts = WorktreeAddOptions::new();

        let wt = repo.worktree("tree-no-ref", &wt_path, Some(&opts)).unwrap();
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
        let mut opts = WorktreeAddOptions::new();
        opts.lock(true);

        let wt = repo.worktree("locked-tree", &wt_path, Some(&opts)).unwrap();
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
        let mut opts = WorktreeAddOptions::new();
        let reference = branch.into_reference();
        opts.reference(Some(&reference));

        let wt = repo
            .worktree("test-worktree", &wt_path, Some(&opts))
            .unwrap();
        assert_eq!(wt.name(), Some("test-worktree"));
        assert_eq!(
            wt.path().canonicalize().unwrap(),
            wt_path.canonicalize().unwrap()
        );
        let status = wt.is_locked().unwrap();
        assert_eq!(status, WorktreeLockStatus::Unlocked);
    }
}
