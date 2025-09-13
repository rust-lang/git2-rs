#![macro_use]

use crate::Error;

macro_rules! call {
    (raw::$p:ident ($($e:expr),*)) => (
        raw::$p($(crate::call::convert(&$e)),*)
    )
}

macro_rules! try_call {
    (raw::$p:ident ($($e:expr),*)) => ({
        match crate::call::c_try(raw::$p($(crate::call::convert(&$e)),*)) {
            Ok(o) => o,
            Err(e) => { crate::panic::check(); return Err(e) }
        }
    })
}

macro_rules! try_call_iter {
    ($($f:tt)*) => {
        match call!($($f)*) {
            0 => {}
            raw::GIT_ITEROVER => return None,
            e => return Some(Err(crate::call::last_error(e)))
        }
    }
}

#[doc(hidden)]
pub trait Convert<T> {
    fn convert(&self) -> T;
}

pub fn convert<T, U: Convert<T>>(u: &U) -> T {
    u.convert()
}

pub fn c_try(ret: libc::c_int) -> Result<libc::c_int, Error> {
    match ret {
        n if n < 0 => Err(last_error(n)),
        n => Ok(n),
    }
}

pub fn last_error(code: libc::c_int) -> Error {
    Error::last_error(code)
}

mod impls {
    use std::ffi::CString;
    use std::ptr;

    use crate::call::Convert;
    use crate::{raw, BranchType, ConfigLevel, Direction, ObjectType, ResetType};
    use crate::{
        AutotagOption, DiffFormat, FetchPrune, FileFavor, SubmoduleIgnore, SubmoduleUpdate,
    };

    impl<T: Copy> Convert<T> for T {
        fn convert(&self) -> T {
            *self
        }
    }

    impl Convert<libc::c_int> for bool {
        fn convert(&self) -> libc::c_int {
            *self as libc::c_int
        }
    }
    impl<T> Convert<*const T> for &T {
        fn convert(&self) -> *const T {
            *self as *const T
        }
    }
    impl<T> Convert<*mut T> for &mut T {
        fn convert(&self) -> *mut T {
            &**self as *const T as *mut T
        }
    }
    impl<T> Convert<*const T> for *mut T {
        fn convert(&self) -> *const T {
            *self as *const T
        }
    }

    impl Convert<*const libc::c_char> for CString {
        fn convert(&self) -> *const libc::c_char {
            self.as_ptr()
        }
    }

    impl<T, U: Convert<*const T>> Convert<*const T> for Option<U> {
        fn convert(&self) -> *const T {
            self.as_ref().map(|s| s.convert()).unwrap_or(ptr::null())
        }
    }

    impl<T, U: Convert<*mut T>> Convert<*mut T> for Option<U> {
        fn convert(&self) -> *mut T {
            self.as_ref()
                .map(|s| s.convert())
                .unwrap_or(ptr::null_mut())
        }
    }

    impl Convert<raw::git_reset_t> for ResetType {
        fn convert(&self) -> raw::git_reset_t {
            match *self {
                Self::Soft => raw::GIT_RESET_SOFT,
                Self::Hard => raw::GIT_RESET_HARD,
                Self::Mixed => raw::GIT_RESET_MIXED,
            }
        }
    }

    impl Convert<raw::git_direction> for Direction {
        fn convert(&self) -> raw::git_direction {
            match *self {
                Self::Push => raw::GIT_DIRECTION_PUSH,
                Self::Fetch => raw::GIT_DIRECTION_FETCH,
            }
        }
    }

    impl Convert<raw::git_object_t> for ObjectType {
        fn convert(&self) -> raw::git_object_t {
            match *self {
                Self::Any => raw::GIT_OBJECT_ANY,
                Self::Commit => raw::GIT_OBJECT_COMMIT,
                Self::Tree => raw::GIT_OBJECT_TREE,
                Self::Blob => raw::GIT_OBJECT_BLOB,
                Self::Tag => raw::GIT_OBJECT_TAG,
            }
        }
    }

    impl Convert<raw::git_object_t> for Option<ObjectType> {
        fn convert(&self) -> raw::git_object_t {
            self.unwrap_or(ObjectType::Any).convert()
        }
    }

    impl Convert<raw::git_branch_t> for BranchType {
        fn convert(&self) -> raw::git_branch_t {
            match *self {
                Self::Remote => raw::GIT_BRANCH_REMOTE,
                Self::Local => raw::GIT_BRANCH_LOCAL,
            }
        }
    }

