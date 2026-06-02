use serde::{
    Deserialize,
    Serialize,
};

use semver::Version;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct PluginVersion {
    pub current: Version,
    pub minimum_runtime: Version,
    pub major: u32,
    pub minor: u32,
    pub patch: u32,
}
