//! # AetherOS Agents — kurumsal agent çalışma zamanı
//!
//! ## Klasör yapısı
//! ```text
//! agents/
//!   runtime/         execute · resume · loops · step · control
//!   planning/        planner · plans
//!   governance/      capabilities · budget · cancel · FSM · approval · errors
//!   tools/           workspace · terminal
//!   memory/          store · context · reasoning · persistence
//!   multi/           manager · registry · quota · isolation
//!   observability/   events · OTel export · metrics
//! ```
//!
//! Düz import yolları (`crate::agents::budget`, `crate::agents::runtime`, …)
//! geriye dönük uyumluluk için re-export edilir.

// ── Bounded contexts ─────────────────────────────────────────
pub mod governance;
pub mod memory;
pub mod multi;
pub mod observability;
pub mod planning;
pub mod runtime;
pub mod tools;

// ── Sık kullanılan tipler ────────────────────────────────────
pub use governance::{
    AgentCapabilities, AgentCapability, AgentError, AgentErrorKind, AgentExecutionBudget,
    AgentLifecycle, AgentState, ApprovalStore, BudgetAccounting, CancellationMode,
    CancellationToken, LifecycleTransition, PendingApproval, PolicyProfile,
};
pub use memory::{
    AgentContext, AgentMemory, AgentMemoryRecord, InMemoryVectorPort, MemoryLayer, ReasoningLog,
    ReasoningPhase, ReasoningTrace, VectorMemoryPort,
};
pub use multi::{
    AgentCapacity, AgentDescriptor, AgentManager, ExecutionLease, IsolationScope, ManagerStats,
    QuotaPolicy, QuotaUsage, SharedAgentManager,
};
pub use observability::{
    AgentEvent, AgentEventKind, AgentEventSink, AgentMetrics, FanoutEventSink, InMemoryEventSink,
    JsonlAuditSink, MetricsEventSink, OtelSpanExport,
};
pub use planning::{AgentPlan, AgentPlanStep, AgentPlanner, ToolCall};
pub use runtime::{AgentOutcome, AgentRuntime};
pub use tools::{
    AgentTool, CallVerdict, RiskLevel, TerminalAgentTool, WorkspaceAgentTool, WorkspaceToolKind,
};

// ── Host uyumlu alt modül yolları ─────────────────────────────
// crate::agents::budget::…, crate::agents::workspace_tool::… vb.
pub use governance::approval;
pub use governance::budget;
pub use governance::cancellation;
pub use governance::capabilities;
pub use governance::errors;
pub use governance::lifecycle;
pub use governance::state;

pub use planning::planner;
pub use planning::plans;

pub use tools::terminal_tool;
pub use tools::tool_invocation;
pub use tools::user_terminal;
pub use tools::workspace_tool;
// tools::tools re-export: AgentTool trait path crate::agents::tools::AgentTool
// (domain `tools` modülü zaten AgentTool'u pub use ediyor)

pub use memory::context;
pub use memory::execution;
pub use memory::persistence;
pub use memory::reasoning;

pub use multi::capacity;
pub use multi::manager;
pub use multi::registry;
pub use multi::subscriptions;

pub use runtime::executor;
