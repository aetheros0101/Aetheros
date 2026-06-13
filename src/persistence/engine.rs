// ============================================================
// src/persistence/engine.rs  (v3)
//
// Optimizasyon #2: serde_json → rmp_serde (MessagePack)
//
// NEDEN SADECE PERSISTENCE?
//   API katmanı (axum) JSON döndürmeye devam eder.
//   Sadece sled'e yazılan binary format değişti.
//   Dışarıya görünür etki yok.
//
// KAZANIM (beklenti):
//   - Serialize: JSON ~4µs → MsgPack ~1µs  (%75 hızlı)
//   - Deserialize: JSON ~3µs → MsgPack ~1µs (%70 hızlı)
//   - Boyut: JSON 200B → MsgPack ~120B (%40 küçük)
//   - UTF-8 validation yok → CPU tasarrufu
//   - Binary array base64 değil raw bytes
//
// MİGRASYON NOTU:
//   Eski JSON formatındaki sled DB'ler okunamaz.
//   Yeni kurulum veya DB migration gerekir.
//   AETHEROS_DB_PATH değiştirilerek temiz başlangıç yapılır.
// ============================================================

use std::sync::Arc;

use sled::{
    Db,
    Tree,
};
use tokio::task::spawn_blocking;
use tokio::time::{
    interval,
    Duration,
};
use tracing::debug;

use crate::errors::persistence::PersistenceError;
use crate::persistence::models::PersistedTask;
use crate::task::task::TaskState;
use crate::types::ids::TaskId;

pub struct PersistenceEngine {
    database: Db,
    tasks: Tree,
    snapshots: Tree,
}

impl PersistenceEngine {
    pub fn open(
        path: &str,
    ) -> Result<Self, PersistenceError> {
        let database = sled::open(path)
            .map_err(|_| PersistenceError::StorageFailure)?;

        let tasks = database
            .open_tree("tasks_v2") // ← v2: msgpack format
            .map_err(|_| PersistenceError::StorageFailure)?;

        let snapshots = database
            .open_tree("snapshots_v2")
            .map_err(|_| PersistenceError::StorageFailure)?;

        Ok(Self {
            database,
            tasks,
            snapshots,
        })
    }

    /// Task'ı MessagePack formatında kaydet.
    ///
    /// rmp_serde::to_vec_named → field isimlerini korur
    /// (schema evrimi için önemli).
    pub fn persist_task(
        &self,
        task: &PersistedTask,
    ) -> Result<(), PersistenceError> {
        let key = task.task.id.0.as_bytes();

        // MessagePack serialize — JSON'a göre ~4x hızlı
        let value = rmp_serde::to_vec_named(task)
            .map_err(|_| {
                PersistenceError::SerializationFailure
            })?;

        debug!(
            task_id = %task.task.id.0,
            bytes = value.len(),
            "Persisting task (msgpack)"
        );

        self.tasks
            .insert(key, value)
            .map_err(|_| PersistenceError::StorageFailure)?;

        Ok(())
    }

    pub fn update_task_state(
        &self,
        task_id: &TaskId,
        new_state: TaskState,
    ) -> Result<(), PersistenceError> {
        let mut persisted = self
            .load_task(task_id)?
            .ok_or(PersistenceError::StorageFailure)?;

        persisted.task.state = new_state;
        persisted.updated_at = chrono::Utc::now();

        self.persist_task(&persisted)
    }

    /// Task'ı Failed durumuna geçir VE gerçek hata mesajını kaydet.
    ///
    /// Worker, WasmError::to_string() çıktısını buraya geçirir
    /// (örn. "missing entrypoint", "invalid module: module not
    /// found in store"). Bridge katmanı bu mesajı doğrudan
    /// TaskStatusResponse.error_message'a yansıtır — böylece
    /// ADB olmadan gerçek hata UI'da görülebilir.
    pub fn set_task_failed(
        &self,
        task_id: &TaskId,
        error: String,
    ) -> Result<(), PersistenceError> {
        let mut persisted = self
            .load_task(task_id)?
            .ok_or(PersistenceError::StorageFailure)?;

        persisted.task.state = TaskState::Failed;
        persisted.last_error = Some(error);
        persisted.updated_at = chrono::Utc::now();

        self.persist_task(&persisted)
    }

    pub fn load_task(
        &self,
        task_id: &TaskId,
    ) -> Result<Option<PersistedTask>, PersistenceError> {
        let value = self
            .tasks
            .get(task_id.0.as_bytes())
            .map_err(|_| PersistenceError::StorageFailure)?;

        match value {
            Some(bytes) => {
                // MessagePack deserialize
                let task = rmp_serde::from_slice(&bytes)
                    .map_err(|_| {
                        PersistenceError::SerializationFailure
                    })?;
                Ok(Some(task))
            }
            None => Ok(None),
        }
    }

    pub fn delete_task(
        &self,
        task_id: &TaskId,
    ) -> Result<(), PersistenceError> {
        self.tasks
            .remove(task_id.0.as_bytes())
            .map_err(|_| PersistenceError::StorageFailure)?;

        Ok(())
    }

    pub fn load_all_tasks(
        &self,
    ) -> Result<Vec<PersistedTask>, PersistenceError> {
        let mut tasks = Vec::new();

        for entry in self.tasks.iter() {
            let (_, value) = entry.map_err(|_| {
                PersistenceError::StorageFailure
            })?;

            let task = rmp_serde::from_slice(&value)
                .map_err(|_| {
                    PersistenceError::SerializationFailure
                })?;

            tasks.push(task);
        }

        Ok(tasks)
    }

    /// Async flush — executor thread'ini bloklamaz.
    pub async fn flush_async(
        &self,
    ) -> Result<(), PersistenceError> {
        let tasks = self.tasks.clone();

        spawn_blocking(move || {
            tasks
                .flush()
                .map(|_| ())
                .map_err(|_| PersistenceError::StorageFailure)
        })
        .await
        .map_err(|_| PersistenceError::StorageFailure)?
    }

    /// Arka plan flush — 100ms aralıkla.
    pub fn start_background_flush(
        self: Arc<Self>,
        period_ms: u64,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        tokio::spawn(async move {
            let mut ticker =
                interval(Duration::from_millis(period_ms));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        let _ = self.flush_async().await;
                    }
                    _ = shutdown_rx.changed() => {
                        let _ = self.flush_async().await;
                        break;
                    }
                }
            }
        });
    }

    pub fn database(&self) -> &Db {
        &self.database
    }

    pub fn snapshots(&self) -> &Tree {
        &self.snapshots
    }
}
