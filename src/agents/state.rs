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
pub enum AgentState {
    Registered,
    Initializing,
    Ready,
    Planning,
    Executing,
    Waiting,
    Retrying,
    Suspended,
    Failed,
    Cancelled,
    Completed,
}
