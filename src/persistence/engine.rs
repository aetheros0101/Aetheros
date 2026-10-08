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

use sled::{Db, Tree};
use tokio::task::spawn_blocking;
use tokio::time::{Duration, interval};
use tracing::{debug, warn};

use crate::errors::persistence::PersistenceError;
use crate::persistence::encryption::{AtRestCipher, cipher_from_env};
use crate::persistence::models::PersistedTask;
use crate::task::task::TaskState;
use crate::types::ids::TaskId;

/// V10 B3: genel amaçlı, çağıran tarafından serileştirilen kayıt türleri.
/// Task'lardan farklı olarak bu kayıtların şeması çağıranda (ApprovalStore,
/// AuditLog); engine yalnızca saklar (+ at-rest şifreleme uygular).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecordKind {
    /// Bekleyen onaylar (agents::approval::PendingApproval, JSON).
    Approvals,
    /// Denetim izi (logging::audit::AuditEvent, JSON).
    Audit,
}

pub struct PersistenceEngine {
    database: Db,
    tasks: Tree,
    snapshots: Tree,
    approvals: Tree,
    audit: Tree,
    /// At-rest şifreleme kancası — bkz. src/persistence/encryption.rs.
    /// `AETHEROS_ENCRYPTION_KEY` (64 hex) ayarlıysa AES-256-GCM; ayarlı
    /// değilse NoopCipher (değerler düz yazılır, açılışta uyarı loglanır).
    cipher: Box<dyn AtRestCipher>,
}

/// Ham (anahtar, değer) kaydı — sled tree'den okunan şifresiz bayt çifti.
pub type RawRecord = (Vec<u8>, Vec<u8>);

impl PersistenceEngine {
    pub fn open(path: &str) -> Result<Self, PersistenceError> {
        let database = sled::open(path).map_err(|_| PersistenceError::StorageFailure)?;

        let tasks = database
            .open_tree("tasks_v2") // ← v2: msgpack format
            .map_err(|_| PersistenceError::StorageFailure)?;

        let snapshots = database
            .open_tree("snapshots_v2")
            .map_err(|_| PersistenceError::StorageFailure)?;

        let approvals = database
            .open_tree("approvals_v1")
            .map_err(|_| PersistenceError::StorageFailure)?;

        let audit = database
            .open_tree("audit_v1")
            .map_err(|_| PersistenceError::StorageFailure)?;

        Ok(Self {
            database,
            tasks,
            snapshots,
            approvals,
            audit,
            cipher: cipher_from_env(),
        })
    }

    fn record_tree(&self, kind: RecordKind) -> &Tree {
        match kind {
            RecordKind::Approvals => &self.approvals,
            RecordKind::Audit => &self.audit,
        }
    }

    /// Bir kaydı (zaten serileştirilmiş baytlar) at-rest şifreleme
    /// kancasından geçirip yazar. Aynı anahtar varsa üzerine yazar.
    pub fn put_record(
        &self,
        kind: RecordKind,
        key: &[u8],
        value: &[u8],
    ) -> Result<(), PersistenceError> {
        let value = self.cipher.encrypt(value)?;
        self.record_tree(kind)
            .insert(key, value)
            .map_err(|_| PersistenceError::StorageFailure)?;
        Ok(())
    }

    pub fn delete_record(&self, kind: RecordKind, key: &[u8]) -> Result<(), PersistenceError> {
        self.record_tree(kind)
            .remove(key)
            .map_err(|_| PersistenceError::StorageFailure)?;
        Ok(())
    }

    /// Bir türün tüm kayıtları, ANAHTAR sırasıyla (sled sıralıdır) —
    /// `(anahtar, çözülmüş değer)`. Okunamayan/şifresi çözülemeyen tek bir
    /// kayıt tüm listeyi mahvetmesin diye atlanır ve loglanır.
    pub fn load_records(&self, kind: RecordKind) -> Result<Vec<RawRecord>, PersistenceError> {
        let mut out = Vec::new();
        for entry in self.record_tree(kind).iter() {
            let (key, value) = match entry {
                Ok(kv) => kv,
                Err(e) => {
                    warn!(err = %e, ?kind, "Kayıt iter hatası, atlanıyor");
                    continue;
                }
            };
            match self.cipher.decrypt(&value) {
                Ok(plain) => out.push((key.to_vec(), plain)),
                Err(e) => {
                    warn!(err = %e, ?kind, "Kayıt şifre çözme hatası, atlanıyor");
                }
            }
        }
        Ok(out)
    }

