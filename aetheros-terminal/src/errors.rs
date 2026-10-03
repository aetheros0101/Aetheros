use std::path::PathBuf;

#[derive(Debug, thiserror::Error)]
pub enum TerminalError {
    #[error("failed to spawn process: {0}")]
    Spawn(#[source] std::io::Error),

    #[error("failed to read process output: {0}")]
    Io(#[source] std::io::Error),

    #[error("process timed out after {0:?}")]
    Timeout(std::time::Duration),

    #[error("process was cancelled")]
    Cancelled,

    #[error("working directory does not exist: {0}")]
    InvalidWorkingDirectory(PathBuf),

    #[error("output exceeded the configured limit of {0} bytes")]
    OutputLimitExceeded(usize),

    #[error("invalid command: {0}")]
    InvalidCommand(String),
}
