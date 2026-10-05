use std::io;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace root does not exist")]
    RootMissing,
    #[error("workspace path is outside the workspace root")]
    PathOutsideRoot,
    #[error("invalid workspace path")]
    InvalidPath,
    #[error("path is a directory")]
    IsDirectory,
    #[error("path is not a directory")]
    NotDirectory,
    #[error("path already exists: {0}")]
    AlreadyExists(String),
    #[error("path does not exist: {0}")]
    NotFound(String),
    #[error("not a git repository")]
    NotGitRepository,
    #[error("UTF-8 text file required")]
    NotText,
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("watcher error: {0}")]
    Watcher(String),
    #[error("git error: {0}")]
    Git(String),
    #[error("file changed since the caller last read it: {path} (expected {expected}, actual {actual})")]
    VersionConflict { path: String, expected: u64, actual: u64 },
    #[error("invalid patch: {0}")]
    InvalidPatch(String),
    #[error("path is protected and cannot be modified: {0}")]
    Protected(String),
    #[error("content exceeds the write limit of {limit} bytes")]
    TooLarge { limit: usize },
}

impl From<crate::path_guard::GuardError> for WorkspaceError {
    fn from(e: crate::path_guard::GuardError) -> Self {
        match e {
            crate::path_guard::GuardError::Outside => WorkspaceError::PathOutsideRoot,
            crate::path_guard::GuardError::Invalid => WorkspaceError::InvalidPath,
        }
    }
}

pub type Result<T> = std::result::Result<T, WorkspaceError>;
