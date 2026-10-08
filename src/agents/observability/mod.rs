//! Gözlemlenebilirlik — event sink, OTel-uyumlu export, audit JSONL.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKind {
    ExecutionStarted,
    ExecutionCompleted,
    ExecutionFailed,
    ExecutionCancelled,
    StateTransition,
    StepStarted,
    StepCompleted,
    StepFailed,
    ToolInvoked,
    ToolResult,
    ApprovalRequested,
    ApprovalResolved,
    BudgetWarning,
    BudgetExceeded,
    CancellationRequested,
    /// P3: lease / quota
    LeaseAcquired,
    LeaseReleased,
    QuotaDenied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentEvent {
    pub kind: AgentEventKind,
    pub agent_id: Uuid,
    pub execution_id: Uuid,
    pub timestamp: DateTime<Utc>,
    #[serde(default)]
    pub step_index: Option<u32>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub duration_ms: Option<u64>,
    #[serde(default)]
    pub success: Option<bool>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

impl AgentEvent {
    pub fn new(kind: AgentEventKind, agent_id: Uuid, execution_id: Uuid) -> Self {
        Self {
            kind,
            agent_id,
            execution_id,
            timestamp: Utc::now(),
            step_index: None,
            tool_name: None,
            duration_ms: None,
            success: None,
            message: None,
            attributes: Default::default(),
        }
    }

    pub fn with_attr(mut self, k: impl Into<String>, v: serde_json::Value) -> Self {
        self.attributes.insert(k.into(), v);
        self
    }
}

/// Runtime'ın olayları ilettiği port.
pub trait AgentEventSink: Send + Sync {
    fn emit(&self, event: AgentEvent);
}

/// Bellek içi sink (test / basit host).
#[derive(Default)]
pub struct InMemoryEventSink {
    pub events: Mutex<Vec<AgentEvent>>,
}

impl AgentEventSink for InMemoryEventSink {
    fn emit(&self, event: AgentEvent) {
        if let Ok(mut g) = self.events.lock() {
            g.push(event);
            if g.len() > 10_000 {
                let excess = g.len() - 10_000;
                g.drain(0..excess);
            }
        }
    }
}

impl InMemoryEventSink {
    pub fn snapshot(&self) -> Vec<AgentEvent> {
        self.events.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// Birden fazla sink'e fan-out.
pub struct FanoutEventSink {
    sinks: Vec<Arc<dyn AgentEventSink>>,
}

impl FanoutEventSink {
    pub fn new(sinks: Vec<Arc<dyn AgentEventSink>>) -> Self {
        Self { sinks }
    }
}

impl AgentEventSink for FanoutEventSink {
    fn emit(&self, event: AgentEvent) {
        for s in &self.sinks {
            s.emit(event.clone());
        }
    }
}

// ── OTel-uyumlu export şeması (crate bağımlılığı yok) ────────

/// OpenTelemetry-benzeri span kaydı — host OTLP'ye map eder.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtelSpanExport {
    pub name: String,
    pub trace_id: String,
    pub span_id: String,
    pub parent_span_id: Option<String>,
    pub start_time_unix_nano: u64,
    pub end_time_unix_nano: Option<u64>,
    pub status: String,
    pub attributes: serde_json::Map<String, serde_json::Value>,
}

impl AgentEvent {
    /// Event → OTel span export (1:1 basit eşleme).
    pub fn to_otel_span(&self) -> OtelSpanExport {
        let trace_id = format!("{}", self.execution_id.simple());
        let span_id = {
            let n = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            format!("{n:016x}")
        };
        let start = self.timestamp.timestamp_nanos_opt().unwrap_or(0) as u64;
        let mut attrs = self.attributes.clone();
        attrs.insert(
            "agent.id".into(),
            serde_json::Value::String(self.agent_id.to_string()),
        );
        attrs.insert(
            "execution.id".into(),
            serde_json::Value::String(self.execution_id.to_string()),
        );
        attrs.insert(
            "event.kind".into(),
            serde_json::Value::String(format!("{:?}", self.kind)),
        );
        if let Some(t) = &self.tool_name {
            attrs.insert("tool.name".into(), serde_json::Value::String(t.clone()));
        }
        if let Some(d) = self.duration_ms {
            attrs.insert("duration_ms".into(), serde_json::Value::from(d));
        }
        let status = match self.kind {
            AgentEventKind::ExecutionFailed
            | AgentEventKind::StepFailed
            | AgentEventKind::BudgetExceeded => "ERROR",
            AgentEventKind::ExecutionCancelled | AgentEventKind::CancellationRequested => {
                "CANCELLED"
            }
            _ => "OK",
        };
        let end = self.duration_ms.map(|ms| start + ms * 1_000_000);
        OtelSpanExport {
            name: format!("agent.{:?}", self.kind),
            trace_id,
            span_id,
            parent_span_id: None,
            start_time_unix_nano: start,
            end_time_unix_nano: end,
            status: status.into(),
            attributes: attrs,
        }
    }
}

/// JSON Lines audit exporter — dosya / pipe için.
pub struct JsonlAuditSink {
    lines: Mutex<Vec<String>>,
    max_lines: usize,
}

impl JsonlAuditSink {
    pub fn new(max_lines: usize) -> Self {
        Self {
            lines: Mutex::new(Vec::new()),
            max_lines,
        }
    }

    pub fn lines(&self) -> Vec<String> {
        self.lines.lock().map(|g| g.clone()).unwrap_or_default()
    }

    pub fn drain_jsonl(&self) -> String {
        let mut g = match self.lines.lock() {
            Ok(g) => g,
            Err(_) => return String::new(),
        };
        let out = g.join("\n");
        g.clear();
        out
    }
}

impl Default for JsonlAuditSink {
    fn default() -> Self {
        Self::new(50_000)
    }
}

impl AgentEventSink for JsonlAuditSink {
    fn emit(&self, event: AgentEvent) {
        let span = event.to_otel_span();
        if let Ok(line) = serde_json::to_string(&span)
            && let Ok(mut g) = self.lines.lock()
        {
            g.push(line);
            if g.len() > self.max_lines {
                let excess = g.len() - self.max_lines;
                g.drain(0..excess);
            }
        }
    }
}

/// Basit sayaç metrikleri.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AgentMetrics {
    pub executions_started: u64,
    pub executions_completed: u64,
    pub executions_failed: u64,
    pub executions_cancelled: u64,
    pub tools_invoked: u64,
    pub approvals_requested: u64,
    pub budget_exceeded: u64,
    pub quota_denied: u64,
}

impl AgentMetrics {
    pub fn observe(&mut self, kind: AgentEventKind) {
        match kind {
            AgentEventKind::ExecutionStarted => self.executions_started += 1,
            AgentEventKind::ExecutionCompleted => self.executions_completed += 1,
            AgentEventKind::ExecutionFailed => self.executions_failed += 1,
            AgentEventKind::ExecutionCancelled => self.executions_cancelled += 1,
            AgentEventKind::ToolInvoked => self.tools_invoked += 1,
            AgentEventKind::ApprovalRequested => self.approvals_requested += 1,
            AgentEventKind::BudgetExceeded => self.budget_exceeded += 1,
            AgentEventKind::QuotaDenied => self.quota_denied += 1,
            _ => {}
        }
    }
}

/// Metrics toplayan sink.
pub struct MetricsEventSink {
    pub metrics: Mutex<AgentMetrics>,
}

impl Default for MetricsEventSink {
    fn default() -> Self {
        Self {
            metrics: Mutex::new(AgentMetrics::default()),
        }
    }
}

impl AgentEventSink for MetricsEventSink {
    fn emit(&self, event: AgentEvent) {
        if let Ok(mut m) = self.metrics.lock() {
            m.observe(event.kind);
        }
    }
}

impl MetricsEventSink {
    pub fn snapshot(&self) -> AgentMetrics {
        self.metrics.lock().map(|m| m.clone()).unwrap_or_default()
    }
}
