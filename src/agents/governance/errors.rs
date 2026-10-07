//! Kurumsal hata modeli: sınıflandırılmış, retryable, korelasyonlu.

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Hatanın operasyonel sınıfı (alert / retry / user message için).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentErrorKind {
    Planning,
    ToolInvocation,
    Capability,
    Budget,
    Cancellation,
    Approval,
    Validation,
    Timeout,
    Persistence,
    Internal,
}

#[derive(Debug, Error, Clone, Serialize, Deserialize)]
#[error("{kind:?}: {message}")]
pub struct AgentError {
    pub kind: AgentErrorKind,
    pub message: String,
    /// Geçici hata ise otomatik retry adayı.
    pub retryable: bool,
    /// execution / step / tool korelasyonu.
    pub correlation_id: Option<String>,
    /// Kullanıcıya gösterilebilir kısa metin (i18n anahtarı da olabilir).
    pub user_message: Option<String>,
}

impl AgentError {
    pub fn new(kind: AgentErrorKind, message: impl Into<String>) -> Self {
        let message = message.into();
        let retryable = matches!(
            kind,
            AgentErrorKind::Timeout | AgentErrorKind::Persistence
        );
        Self {
            kind,
            message,
            retryable,
            correlation_id: None,
            user_message: None,
        }
    }

    pub fn planning(msg: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::Planning, msg)
    }
    pub fn tool(msg: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::ToolInvocation, msg)
    }
    pub fn capability(msg: impl Into<String>) -> Self {
        let mut e = Self::new(AgentErrorKind::Capability, msg);
        e.retryable = false;
        e
    }
    pub fn budget(msg: impl Into<String>) -> Self {
        let mut e = Self::new(AgentErrorKind::Budget, msg);
        e.retryable = false;
        e
    }
    pub fn cancelled() -> Self {
        let mut e = Self::new(AgentErrorKind::Cancellation, "agent cancelled");
        e.retryable = false;
        e.user_message = Some("İşlem iptal edildi".into());
        e
    }
    pub fn validation(msg: impl Into<String>) -> Self {
        let mut e = Self::new(AgentErrorKind::Validation, msg);
        e.retryable = false;
        e
    }
    pub fn approval(msg: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::Approval, msg)
    }
    pub fn timeout(msg: impl Into<String>) -> Self {
        let mut e = Self::new(AgentErrorKind::Timeout, msg);
        e.retryable = true;
        e
    }
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::new(AgentErrorKind::Internal, msg)
    }

    pub fn with_correlation(mut self, id: impl Into<String>) -> Self {
        self.correlation_id = Some(id.into());
        self
    }
    pub fn with_user_message(mut self, msg: impl Into<String>) -> Self {
        self.user_message = Some(msg.into());
        self
    }
    pub fn retryable(mut self, yes: bool) -> Self {
        self.retryable = yes;
        self
    }
}

/// Geriye dönük uyumluluk: eski unit-variant isimleri.
impl AgentError {
    pub fn planning_failed() -> Self {
        Self::planning("planning failed")
    }
    pub fn tool_invocation_failed() -> Self {
        Self::tool("tool invocation failed")
    }
    pub fn capability_denied() -> Self {
        Self::capability("capability denied")
    }
    pub fn budget_exceeded() -> Self {
        Self::budget("execution budget exceeded")
    }
}
