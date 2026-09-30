// ============================================================
// src/tests/risk_policy_tests.rs
//
// V10 Sprint 3: RiskEngine kendisi hiçbir şeyi bloklamıyor (bkz.
// security/risk_engine.rs'in kendi testleri — sadece assess() doğru
// seviyeyi mi döndürüyor diye bakar). Asıl blocking kararı burada,
// AgentRuntime'ın HighRiskPolicy'sinde test ediliyor:
//
//   - Varsayılan (Block): High risk bir tool, CAPABILITY GRANT EDİLMİŞ
//     OLSA BİLE reddedilir.
//   - AllowWithWarning: bilinçli gevşetmeyle High risk tool çalışır.
//   - Low/Medium risk tool'lar, politikadan etkilenmeden her zaman
//     normal işler.
//   - risk_engine = None: eski davranış — hiç değerlendirme yok.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use async_trait::async_trait;
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::runtime::{AgentRuntime, HighRiskPolicy};
use crate::security::capability_engine::CapabilityEngine;
use crate::security::risk_engine::RiskEngine;
use crate::types::agent_tool::{AgentTool, RiskLevel};

struct RiskyTool {
    level: RiskLevel,
    invoked: Arc<AtomicBool>,
}

#[async_trait]
impl AgentTool for RiskyTool {
    fn name(&self) -> &'static str {
        "risky_tool"
    }
    fn risk_level(&self) -> RiskLevel {
        self.level
    }
    async fn invoke(&self, _arguments: Vec<String>) -> Result<String, String> {
        self.invoked.store(true, Ordering::SeqCst);
        Ok("ran".to_string())
    }
}

fn test_budget() -> AgentExecutionBudget {
    AgentExecutionBudget {
        max_tokens: 10_000,
        max_steps: 10,
        max_runtime_seconds: 30,
    }
}

#[tokio::test]
async fn default_policy_blocks_high_risk_tool_even_with_capability_granted() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
        level: RiskLevel::High,
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();

    // Capability GRANT EDİLMİŞ — ama tool High risk, politika varsayılan Block.
    let cap_engine = Arc::new(CapabilityEngine::new());
    // risky_tool hiç required_capability() bildirmiyor (None), yani
    // capability kontrolü zaten geçer — asıl engel risk katmanı olacak.
    let risk_engine = Arc::new(RiskEngine::new());

    let runtime = AgentRuntime::new(
        test_budget(),
        vec![tool],
        None,
        Some(cap_engine),
        Some(risk_engine),
        None,
        None,
    );

    let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

    assert!(
        result.is_err(),
        "varsayılan Block politikasında High risk bir tool çalışmamalı"
    );
    assert!(!invoked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn allow_with_warning_policy_lets_high_risk_tool_run() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
        level: RiskLevel::High,
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();
    let risk_engine = Arc::new(RiskEngine::new());

    let mut runtime = AgentRuntime::new(test_budget(), vec![tool], None, None, Some(risk_engine), None, None);
    runtime.set_high_risk_policy(HighRiskPolicy::AllowWithWarning);

    let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

    assert!(
        result.is_ok(),
        "AllowWithWarning altında High risk tool çalışabilmeli: {:?}",
        result.err()
    );
    assert!(invoked.load(Ordering::SeqCst));
}

#[tokio::test]
async fn low_and_medium_risk_tools_are_unaffected_by_default_policy() {
    for level in [RiskLevel::Low, RiskLevel::Medium] {
        let invoked = Arc::new(AtomicBool::new(false));
        let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
            level,
            invoked: invoked.clone(),
        });
        let agent_id = Uuid::new_v4();
        let risk_engine = Arc::new(RiskEngine::new());

        let runtime =
            AgentRuntime::new(test_budget(), vec![tool], None, None, Some(risk_engine), None, None);
        let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

        assert!(
            result.is_ok(),
            "{level:?} risk seviyesi varsayılan politikadan etkilenmemeli: {:?}",
            result.err()
        );
        assert!(invoked.load(Ordering::SeqCst));
    }
}

#[tokio::test]
async fn missing_risk_engine_preserves_pre_sprint3_behavior() {
    let invoked = Arc::new(AtomicBool::new(false));
    let tool: Arc<dyn AgentTool> = Arc::new(RiskyTool {
        level: RiskLevel::High,
        invoked: invoked.clone(),
    });
    let agent_id = Uuid::new_v4();

    // risk_engine = None → değerlendirme hiç yapılmaz, High risk bile
    // geçer. Bu, RiskEngine bağlanmamış eski/basit kullanım senaryoları
    // için geriye dönük uyumluluk.
    let runtime = AgentRuntime::new(test_budget(), vec![tool], None, None, None, None, None);
    let result = runtime.invoke_best_tool(agent_id, "risky_tool").await;

    assert!(result.is_ok());
    assert!(invoked.load(Ordering::SeqCst));
}
