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
// RequiresApproval: AgentRuntime eylemi invoke etmeden DURAKLATIR ve
// ApprovalStore'a yazar; kullanıcı onaylarsa resume() ile devam eder
// (bkz. agents::approval, bridge::api::respond_to_approval). Onay
// yolu olmayan çağırıcılar (ör. REST) bunu fiilen Deny gibi yaşar.
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
use crate::types::agent_tool::{AgentTool, CallVerdict, RiskLevel};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GovernorDecision {
    Allow,
    Deny { reason: String },
    /// İnsan onayı gerekir: çağıran taraf eylemi çalıştırmadan duraklatır
    /// (AgentRuntime) — onay yolu yoksa çalıştırmamalıdır.
    RequiresApproval { reason: String },
}

impl GovernorDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, GovernorDecision::Allow)
    }
}

/// High risk tespit edildiğinde Governor'ın ne yapacağı.
/// Varsayılan `RequireApproval`: High riskli çağrı onay bekler (duraklar).
/// `Block` onay akışı olmadan kesin reddeder.
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
    /// B10: true ise eksik motor "izin ver" DEĞİL "reddet/onay iste"
    /// demektir (capability gerektiren tool + capability motoru yok →
    /// Deny; High riskli tool + risk motoru yok → RequiresApproval).
    /// `new()` varsayılanı false (geriye uyumlu, testler); üretim
    /// giriş noktaları (AgentRuntime::execute/resume) true yapar.
    fail_closed: bool,
}

impl SecurityGovernor {
    pub fn new(
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
    ) -> Self {
        Self {
            capability_engine,
            risk_engine,
            // V10 Faz 1: varsayılan artık Block değil RequireApproval.
            // Block, Approval Engine (Sprint 5) henüz yokken doğru
            // güvenli-varsayılandı — ama artık gerçek, test edilmiş bir
            // onay akışı var. Block'ta bırakmak High-risk her şeyi
            // (terminal dahil) kalıcı olarak kilitler ve Approval
            // Engine'i hiç devreye sokmaz. Hâlâ güvenli: hiçbir şey
            // kullanıcı açıkça onaylamadan ÇALIŞMAZ.
            high_risk_policy: HighRiskPolicy::RequireApproval,
            fail_closed: false,
        }
    }

    /// B10: eksik motorlarda fail-closed davranışını aç/kapat.
    pub fn set_fail_closed(&mut self, on: bool) {
        self.fail_closed = on;
    }

    pub fn set_high_risk_policy(&mut self, policy: HighRiskPolicy) {
        self.high_risk_policy = policy;
    }

    /// Capability + Risk kararlarını birleştirip TEK bir verdict döner.
    /// Hem kararı verir hem de gözlemlenebilirliği (log) tek yerde tutar.
    pub fn evaluate(&self, agent_id: Uuid, tool: &Arc<dyn AgentTool>) -> GovernorDecision {
        self.evaluate_inner(agent_id, tool, None)
    }

    /// B4: `evaluate` + çağrının ARGÜMANLARI. Sıra: capability → tool'un
    /// argüman-bazlı kararı (`AgentTool::assess_call`) → tool-bazlı risk.
    ///   - Deny  → kesin ret (onayla bile çalışmaz; HighRiskPolicy'den bağımsız)
    ///   - Allow → onay gerekmez (sabit High risk bu çağrı için aşılır)
    ///   - Ask   → RequiresApproval (risk motoru yokken bile)
    ///   - tool argümana bakmıyorsa (None) eski davranış aynen sürer.
    pub fn evaluate_call(
        &self,
        agent_id: Uuid,
        tool: &Arc<dyn AgentTool>,
        arguments: &[String],
    ) -> GovernorDecision {
        self.evaluate_inner(agent_id, tool, Some(arguments))
    }

