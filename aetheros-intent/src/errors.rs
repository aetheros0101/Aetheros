//! Intent pipeline hata modeli.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, IntentError>;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum IntentError {
    #[error("empty or blank user input")]
    EmptyInput,

    #[error("could not extract intent: {0}")]
    Extraction(String),

    #[error("ambiguous intent: {0}")]
    Ambiguous(String),

    #[error("requirement extraction failed: {0}")]
    RequirementExtraction(String),

    #[error("invalid requirement: {0}")]
    InvalidRequirement(String),

    #[error("constraint violation: {0}")]
    Constraint(String),

    #[error("compilation failed: {0}")]
    Compile(String),

    #[error("task graph error: {0}")]
    TaskGraph(String),

    #[error("dependency cycle detected involving: {0}")]
    DependencyCycle(String),

    #[error("validation failed: {0}")]
    Validation(String),

    #[error("unknown task node: {0}")]
    UnknownNode(String),
}
