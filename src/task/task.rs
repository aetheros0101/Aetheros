// ============================================================
// src/task/task.rs
//
// Optimizasyon #1: wasm_module → wasm_module_hash
//
// ÖNCE:
//   pub wasm_module: Vec<u8>
//   - Her TaskDefinition bir WASM binary kopyası taşıyordu
//   - Clone = MB kopyası, persist = base64 JSON
//
// SONRA:
//   pub wasm_module_hash: [u8; 32]
//   - 32 byte SHA-256 hash referansı
//   - Gerçek binary ModuleStore'da (hash → Arc<Vec<u8>>)
//   - Clone = 32 byte kopyası (~1 CPU cache line)
//   - Persist = 64 char hex string (sabit küçük boyut)
//
// BACKWARD COMPATIBILITY:
//   wasm_module_hash = [0u8; 32] → "empty/no module"
//   API'de upload → ModuleStore → hash → TaskDefinition
// ============================================================

use std::collections::HashMap;

use serde::{
    Deserialize,
    Serialize,
};

use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::orchestration::TaskOrchestration;
use crate::types::ids::TaskId;
use crate::types::timestamps::Timestamp;
use crate::wasm::module_store::ModuleHash;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct TaskMetadata {
    pub labels: HashMap<String, String>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub enum TaskState {
    Created,
    Persisted,
    Queued,
    Scheduled,
    Executing,
    Completed,
    Failed,
    Cancelled,
    Retrying,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct TaskDefinition {
    pub id: TaskId,

    pub parent: Option<TaskId>,

    pub orchestration: Option<TaskOrchestration>,

    pub priority: TaskPriority,

    pub deadline: Option<Timestamp>,

    pub timeout_ms: u64,

    pub retry_policy: RetryPolicy,

    pub metadata: TaskMetadata,

    /// Optimizasyon #1: wasm_module yerine hash.
    ///
    /// 32 byte → sabit boyut, clone bedava.
    /// ModuleStore::get(hash) → Arc<Vec<u8>> binary.
    ///
    /// [0u8; 32] → modül yok (test/stub task'ları için).
    #[serde(
        serialize_with = "serialize_hash",
        deserialize_with = "deserialize_hash"
    )]
    pub wasm_module_hash: ModuleHash,

    pub entrypoint: String,

    pub state: TaskState,

    pub created_at: Timestamp,

    pub updated_at: Timestamp,
}

impl TaskDefinition {
    /// Hash sıfır mı? (modül yok)
    pub fn has_module(&self) -> bool {
        self.wasm_module_hash != [0u8; 32]
    }
}

// ── Serde helpers: hash ↔ hex string ─────────────────────
//
// JSON'da: "wasm_module_hash": "a3f1b2..."  (64 char hex)
// Persist boyutu: 32 byte binary → 64 byte hex
// vs eski: 100KB binary → 133KB base64 JSON
// Kazanım: ~%99.9 boyut azalması

fn serialize_hash<S>(
    hash: &ModuleHash,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&hex::encode(hash))
}

fn deserialize_hash<'de, D>(
    deserializer: D,
) -> Result<ModuleHash, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let hex_str = String::deserialize(deserializer)?;

    let bytes = hex::decode(&hex_str)
        .map_err(serde::de::Error::custom)?;

    if bytes.len() != 32 {
        return Err(serde::de::Error::custom(
            "expected 32-byte hash (64 hex chars)",
        ));
    }

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}
