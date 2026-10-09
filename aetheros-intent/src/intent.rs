//! Kullanıcı niyeti (UserIntent).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IntentKind {
    Feature,
    Bugfix,
    Refactor,
    Docs,
    Research,
    Architecture,
    /// Tek dosya / küçük metin düzenlemesi.
    Chore,
    General,
}

impl Default for IntentKind {
    fn default() -> Self {
        IntentKind::General
    }
}

/// Çıkarılmış kullanıcı niyeti.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserIntent {
    pub id: Uuid,
    pub raw_text: String,
    pub kind: IntentKind,
    /// 0.0–1.0
    pub confidence: f32,
    pub language: Option<String>,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Geriye uyum takma adı.
pub type Intent = UserIntent;

impl UserIntent {
    pub fn new(raw_text: impl Into<String>, kind: IntentKind, confidence: f32) -> Self {
        Self {
            id: Uuid::new_v4(),
            raw_text: raw_text.into(),
            kind,
            confidence: confidence.clamp(0.0, 1.0),
            language: None,
            created_at: Utc::now(),
            tags: Vec::new(),
        }
    }
}
