extern crate "libgit2-sys" as raw;

use std::fmt;

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

/// A structure to represent the git status
pub struct RepoStatus {
    /// The path of the file
    pub path: String,
    /// Is the file ignored
    pub is_ignored: bool,
    /// State of the staged changes
    pub indexed_state: FileState,
    /// State of the unstaged changes
    pub working_state: FileState,
}

impl RepoStatus {
    /// Initializes an instance of RepoStatus
    pub fn new(flags: RepoStatusFlags) -> RepoStatus {
        RepoStatus {
            path: "".to_string(),
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
            match self.indexed_state {
                FileState::Clean => " ",
                FileState::Deleted => "D",
                FileState::Modified => "M",
                FileState::New => "A",
                FileState::Renamed => "R",
                FileState::TypeChange => "T",
                FileState::Untracked => "?"
            },

            match self.working_state {
                FileState::Clean => " ",
                FileState::Deleted => "D",
                FileState::Modified => "M",
                FileState::New => "A",
                FileState::Renamed => "R",
                FileState::TypeChange => "T",
                FileState::Untracked => "?"
            },

            self.path)
    }
}
