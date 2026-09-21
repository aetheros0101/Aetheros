// AgentPlan / AgentPlanStep buradan src/types/agent_plan.rs'e taşındı
// (agents <-> ai döngüsel bağımlılığını kırmak için — bkz. .ai/architecture.json cycles).
// Mevcut `crate::agents::plans::AgentPlan` importları kırılmasın diye re-export ediliyor.
pub use crate::types::agent_plan::{AgentPlan, AgentPlanStep, ToolCall};