    impl Convert<raw::git_branch_t> for Option<BranchType> {
        fn convert(&self) -> raw::git_branch_t {
            self.map(|s| s.convert()).unwrap_or(raw::GIT_BRANCH_ALL)
        }
    }

    impl Convert<raw::git_config_level_t> for ConfigLevel {
        fn convert(&self) -> raw::git_config_level_t {
            match *self {
                Self::ProgramData => raw::GIT_CONFIG_LEVEL_PROGRAMDATA,
                Self::System => raw::GIT_CONFIG_LEVEL_SYSTEM,
                Self::XDG => raw::GIT_CONFIG_LEVEL_XDG,
                Self::Global => raw::GIT_CONFIG_LEVEL_GLOBAL,
                Self::Local => raw::GIT_CONFIG_LEVEL_LOCAL,
                Self::Worktree => raw::GIT_CONFIG_LEVEL_WORKTREE,
                Self::App => raw::GIT_CONFIG_LEVEL_APP,
                Self::Highest => raw::GIT_CONFIG_HIGHEST_LEVEL,
            }
        }
    }

    impl Convert<raw::git_diff_format_t> for DiffFormat {
        fn convert(&self) -> raw::git_diff_format_t {
            match *self {
                Self::Patch => raw::GIT_DIFF_FORMAT_PATCH,
                Self::PatchHeader => raw::GIT_DIFF_FORMAT_PATCH_HEADER,
                Self::Raw => raw::GIT_DIFF_FORMAT_RAW,
                Self::NameOnly => raw::GIT_DIFF_FORMAT_NAME_ONLY,
                Self::NameStatus => raw::GIT_DIFF_FORMAT_NAME_STATUS,
                Self::PatchId => raw::GIT_DIFF_FORMAT_PATCH_ID,
            }
        }
    }

    impl Convert<raw::git_merge_file_favor_t> for FileFavor {
        fn convert(&self) -> raw::git_merge_file_favor_t {
            match *self {
                Self::Normal => raw::GIT_MERGE_FILE_FAVOR_NORMAL,
                Self::Ours => raw::GIT_MERGE_FILE_FAVOR_OURS,
                Self::Theirs => raw::GIT_MERGE_FILE_FAVOR_THEIRS,
                Self::Union => raw::GIT_MERGE_FILE_FAVOR_UNION,
            }
        }
    }

    impl Convert<raw::git_submodule_ignore_t> for SubmoduleIgnore {
        fn convert(&self) -> raw::git_submodule_ignore_t {
            match *self {
                Self::Unspecified => raw::GIT_SUBMODULE_IGNORE_UNSPECIFIED,
                Self::None => raw::GIT_SUBMODULE_IGNORE_NONE,
                Self::Untracked => raw::GIT_SUBMODULE_IGNORE_UNTRACKED,
                Self::Dirty => raw::GIT_SUBMODULE_IGNORE_DIRTY,
                Self::All => raw::GIT_SUBMODULE_IGNORE_ALL,
            }
        }
    }

    impl Convert<raw::git_submodule_update_t> for SubmoduleUpdate {
        fn convert(&self) -> raw::git_submodule_update_t {
            match *self {
                Self::Checkout => raw::GIT_SUBMODULE_UPDATE_CHECKOUT,
                Self::Rebase => raw::GIT_SUBMODULE_UPDATE_REBASE,
                Self::Merge => raw::GIT_SUBMODULE_UPDATE_MERGE,
                Self::None => raw::GIT_SUBMODULE_UPDATE_NONE,
                Self::Default => raw::GIT_SUBMODULE_UPDATE_DEFAULT,
            }
        }
    }

    impl Convert<raw::git_remote_autotag_option_t> for AutotagOption {
        fn convert(&self) -> raw::git_remote_autotag_option_t {
            match *self {
                Self::Unspecified => raw::GIT_REMOTE_DOWNLOAD_TAGS_UNSPECIFIED,
                Self::None => raw::GIT_REMOTE_DOWNLOAD_TAGS_NONE,
                Self::Auto => raw::GIT_REMOTE_DOWNLOAD_TAGS_AUTO,
                Self::All => raw::GIT_REMOTE_DOWNLOAD_TAGS_ALL,
            }
        }
    }

    impl Convert<raw::git_fetch_prune_t> for FetchPrune {
        fn convert(&self) -> raw::git_fetch_prune_t {
            match *self {
                Self::Unspecified => raw::GIT_FETCH_PRUNE_UNSPECIFIED,
                Self::On => raw::GIT_FETCH_PRUNE,
                Self::Off => raw::GIT_FETCH_NO_PRUNE,
            }
        }
    }
}
