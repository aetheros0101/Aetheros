use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct CapabilitySet {
    pub filesystem:
        bool,

    pub networking:
        bool,

    pub process_execution:
        bool,

    pub ai_inference:
        bool,
}
