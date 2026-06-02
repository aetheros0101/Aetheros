// ============================================================
// src/agents/memory.rs
//
// Faz 5 Eklemesi:
//
// store() ve get() metodları eklendi.
// AgentRuntime, her adım sonucunu memory'e yazar,
// sonraki adımlar önceki sonuçlara erişebilir.
//
// DashMap: lock-free concurrent okuma/yazma ✓
// ============================================================

use chrono::{
    DateTime,
    Utc,
};
use dashmap::DashMap;
use serde::{
    Deserialize,
    Serialize,
};
use serde_json::Value;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct AgentMemoryRecord {
    pub key: String,
    pub value: String,
    pub created_at: DateTime<Utc>,
}

pub struct AgentMemory {
    entries: DashMap<String, Value>,
}

impl AgentMemory {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
        }
    }

    /// Adım sonucunu key-value olarak kaydet.
    pub fn store(
        &self,
        key: impl Into<String>,
        value: Value,
    ) {
        self.entries.insert(key.into(), value);
    }

    /// Daha önce kaydedilmiş değeri getir.
    pub fn get(
        &self,
        key: &str,
    ) -> Option<Value> {
        self.entries.get(key).map(|v| v.clone())
    }

    /// Tüm kayıtları döndür (reasoning context için).
    pub fn snapshot(&self) -> Vec<(String, Value)> {
        self.entries
            .iter()
            .map(|entry| {
                (entry.key().clone(), entry.value().clone())
            })
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}