    fn evaluate_inner(
        &self,
        agent_id: Uuid,
        tool: &Arc<dyn AgentTool>,
        arguments: Option<&[String]>,
    ) -> GovernorDecision {
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

        if self.fail_closed
            && self.capability_engine.is_none()
            && tool.required_capability().is_some()
        {
            warn!(
                agent_id = %agent_id,
                tool = tool.name(),
                "Governor: capability motoru bağlı değil — capability gerektiren tool reddedildi (fail-closed)"
            );
            return GovernorDecision::Deny {
                reason: "capability motoru bağlı değil (fail-closed)".to_string(),
            };
        }

        if let Some(args) = arguments {
            match tool.assess_call(args) {
                Some(CallVerdict::Deny { reason }) => {
                    warn!(
                        agent_id = %agent_id,
                        tool = tool.name(),
                        reason = %reason,
                        "Governor: çağrı argüman politikasınca reddedildi"
                    );
                    return GovernorDecision::Deny { reason };
                }
                Some(CallVerdict::Ask { reason }) => {
                    debug!(
                        agent_id = %agent_id,
                        tool = tool.name(),
                        reason = %reason,
                        "Governor: argüman politikası onay istiyor"
                    );
                    return GovernorDecision::RequiresApproval { reason };
                }
                Some(CallVerdict::Allow) => {
                    debug!(
                        agent_id = %agent_id,
                        tool = tool.name(),
                        "Governor: argüman politikası açıkça izin veriyor"
                    );
                    return GovernorDecision::Allow;
                }
                None => {}
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
                            "Governor: high-risk tool call requires approval (execution duraklatılacak)"
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

        if self.fail_closed && self.risk_engine.is_none() && tool.risk_level() == RiskLevel::High {
            warn!(
                agent_id = %agent_id,
                tool = tool.name(),
                "Governor: risk motoru bağlı değil — High riskli tool onaya bağlandı (fail-closed)"
            );
            return GovernorDecision::RequiresApproval {
                reason: "risk motoru bağlı değil; High riskli tool (fail-closed)".to_string(),
            };
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

    /// Argüman-bazlı karar veren sahte tool: ilk argümana göre.
    struct CallTool;

    #[async_trait]
    impl AgentTool for CallTool {
        fn name(&self) -> &'static str {
            "call_tool"
        }
        fn risk_level(&self) -> RiskLevel {
            RiskLevel::High
        }
        fn assess_call(&self, arguments: &[String]) -> Option<CallVerdict> {
            match arguments.first().map(String::as_str) {
                Some("ok") => Some(CallVerdict::Allow),
                Some("ask") => Some(CallVerdict::Ask { reason: "sor".into() }),
                Some("no") => Some(CallVerdict::Deny { reason: "yasak".into() }),
                _ => None,
            }
        }
        async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
            Ok(String::new())
        }
    }

    fn args(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn evaluate_call_allow_skips_high_risk_approval() {
        let g = SecurityGovernor::new(None, Some(Arc::new(RiskEngine::new())));
        let t: Arc<dyn AgentTool> = Arc::new(CallTool);
        // Argümansız eski yol: High → onay
        assert!(matches!(g.evaluate(Uuid::new_v4(), &t), GovernorDecision::RequiresApproval { .. }));
        // Argüman-bazlı açık izin → otomatik
        assert_eq!(g.evaluate_call(Uuid::new_v4(), &t, &args(&["ok"])), GovernorDecision::Allow);
    }

    #[test]
    fn evaluate_call_deny_is_final_even_under_allow_with_warning() {
        let mut g = SecurityGovernor::new(None, Some(Arc::new(RiskEngine::new())));
        g.set_high_risk_policy(HighRiskPolicy::AllowWithWarning);
        let t: Arc<dyn AgentTool> = Arc::new(CallTool);
        assert!(matches!(
            g.evaluate_call(Uuid::new_v4(), &t, &args(&["no"])),
            GovernorDecision::Deny { .. }
        ));
    }

    #[test]
    fn evaluate_call_ask_requires_approval_even_without_risk_engine() {
        let g = SecurityGovernor::new(None, None);
        let t: Arc<dyn AgentTool> = Arc::new(CallTool);
        assert!(matches!(
            g.evaluate_call(Uuid::new_v4(), &t, &args(&["ask"])),
            GovernorDecision::RequiresApproval { .. }
        ));
    }

    #[test]
    fn evaluate_call_falls_back_to_tool_risk_when_tool_has_no_opinion() {
        let g = SecurityGovernor::new(None, Some(Arc::new(RiskEngine::new())));
        let t: Arc<dyn AgentTool> = Arc::new(CallTool);
        assert!(matches!(
            g.evaluate_call(Uuid::new_v4(), &t, &args(&["bilinmeyen"])),
            GovernorDecision::RequiresApproval { .. }
        ));
    }

    #[test]
    fn evaluate_call_capability_denied_beats_policy_allow() {
        // Capability reddi, argüman politikasından ÖNCE gelir.
        struct CapTool;
        #[async_trait]
        impl AgentTool for CapTool {
            fn name(&self) -> &'static str { "cap_tool" }
            fn required_capability(&self) -> Option<AgentCapability> {
                Some(AgentCapability::TerminalExecution)
            }
            fn assess_call(&self, _a: &[String]) -> Option<CallVerdict> {
                Some(CallVerdict::Allow)
            }
            async fn invoke(&self, _a: Vec<String>) -> Result<String, String> { Ok(String::new()) }
        }
        let cap_engine = Arc::new(CapabilityEngine::new());
        let g = SecurityGovernor::new(Some(cap_engine), None);
        let t: Arc<dyn AgentTool> = Arc::new(CapTool);
        assert!(matches!(
            g.evaluate_call(Uuid::new_v4(), &t, &args(&["x"])),
            GovernorDecision::Deny { .. }
        ));
    }

    #[test]
    fn fail_closed_denies_capability_tool_without_capability_engine() {
        struct CapTool2;
        #[async_trait]
        impl AgentTool for CapTool2 {
            fn name(&self) -> &'static str { "cap2" }
            fn required_capability(&self) -> Option<AgentCapability> {
                Some(AgentCapability::TerminalExecution)
            }
            async fn invoke(&self, _a: Vec<String>) -> Result<String, String> { Ok(String::new()) }
        }
        let t: Arc<dyn AgentTool> = Arc::new(CapTool2);
        let mut g = SecurityGovernor::new(None, None);
        assert_eq!(g.evaluate(Uuid::new_v4(), &t), GovernorDecision::Allow); // eski davranış
        g.set_fail_closed(true);
        assert!(matches!(g.evaluate(Uuid::new_v4(), &t), GovernorDecision::Deny { .. }));
    }

    #[test]
    fn fail_closed_requires_approval_for_high_risk_without_risk_engine() {
        let mut g = SecurityGovernor::new(None, None);
        g.set_fail_closed(true);
        let high = tool(None, RiskLevel::High);
        let low = tool(None, RiskLevel::Low);
        assert!(matches!(
            g.evaluate(Uuid::new_v4(), &high),
            GovernorDecision::RequiresApproval { .. }
        ));
        // Düşük riskli, capability istemeyen tool etkilenmez.
        assert_eq!(g.evaluate(Uuid::new_v4(), &low), GovernorDecision::Allow);
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
    fn default_policy_requires_approval_for_high_risk_even_with_capability_granted() {
        let cap_engine = Arc::new(CapabilityEngine::new());
        let agent_id = Uuid::new_v4();
        cap_engine.grant(
            agent_id,
            AgentCapabilities {
                workflow_execution: false,
                wasm_execution: true,
                ai_reasoning: false,
                remote_execution: false,
                terminal_execution: false,
                workspace_read: false,
                workspace_write: false,
                workspace_mutate: false,
            },
        );
        let risk_engine = Arc::new(RiskEngine::new());
        let governor = SecurityGovernor::new(Some(cap_engine), Some(risk_engine));

        let decision = governor.evaluate(
            agent_id,
            &tool(Some(AgentCapability::WasmExecution), RiskLevel::High),
        );

        assert!(!decision.is_allowed());
        assert!(matches!(decision, GovernorDecision::RequiresApproval { .. }));
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
