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
pub enum PluginState {
    Loading,
    Active,
    Suspended,
    Failed,
    Unloaded,
}
