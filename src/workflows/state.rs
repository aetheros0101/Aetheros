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
pub enum WorkflowState {
    Pending,
        Scheduled,
        Running,
        Suspended,
        Retrying,
        Failed,
        Completed,
        Cancelled,
}
