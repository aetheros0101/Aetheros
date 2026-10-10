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

use serde::{Deserialize, Serialize};

use crate::task::orchestration::TaskOrchestration;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::types::ids::ModuleHash;
use crate::types::ids::TaskId;
use crate::types::timestamps::Timestamp;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskMetadata {
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    ///
    /// NOT: Önceden hex-string custom (de)serializer kullanılıyordu
    /// (serialize_hash/deserialize_hash) — bu, persistence JSON
    /// kullanırken boyut optimizasyonu için eklenmişti. Persistence
    /// artık MessagePack (rmp-serde) kullanıyor; bu format [u8;32]'yi
    /// zaten native ve hex string'den daha küçük şekilde saklıyor.
    /// Custom hex katmanı kaldırıldı — gereksiz + persist/load
    /// round-trip'inde wasm_module_hash'in sıfırlanmasına yol açan
    /// şüpheli davranışın kaynağıydı (bkz. resubmit_task bug'ı).
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
