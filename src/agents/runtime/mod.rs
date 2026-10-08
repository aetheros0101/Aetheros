//! Agent runtime — plan → guard → invoke → memory döngüsü.
//!
//! Alt modüller:
//! - [`outcome`] — AgentOutcome / StepOutcome
//! - [`control`] — cancel, budget, lifecycle, events
//! - [`entry`] — execute / resume giriş noktaları
//! - [`loops`] — sabit plan ve otonom döngüler
//! - [`step`] — guarded step + tool invoke

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::agents::approval::ApprovalStore;
use crate::agents::budget::{AgentExecutionBudget, BudgetAccounting};
use crate::agents::cancellation::CancellationToken;
use crate::agents::lifecycle::AgentLifecycle;
use crate::agents::memory::AgentMemory;
use crate::agents::observability::AgentEventSink;
use crate::agents::reasoning::ReasoningLog;
use crate::agents::tools::AgentTool;
use crate::ai::routing::router::ProviderRouter;
use crate::logging::audit::AuditLog;
use crate::security::capability_engine::CapabilityEngine;
use crate::security::governor::SecurityGovernor;
use crate::security::risk_engine::RiskEngine;

pub mod control;
pub mod entry;
pub mod executor;
pub mod loops;
pub mod outcome;
pub mod step;

pub use outcome::AgentOutcome;
pub(crate) use outcome::StepOutcome;

pub use crate::security::governor::HighRiskPolicy;

pub struct AgentRuntime {
    memory: AgentMemory,
    tools: Vec<Arc<dyn AgentTool>>,
    budget: AgentExecutionBudget,
    ai_router: Option<Arc<ProviderRouter>>,
    /// Planlayıcıya yalnız agent'ın KULLANABİLECEĞİ araçları göstermek için
    /// (Governor'ın içindeki kopyaya erişilemez).
    capability_engine: Option<Arc<CapabilityEngine>>,
    /// V10 Sprint 4: Capability + Risk kararları artık tek bir
    /// SecurityGovernor üzerinden alınıyor — bkz. check_and_invoke.
    governor: SecurityGovernor,
    /// V10 Sprint 5: RequiresApproval durumunda duraklatma kaydının
    /// yazılacağı yer. None ise (eski/basit kullanım) execution yine
    /// duraklatılır ama kayıt hiçbir yerde saklanmaz — kullanıcı asla
    /// onaylayamaz, fiilen kalıcı ret gibi davranır.
    approval_store: Option<Arc<ApprovalStore>>,
    /// V10 Sprint 6: Governor kararlarının, tool çağrılarının ve
    /// pause/resume olaylarının yazıldığı denetim izi. None ise
    /// (eski/basit kullanım) hiçbir şey kaydedilmez — davranış
    /// değişmez, sadece iz kalmaz.
    audit_log: Option<Arc<AuditLog>>,
    /// Kurumsal kontrol düzlemi (opsiyonel — None ise davranış eski sürümle aynı).
    lifecycle: AgentLifecycle,
    cancellation: CancellationToken,
    event_sink: Option<Arc<dyn AgentEventSink>>,
    accounting: BudgetAccounting,
    reasoning_log: ReasoningLog,
}

impl AgentRuntime {
    pub fn new(
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
        approval_store: Option<Arc<ApprovalStore>>,
        audit_log: Option<Arc<AuditLog>>,
    ) -> Self {
        Self::new_with_control(
            budget,
            tools,
            ai_router,
            capability_engine,
            risk_engine,
            approval_store,
            audit_log,
            CancellationToken::new(),
            None,
        )
    }

    /// Kontrol düzlemi ile oluştur (cancel token + event sink).
    #[allow(clippy::too_many_arguments)] // TODO(Faz 2): parametre struct'ı
    pub fn new_with_control(
        budget: AgentExecutionBudget,
        tools: Vec<Arc<dyn AgentTool>>,
        ai_router: Option<Arc<ProviderRouter>>,
        capability_engine: Option<Arc<CapabilityEngine>>,
        risk_engine: Option<Arc<RiskEngine>>,
        approval_store: Option<Arc<ApprovalStore>>,
        audit_log: Option<Arc<AuditLog>>,
        cancellation: CancellationToken,
        event_sink: Option<Arc<dyn AgentEventSink>>,
    ) -> Self {
        let now = now_ms();
        Self {
            memory: AgentMemory::new(),
            tools,
            budget,
            ai_router,
            capability_engine: capability_engine.clone(),
            governor: SecurityGovernor::new(capability_engine, risk_engine),
            approval_store,
            audit_log,
            lifecycle: AgentLifecycle::new(),
            cancellation,
            event_sink,
            accounting: BudgetAccounting::start(now),
            reasoning_log: ReasoningLog::default(),
        }
    }
}

pub(super) fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
