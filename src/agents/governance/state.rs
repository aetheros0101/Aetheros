//! Kanonik agent yaşam döngüsü durumları.
//!
//! Tek kaynak: UI, runtime ve audit aynı enum'u kullanır.

use serde::{Deserialize, Serialize};

/// Agent örneğinin kalıcı/oturum durumu (registry / manager).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentState {
    /// Kayıtlı, henüz başlatılmadı.
    Registered,
    /// Kaynaklar ayrılıyor.
    Initializing,
    /// Plan üretiliyor.
    Planning,
    /// Adımlar çalışıyor.
    Executing,
    /// Onay veya harici girdi bekleniyor.
    Waiting,
    /// Geçici hata sonrası yeniden deneme.
    Retrying,
    /// Kullanıcı veya sistem askıya aldı.
    Suspended,
    /// İptal isteği işleniyor / tamamlandı.
    Cancelled,
    /// Kalıcı hata.
    Failed,
    /// Başarıyla bitti.
    Completed,
}

impl AgentState {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            AgentState::Completed | AgentState::Failed | AgentState::Cancelled
        )
    }

    pub fn is_active(self) -> bool {
        matches!(
            self,
            AgentState::Initializing
                | AgentState::Planning
                | AgentState::Executing
                | AgentState::Waiting
                | AgentState::Retrying
        )
    }

    pub fn label(self) -> &'static str {
        match self {
            AgentState::Registered => "registered",
            AgentState::Initializing => "initializing",
            AgentState::Planning => "planning",
            AgentState::Executing => "executing",
            AgentState::Waiting => "waiting",
            AgentState::Retrying => "retrying",
            AgentState::Suspended => "suspended",
            AgentState::Cancelled => "cancelled",
            AgentState::Failed => "failed",
            AgentState::Completed => "completed",
        }
    }
}

/// Geriye dönük alias — eski `AgentLifecycleState` tüketicileri için.
pub type AgentLifecycleState = AgentState;
