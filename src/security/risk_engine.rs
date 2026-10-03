// ============================================================
// src/security/risk_engine.rs
//
// V10 Sprint 3: Risk Engine
//
// Tasarım kararı (Afsi ile netleştirildi):
//   1. Risk skoru şimdilik SADECE tool-bazlı — argüman analizi yok.
//   2. RiskEngine ASLA bloklamaz. Sadece değerlendirir ve bir
//      RiskAssessment döner. Blocking kararı kasıtlı olarak burada
//      DEĞİL — bkz. AgentRuntime'daki HighRiskPolicy.
//
// Bu ayrım bilinçli: Risk Engine "ne kadar riskli" sorusuna cevap
// verir; "bu risk seviyesinde ne yapılır" kararı SecurityGovernor'dadır
// (bkz. security::governor, HighRiskPolicy). Argüman-bazlı karar ise
// AgentTool::assess_call / security::command_policy üzerinden gelir.
// ============================================================

use std::sync::Arc;

use crate::types::agent_tool::{AgentTool, RiskLevel};

#[derive(Debug, Clone)]
pub struct RiskAssessment {
    pub level: RiskLevel,
    pub reason: String,
}

pub struct RiskEngine;

impl RiskEngine {
    pub fn new() -> Self {
        Self
    }

    /// Şimdilik tool'un kendi bildirdiği sabit risk seviyesi.
    /// `arguments` henüz analiz edilmiyor — argüman-bazlı puanlama
    /// (örn. "yıkıcı" argüman kalıpları) sonraki bir sprint.
    pub fn assess(&self, tool: &Arc<dyn AgentTool>) -> RiskAssessment {
        let level = tool.risk_level();
        RiskAssessment {
            reason: format!(
                "tool '{}' sabit risk seviyesi bildiriyor: {level:?} \
                 (argüman analizi tool'un assess_call'ında)",
                tool.name()
            ),
            level,
        }
    }
}

impl Default for RiskEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use async_trait::async_trait;

    use super::*;

    struct HighRiskTool;
    #[async_trait]
    impl AgentTool for HighRiskTool {
        fn name(&self) -> &'static str {
            "high_risk_tool"
        }
        fn risk_level(&self) -> RiskLevel {
            RiskLevel::High
        }
        async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
            Ok(String::new())
        }
    }

    struct DefaultTool;
    #[async_trait]
    impl AgentTool for DefaultTool {
        fn name(&self) -> &'static str {
            "default_tool"
        }
        async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
            Ok(String::new())
        }
    }

    #[test]
    fn assess_reports_tools_declared_level() {
        let engine = RiskEngine::new();
        let tool: Arc<dyn AgentTool> = Arc::new(HighRiskTool);
        assert_eq!(engine.assess(&tool).level, RiskLevel::High);
    }

    #[test]
    fn assess_defaults_to_low_when_tool_does_not_override() {
        let engine = RiskEngine::new();
        let tool: Arc<dyn AgentTool> = Arc::new(DefaultTool);
        assert_eq!(engine.assess(&tool).level, RiskLevel::Low);
    }

    #[test]
    fn risk_levels_are_ordered() {
        assert!(RiskLevel::High > RiskLevel::Medium);
        assert!(RiskLevel::Medium > RiskLevel::Low);
    }
}
