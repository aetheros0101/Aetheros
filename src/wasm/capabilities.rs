use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySet {
    pub allow_network: bool,
    pub allow_fs: bool,
    pub allow_env: bool,
    pub allow_spawn: bool,
}
