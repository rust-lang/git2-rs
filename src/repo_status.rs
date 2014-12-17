use raw;
use std::fmt;
use std::c_str::CString;
use libc::c_char;

bitflags! {
    #[doc = "
Flags for repository status
"]
    flags RepoStatusFlags: u32 {
        const INDEX_NEW = raw::GIT_STATUS_INDEX_NEW as u32,
        const INDEX_MODIFIED = raw::GIT_STATUS_INDEX_MODIFIED as u32,
        const INDEX_DELETED = raw::GIT_STATUS_INDEX_DELETED as u32,
        const INDEX_RENAMED = raw::GIT_STATUS_INDEX_RENAMED as u32,
        const INDEX_TYPECHANGE = raw::GIT_STATUS_INDEX_TYPECHANGE as u32,

        const WT_NEW = raw::GIT_STATUS_WT_NEW as u32,
        const WT_MODIFIED = raw::GIT_STATUS_WT_MODIFIED as u32,
        const WT_DELETED = raw::GIT_STATUS_WT_DELETED as u32,
        const WT_TYPECHANGE = raw::GIT_STATUS_WT_TYPECHANGE as u32,
        const WT_RENAMED = raw::GIT_STATUS_WT_RENAMED as u32,

        const IGNORED = raw::GIT_STATUS_IGNORED as u32,
    }
}

/// Describes the state of a file
pub enum FileState {
    /// Nothing Changed
    Clean,
    /// File has been deleted
    Deleted,
    /// File has changed
    Modified,
    /// File was added
    New,
    /// File was renamed
    Renamed,
    /// File's type was changed
    TypeChange,
    /// Untracked
    Untracked,
}

impl fmt::Show for FileState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match *self {
            FileState::Clean => " ",
            FileState::Deleted => "D",
            FileState::Modified => "M",
            FileState::New => "A",
            FileState::Renamed => "R",
            FileState::TypeChange => "T",
            FileState::Untracked => "?"
        })
    }
}

pub struct DeltaFiles {
    pub old: Option<Path>,
    pub new: Option<Path>
}

impl DeltaFiles {
    pub fn new(delta: *mut raw::git_diff_delta) -> Option<DeltaFiles> {
        fn to_path(buf: *const c_char) -> Option<Path> {
            unsafe {
                match CString::new(buf, false).as_str() {
                    Some(p) => Some(Path::new(p)),
                    None => None
                }
            }
        }

        if delta != 0 as *mut raw::git_diff_delta {
            let d = unsafe { *delta };
            Some(DeltaFiles {
                old: to_path(d.old_file.path),
                new: to_path(d.new_file.path),
            })
        } else {
            None
        }
    }
}

impl fmt::Show for DeltaFiles {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", match (self.old.clone(), self.new.clone()) {
            (Some(o), Some(n)) => {
                if o != n {
                    format!("{} -> {}", o.as_str().unwrap(), n.as_str().unwrap())
                } else {
                    o.as_str().unwrap().to_string()
                }
            },
            (Some(o), None) => o.as_str().unwrap().to_string(),
            (None, Some(n)) => n.as_str().unwrap().to_string(),
            (None, None) => "".to_string()
        })
    }
}

/// A structure to represent the git status
pub struct RepoStatus {
    /// The filenames from head to index
    pub head_to_index: Option<DeltaFiles>,
    /// The filenames from index to workdir
    pub index_to_wd: Option<DeltaFiles>,
    /// Is the file ignored
    pub is_ignored: bool,
    /// State of the staged changes
    pub indexed_state: FileState,
    /// State of the unstaged changes
    pub working_state: FileState,
}


impl RepoStatus {
    /// Initializes an instance of RepoStatus
    pub fn new(s: raw::git_status_entry) -> RepoStatus {
        let flags = match RepoStatusFlags::from_bits(s.status as u32) {
            Some(flags) => flags,
            None => RepoStatusFlags::empty()
        };

        RepoStatus {
            head_to_index: DeltaFiles::new(s.head_to_index),
            index_to_wd: DeltaFiles::new(s.index_to_workdir),
            is_ignored: flags.contains(IGNORED),
            indexed_state:
                     if flags.contains(INDEX_RENAMED) { FileState::Renamed }
                else if flags.contains(INDEX_DELETED) { FileState::Deleted }
                else if flags.contains(INDEX_MODIFIED) { FileState::Modified }
                else if flags.contains(INDEX_NEW) { FileState::New }
                else if flags.contains(INDEX_TYPECHANGE) { FileState::TypeChange }
                else if flags.contains(WT_NEW) { FileState::Untracked }
                else { FileState::Clean },
            working_state:
                     if flags.contains(WT_RENAMED) { FileState::Renamed }
                else if flags.contains(WT_DELETED) { FileState::Deleted }
                else if flags.contains(WT_MODIFIED) { FileState::Modified }
                else if flags.contains(WT_NEW) { FileState::Untracked }
                else if flags.contains(WT_TYPECHANGE) { FileState::TypeChange }
                else { FileState::Clean }
        }
    }
}

impl fmt::Show for RepoStatus {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{} {}",
            self.indexed_state,
            self.working_state,
            match self.head_to_index {
                Some(ref files) => Some(files),
                None => match self.index_to_wd {
                    Some(ref files) => Some(files),
                    None => None
                }
            }.unwrap())
    }
}
