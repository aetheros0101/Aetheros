use thiserror::Error;

#[derive(Debug, Error)]
pub enum TaskError {
    #[error("task cancelled")]
    Cancelled,

    #[error("task deadline exceeded")]
    DeadlineExceeded,

    #[error("task retry exhausted")]
    RetryExhausted,

    #[error("invalid task state")]
    InvalidState,

    #[error("task persistence failure")]
    PersistenceFailure,
}
