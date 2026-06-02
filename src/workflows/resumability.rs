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
pub enum ResumeStrategy {
    FromCheckpoint,
    Replay,
    Restart,
}
