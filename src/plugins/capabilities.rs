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
pub enum PluginCapability {
    WorkflowNode,
    ToolProvider,
    TelemetryExporter,
    PersistenceAdapter,
    RemoteTransport,
}
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct PluginCapabilities {
    pub filesystem:
        bool,

    pub network:
        bool,

    pub ai_access:
        bool,

    pub workflow_access:
        bool,
}
