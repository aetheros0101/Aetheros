use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
)]
pub enum FailureSeverity {
    Recoverable,
    Critical,
    Fatal,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct FailureClassification {
    pub severity:
        FailureSeverity,

    pub retryable:
        bool,
}
