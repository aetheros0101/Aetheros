use serde::{
    Deserialize,
    Serialize,
};

use crate::security::capabilities::CapabilitySet;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct SecurityPolicy {
    pub capabilities:
        CapabilitySet,

    pub audit_required:
        bool,
}