    /// Onay/denetim kayıtlarını diske zorla yazar (onay kaydı kritik:
    /// uygulama hemen ölse bile kaybolmamalı).
    pub fn flush_records(&self) -> Result<(), PersistenceError> {
        self.approvals
            .flush()
            .map_err(|_| PersistenceError::StorageFailure)?;
        self.audit
            .flush()
            .map_err(|_| PersistenceError::StorageFailure)?;
        Ok(())
    }

    /// Task'ı MessagePack formatında kaydet.
    ///
    /// rmp_serde::to_vec_named → field isimlerini korur
    /// (schema evrimi için önemli).
    pub fn persist_task(&self, task: &PersistedTask) -> Result<(), PersistenceError> {
        let key = task.task.id.0.as_bytes();

        // MessagePack serialize — JSON'a göre ~4x hızlı
        let value =
            rmp_serde::to_vec_named(task).map_err(|_| PersistenceError::SerializationFailure)?;

        // At-rest şifreleme kancasından geçir (bkz. persistence/encryption.rs).
        let value = self.cipher.encrypt(&value)?;

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
    pub fn set_task_failed(&self, task_id: &TaskId, error: String) -> Result<(), PersistenceError> {
        let mut persisted = self
            .load_task(task_id)?
            .ok_or(PersistenceError::StorageFailure)?;

        persisted.task.state = TaskState::Failed;
        persisted.last_error = Some(error);
        persisted.updated_at = chrono::Utc::now();

        self.persist_task(&persisted)
    }

    pub fn load_task(&self, task_id: &TaskId) -> Result<Option<PersistedTask>, PersistenceError> {
        let value = self
            .tasks
            .get(task_id.0.as_bytes())
            .map_err(|_| PersistenceError::StorageFailure)?;

        match value {
            Some(bytes) => {
                let bytes = self.cipher.decrypt(&bytes)?;
                // MessagePack deserialize
                let task = rmp_serde::from_slice(&bytes)
                    .map_err(|_| PersistenceError::SerializationFailure)?;
                Ok(Some(task))
            }
            None => Ok(None),
        }
    }

    pub fn delete_task(&self, task_id: &TaskId) -> Result<(), PersistenceError> {
        self.tasks
            .remove(task_id.0.as_bytes())
            .map_err(|_| PersistenceError::StorageFailure)?;

        Ok(())
    }

    pub fn load_all_tasks(&self) -> Result<Vec<PersistedTask>, PersistenceError> {
        let mut tasks = Vec::new();
        let mut skipped = 0usize;

        for entry in self.tasks.iter() {
            let (key, value) = match entry {
                Ok(kv) => kv,
                Err(e) => {
                    warn!(err = %e, "Sled iter hatası, kayıt atlanıyor");
                    skipped += 1;
                    continue;
                }
            };

            let decrypted = match self.cipher.decrypt(&value) {
                Ok(d) => d,
                Err(e) => {
                    warn!(
                        key = %String::from_utf8_lossy(&key),
                        err = %e,
                        "Kayıt şifre çözme hatası, atlanıyor"
                    );
                    skipped += 1;
                    continue;
                }
            };

            match rmp_serde::from_slice::<PersistedTask>(&decrypted) {
                Ok(task) => tasks.push(task),
                Err(e) => {
                    // Eski format (hex serde dönemi) veya schema değişikliği.
                    // Tek kayıt bozuksa tüm listeyi mahvetme — atla ve logla.
                    warn!(
                        key  = %String::from_utf8_lossy(&key),
                        err  = %e,
                        "Kayıt deserialize edilemedi, atlanıyor (eski format?)"
                    );
                    skipped += 1;
                }
            }
        }

        if skipped > 0 {
            warn!(skipped, "load_all_tasks: bazı kayıtlar atlandı");
        }

        Ok(tasks)
    }

    /// Async flush — executor thread'ini bloklamaz.
    pub async fn flush_async(&self) -> Result<(), PersistenceError> {
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
            let mut ticker = interval(Duration::from_millis(period_ms));

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

    /// At-rest cipher'ı dışarıya sızdırmadan kullanmak isteyen diğer
    /// depolama katmanları (ör. ModuleStore) için ince wrapper'lar.
    /// Böylece "tek şifreleme noktası" PersistenceEngine'de kalır.
    pub fn encrypt_bytes(&self, plaintext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        self.cipher.encrypt(plaintext)
    }

    pub fn decrypt_bytes(&self, ciphertext: &[u8]) -> Result<Vec<u8>, PersistenceError> {
        self.cipher.decrypt(ciphertext)
    }
}
