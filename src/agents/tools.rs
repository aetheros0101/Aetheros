// AgentTool buradan src/types/agent_tool.rs'e taşındı
// (agents <-> ai döngüsel bağımlılığını kırmak için — bkz. .ai/architecture.json cycles).
// Mevcut `crate::agents::tools::AgentTool` importları kırılmasın diye re-export ediliyor.

pub use crate::types::agent_tool::AgentTool;
