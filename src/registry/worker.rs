// ============================================================
// src/registry/worker.rs
//
// Faz 8: WorkerRegistry aktif hale getirildi.
//
// ÖNCE: workers: DashMap tanımlı ama hiç kullanılmıyordu.
//       dead_code warning.
//
// SONRA:
//   register()      → worker kaydeder
//   update_state()  → WorkerState günceller
//   remove()        → worker'ı siler
//   list()          → tüm worker'ları döndürür
//   count_by_state()→ state bazlı istatistik
//
// WorkerSupervisor, spawn/crash/respawn döngüsünde
// bu registry'yi günceller (Faz 9 entegrasyonu).
// ============================================================

use dashmap::DashMap;

use crate::types::ids::WorkerId;
use crate::worker::state::WorkerState;

pub struct WorkerRegistry {
    workers: DashMap<WorkerId, WorkerState>,
}

impl WorkerRegistry {
    pub fn new() -> Self {
        Self {
            workers: DashMap::new(),
        }
    }

    pub fn register(
        &self,
        id: WorkerId,
        state: WorkerState,
    ) {
        self.workers.insert(id, state);
    }

    pub fn update_state(
        &self,
        id: &WorkerId,
        state: WorkerState,
    ) {
        if let Some(mut entry) = self.workers.get_mut(id) {
            *entry = state;
        }
    }

    pub fn remove(&self, id: &WorkerId) {
        self.workers.remove(id);
    }

    pub fn list(
        &self,
    ) -> Vec<(WorkerId, WorkerState)> {
        self.workers
            .iter()
            .map(|e| (*e.key(), *e.value()))
            .collect()
    }

    pub fn count_by_state(
        &self,
        state: WorkerState,
    ) -> usize {
        self.workers
            .iter()
            .filter(|e| *e.value() == state)
            .count()
    }

    pub fn total(&self) -> usize {
        self.workers.len()
    }
}
