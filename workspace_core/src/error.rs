use thiserror::Error;

pub type Result<T> = std::result::Result<T, WorkspaceError>;

#[derive(Debug, Error)]
pub enum WorkspaceError {
    #[error("workspace root missing or inaccessible")]
    RootMissing,

    #[error("path is not a directory")]
    NotDirectory,

    #[error("path is a directory")]
    IsDirectory,

    #[error("invalid path")]
    InvalidPath,

    /// Host bridge uyumu: path workspace dışına çıkıyor.
    #[error("path outside workspace root")]
    PathOutsideRoot,

    /// Host bridge uyumu: ikili/null içerik.
    #[error("file is not valid text")]
    NotText,

    /// Host bridge uyumu: dış kaynak geçersiz.
    #[error("source invalid: {0}")]
    SourceInvalid(String),

    #[error("path not found: {0}")]
    NotFound(String),

    #[error("already exists: {0}")]
    AlreadyExists(String),

    #[error("protected path: {0}")]
    Protected(String),

    #[error("content too large (limit {limit} bytes)")]
    TooLarge { limit: usize },

    #[error("version conflict on {path}: expected {expected}, actual {actual}")]
    VersionConflict {
        path: String,
        expected: u64,
        actual: u64,
    },

    #[error("patch failed: {0}")]
    Patch(String),

    #[error("not a git repository")]
    NotGitRepository,

    #[error("git error: {0}")]
    Git(String),

    #[error("search error: {0}")]
    Search(String),

    #[error("regex error: {0}")]
    Regex(String),

    #[error("watcher error: {0}")]
    Watcher(String),

    #[error("lsp error: {0}")]
    Lsp(String),

    #[error("symbol index error: {0}")]
    Symbol(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<regex::Error> for WorkspaceError {
    fn from(e: regex::Error) -> Self {
        WorkspaceError::Regex(e.to_string())
    }
}

impl From<crate::path_guard::GuardError> for WorkspaceError {
    fn from(e: crate::path_guard::GuardError) -> Self {
        match e {
            crate::path_guard::GuardError::Outside => WorkspaceError::PathOutsideRoot,
            crate::path_guard::GuardError::Invalid => WorkspaceError::InvalidPath,
        }
    }
}
