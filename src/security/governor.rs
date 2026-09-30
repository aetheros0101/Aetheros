// ============================================================
// src/security/governor.rs
//
// V10 Sprint 4: Security Governor
//
// Capability Engine "bu izin var mı?" sorusuna cevap verir.
// Risk Engine "ne kadar riskli?" sorusuna cevap verir.
// Security Governor bu ikisini TEK bir karara birleştirir:
// Allow / Deny / RequiresApproval.
//
// RequiresApproval, Approval Engine (henüz yok) geldiğinde onun
// devreye gireceği yer — şimdilik çağıran taraf (AgentRuntime) bunu
// Deny gibi işler: onaylayacak kimse olmadan "belki" bir eylemi
// çalıştırmak güvenli değil.
//
// NOT: Bu dosyadaki önceki SecurityGovernor/SecurityPolicy tanımı
// (security::capabilities::CapabilitySet ile birlikte) hiçbir yerde
// kullanılmıyordu — tamamen başıboştu. Bu yeni tanım, gerçek
// CapabilityEngine/RiskEngine zincirine bağlı olarak onun yerini
// alıyor. HighRiskPolicy de buraya taşındı — daha önce AgentRuntime
// üzerinde "Security Governor gelene kadar" notuyla duruyordu.
// ============================================================

use std::sync::Arc;

use tracing::{debug, warn};
use uuid::Uuid;

use crate::security::capability_engine::{CapabilityDecision, CapabilityEngine};
use crate::security::risk_engine::RiskEngine;
use crate::types::agent_tool::{AgentTool, RiskLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernorDecision {
    Allow,
    Deny { reason: String },
    /// Approval Engine henüz yok. Çağıran taraf bunu şimdilik Deny
    /// gibi işlemeli.
    RequiresApproval { reason: String },
}

impl GovernorDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, GovernorDecision::Allow)
    }
}

/// High risk tespit edildiğinde Governor'ın ne yapacağı.
/// Varsayılan `Block` (güvenli). Approval Engine gelince `RequireApproval`
/// gerçek anlamına kavuşacak — bugün pratikte Block ile aynı sonucu verir.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HighRiskPolicy {
    Block,
    AllowWithWarning,
    RequireApproval,
}

pub struct SecurityGovernor {
    capability_engine: Option<Arc<CapabilityEngine>>,
    risk_engine: Option<Arc<RiskEngine>>,
    high_risk_policy: HighRiskPolicy,
}

impl SecurityGovernor {
    pub fn new(
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
    ) -> Self {
        Self {
            capability_engine,
            risk_engine,
            high_risk_policy: HighRiskPolicy::Block,
        }
    }

    pub fn set_high_risk_policy(&mut self, policy: HighRiskPolicy) {
        self.high_risk_policy = policy;
    }

