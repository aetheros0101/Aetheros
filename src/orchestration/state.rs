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
pub enum OrchestrationState {
    Pending,
        Scheduling,
        Dispatching,
        Running,
        Waiting,
        Retrying,
        Recovering,
        Checkpointing,
        Replaying,
        Suspended,
        Paused,
        TimedOut,
        Failed,
        Cancelled,
        Completed,
}
