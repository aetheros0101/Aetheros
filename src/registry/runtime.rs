// ============================================================
// src/registry/runtime.rs
//
// Faz 8: RuntimeRegistry aktif hale getirildi.
//
// ÖNCE: runtimes: DashMap tanımlı ama hiç kullanılmıyordu.
//       dead_code warning.
//
// SONRA:
//   register() → runtime kaydeder
//   get()      → runtime bilgisini döndürür
//   remove()   → runtime'ı siler
//   list()     → tüm aktif runtime'ları listeler
//
// Kullanım: ApiServer /health endpoint'i tüm runtime'ları
// bu registry'den okuyacak (Faz 9).
// ============================================================

use dashmap::DashMap;
use serde::{
    Deserialize,
    Serialize,
};

use crate::types::ids::RuntimeId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeInfo {
    pub id: RuntimeId,
    pub label: String,
    pub healthy: bool,
}

pub struct RuntimeRegistry {
    runtimes: DashMap<RuntimeId, RuntimeInfo>,
}

impl RuntimeRegistry {
    pub fn new() -> Self {
        Self {
            runtimes: DashMap::new(),
        }
    }

    pub fn register(
        &self,
        info: RuntimeInfo,
    ) {
        self.runtimes.insert(info.id, info);
    }

    pub fn get(
        &self,
        id: &RuntimeId,
    ) -> Option<RuntimeInfo> {
        self.runtimes
            .get(id)
            .map(|entry| entry.clone())
    }

    pub fn remove(&self, id: &RuntimeId) {
        self.runtimes.remove(id);
    }

    pub fn list(&self) -> Vec<RuntimeInfo> {
        self.runtimes
            .iter()
            .map(|entry| entry.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.runtimes.len()
    }
}