    /// Capability + Risk kararlarını birleştirip TEK bir verdict döner.
    /// Hem kararı verir hem de gözlemlenebilirliği (log) tek yerde tutar.
    pub fn evaluate(&self, agent_id: Uuid, tool: &Arc<dyn AgentTool>) -> GovernorDecision {
        if let Some(engine) = &self.capability_engine {
            if let CapabilityDecision::Denied { reason } =
                engine.check(&agent_id, tool.required_capability())
            {
                warn!(
                    agent_id = %agent_id,
                    tool = tool.name(),
                    reason = %reason,
                    "Governor: capability denied"
                );
                return GovernorDecision::Deny { reason };
            }
        }

        if let Some(risk_engine) = &self.risk_engine {
            let assessment = risk_engine.assess(tool);
            if assessment.level == RiskLevel::High {
                return match self.high_risk_policy {
                    HighRiskPolicy::Block => {
                        warn!(
                            agent_id = %agent_id,
                            tool = tool.name(),
                            reason = %assessment.reason,
                            "Governor: high-risk tool call blocked"
                        );
                        GovernorDecision::Deny { reason: assessment.reason }
                    }
                    HighRiskPolicy::RequireApproval => {
                        warn!(
                            agent_id = %agent_id,
                            tool = tool.name(),
                            reason = %assessment.reason,
                            "Governor: high-risk tool call requires approval (Approval Engine henüz yok — Deny gibi işlenecek)"
                        );
                        GovernorDecision::RequiresApproval { reason: assessment.reason }
                    }
                    HighRiskPolicy::AllowWithWarning => {
                        warn!(
                            agent_id = %agent_id,
                            tool = tool.name(),
                            reason = %assessment.reason,
                            "Governor: high-risk tool call allowed under AllowWithWarning"
                        );
                        GovernorDecision::Allow
                    }
                };
            }

            debug!(
                agent_id = %agent_id,
                tool = tool.name(),
                level = ?assessment.level,
                "Governor: risk assessment proceeding"
            );
        }

        GovernorDecision::Allow
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;
    use crate::agents::capabilities::{AgentCapabilities, AgentCapability};

    struct FakeTool {
        cap: Option<AgentCapability>,
        risk: RiskLevel,
    }

    #[async_trait]
    impl AgentTool for FakeTool {
        fn name(&self) -> &'static str {
            "fake_tool"
        }
        fn required_capability(&self) -> Option<AgentCapability> {
            self.cap
        }
        fn risk_level(&self) -> RiskLevel {
            self.risk
        }
        async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
            Ok(String::new())
        }
    }

    fn tool(cap: Option<AgentCapability>, risk: RiskLevel) -> Arc<dyn AgentTool> {
        Arc::new(FakeTool { cap, risk })
    }

    #[test]
    fn no_engines_always_allows() {
        let governor = SecurityGovernor::new(None, None);
        let decision = governor.evaluate(Uuid::new_v4(), &tool(None, RiskLevel::High));
        assert_eq!(decision, GovernorDecision::Allow);
    }

    #[test]
    fn capability_denial_wins_before_risk_is_even_checked() {
        let cap_engine = Arc::new(CapabilityEngine::new()); // hiç grant yok
        let risk_engine = Arc::new(RiskEngine::new());
        let governor = SecurityGovernor::new(Some(cap_engine), Some(risk_engine));

        let decision = governor.evaluate(
            Uuid::new_v4(),
            &tool(Some(AgentCapability::WasmExecution), RiskLevel::Low),
        );

        assert!(!decision.is_allowed());
        assert!(matches!(decision, GovernorDecision::Deny { .. }));
    }

    #[test]
    fn default_policy_denies_high_risk_even_with_capability_granted() {
        let cap_engine = Arc::new(CapabilityEngine::new());
        let agent_id = Uuid::new_v4();
        cap_engine.grant(
            agent_id,
            AgentCapabilities {
                workflow_execution: false,
                wasm_execution: true,
                ai_reasoning: false,
                remote_execution: false,
            },
        );
        let risk_engine = Arc::new(RiskEngine::new());
        let governor = SecurityGovernor::new(Some(cap_engine), Some(risk_engine));

        let decision = governor.evaluate(
            agent_id,
            &tool(Some(AgentCapability::WasmExecution), RiskLevel::High),
        );

        assert!(matches!(decision, GovernorDecision::Deny { .. }));
    }

    #[test]
    fn allow_with_warning_permits_high_risk() {
        let risk_engine = Arc::new(RiskEngine::new());
        let mut governor = SecurityGovernor::new(None, Some(risk_engine));
        governor.set_high_risk_policy(HighRiskPolicy::AllowWithWarning);

        let decision = governor.evaluate(Uuid::new_v4(), &tool(None, RiskLevel::High));
        assert_eq!(decision, GovernorDecision::Allow);
    }

    #[test]
    fn require_approval_policy_returns_requires_approval_not_allow() {
        let risk_engine = Arc::new(RiskEngine::new());
        let mut governor = SecurityGovernor::new(None, Some(risk_engine));
        governor.set_high_risk_policy(HighRiskPolicy::RequireApproval);

        let decision = governor.evaluate(Uuid::new_v4(), &tool(None, RiskLevel::High));
        assert!(!decision.is_allowed());
        assert!(matches!(decision, GovernorDecision::RequiresApproval { .. }));
    }

    #[test]
    fn low_and_medium_risk_always_allowed_regardless_of_policy() {
        let risk_engine = Arc::new(RiskEngine::new());
        let governor = SecurityGovernor::new(None, Some(risk_engine));

        for level in [RiskLevel::Low, RiskLevel::Medium] {
            let decision = governor.evaluate(Uuid::new_v4(), &tool(None, level));
            assert_eq!(decision, GovernorDecision::Allow);
        }
    }
}
