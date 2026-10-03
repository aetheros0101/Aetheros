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
pub struct AgentCapabilities {
    pub workflow_execution:
        bool,

    pub wasm_execution:
        bool,

    pub ai_reasoning:
        bool,

    pub remote_execution:
        bool,

    /// V10 Faz 1: agent'ın gerçek bir shell komutu / süreç çalıştırıp
    /// çalıştıramayacağı (bkz. agents::tools::terminal_tool::TerminalAgentTool).
    pub terminal_execution:
        bool,
}

#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
)]
pub enum AgentCapability {
    WorkflowExecution,

    WasmExecution,

    AiReasoning,

    RemoteExecution,

    TerminalExecution,
}
