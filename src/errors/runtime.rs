use thiserror::Error;

use crate::errors::persistence::PersistenceError;
use crate::errors::task::TaskError;
use crate::errors::wasm::WasmError;

#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("worker subsystem failure")]
    WorkerSubsystemFailure,

    #[error("scheduler failure")]
    SchedulerFailure,

    #[error("task failure: {0}")]
    Task(
        #[from]
        TaskError,
    ),

    #[error("wasm failure: {0}")]
    Wasm(
        #[from]
        WasmError,
    ),

    #[error("persistence failure: {0}")]
    Persistence(
        #[from]
        PersistenceError,
    ),

    #[error(
        "task execution failed: {message}"
    )]
    TaskExecutionFailed {
        message: String,
    },

    #[error("runtime shutdown")]
    Shutdown,

    #[error("dispatcher failure")]
    DispatcherFailure,

    #[error("task queue closed")]
    QueueClosed,

    // Mevcut son variant'tan sonra ekle:
    #[error("orchestration failure")]
    OrchestrationFailure,

    #[error(
        "validation error: {0}"
    )]
    ValidationError(String),
}
