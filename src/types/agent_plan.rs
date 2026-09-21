//src/types/agent_plan.rs
use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

/// `agents` ve `ai` modüllerinin her ikisi de bu tipe ihtiyaç duyduğu için
/// döngüsel bağımlılığı önlemek amacıyla buraya (types) taşındı.
/// Önceki konum: src/agents/plans.rs
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AgentPlan {
    pub id: Uuid,

    pub objective:
        String,

    pub planned_steps:
        Vec<AgentPlanStep>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AgentPlanStep {
    pub id: Uuid,

    pub name: String,

    pub retryable: bool,

    /// V10 Sprint 2 (Action/Tool Protocol): bu adımın çağıracağı tool,
    /// varsa. `None` → bu adım salt planlama/muhasebe adımı, hiçbir
    /// tool invoke edilmez (örn. fallback_plan'ın ürettiği adımlar).
    /// `Some` → AI, kendisine sunulan GERÇEK tool listesinden bilerek
    /// seçim yaptı; adım ismiyle tool ismi arasındaki tesadüfi eşleşmeye
    /// artık bağımlı değiliz (bkz. AgentPlanner::ai_plan).
    #[serde(default)]
    pub tool_call: Option<ToolCall>,
}

/// AI planner'ın bir adımda hangi tool'u, hangi argümanlarla çağırmak
/// istediğini taşıyan yapısal çağrı. Serbest metin adım isimlerinin
/// yerini alır — Risk/Approval/Audit katmanlarının denetleyebileceği
/// tek, tipli bir yüzey.
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct ToolCall {
    pub tool_name: String,

    #[serde(default)]
    pub arguments: Vec<String>,
}
