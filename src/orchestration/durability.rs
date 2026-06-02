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
pub enum DurabilityLevel {
    Ephemeral,
    Persistent,
    Replicated,
    QuorumReplicated,
}
