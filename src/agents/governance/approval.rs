// ============================================================
// src/agents/approval.rs
//
// V10 Sprint 5: Approval Engine — durdur & sonra devam ettir.
//
// SecurityGovernor bir ToolCall için RequiresApproval dediğinde,
// AgentRuntime execution'ı tool'u hiç invoke etmeden DURDURUR ve
// devam edebilmesi için gereken her şeyi burada saklar:
//   - hangi ToolCall onay bekliyor (ve neden)
//   - execution devam ederse çalıştırılacak KALAN adımlar
//   - kalan budget
//
// KAPSAM NOTU: Bu sprint'te depo in-memory (Arc<RwLock<HashMap>>) —
// ScriptRegistry/CapabilityEngine ile aynı desen, uygulama/runtime
// süreci ayakta olduğu sürece hayatta kalır. Süreç yeniden başlarsa
// bekleyen onaylar kaybolur. Disk'e (PersistenceEngine üzerinden)
// yazıp süreç yeniden başlasa bile hayatta kalmasını sağlamak,
// bilerek ayrı bir sertleştirme adımı olarak bırakıldı.
// ============================================================

use std::collections::HashMap;
use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tracing::{error, warn};
use uuid::Uuid;

use crate::persistence::engine::{PersistenceEngine, RecordKind};

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::plans::{AgentPlanStep, ToolCall};

/// Otonom döngüde duraklama anına kadar çalışan adımların kalıcı özeti.
/// Onaydan sonra planlayıcı "şimdiye kadar ne oldu"yu bilerek devam eder (B6).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StepSnapshot {
    pub step_name: String,
    pub success: bool,
    pub output: String,
    pub tool_call: Option<ToolCall>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingApproval {
    pub id: Uuid,
    pub context: AgentContext,
    pub objective: String,
    /// Onay bekleyen, henüz invoke EDİLMEMİŞ çağrı.
    pub tool_call: ToolCall,
    /// SecurityGovernor'ın RequiresApproval derken verdiği gerekçe.
    pub reason: String,
    /// Onaylanırsa (bu tool_call çalıştırıldıktan SONRA) devam
    /// edilecek adımlar — mevcut adım hariç.
    pub remaining_steps: Vec<AgentPlanStep>,
    /// Duraklatma anındaki kalan budget (harcanan düşülmüş).
    pub budget: AgentExecutionBudget,
    pub created_at: DateTime<Utc>,
    /// true: duraklayan execution OTONOM döngüdeydi; onaydan sonra sabit
    /// `remaining_steps` yerine planlayıcıyla devam edilir. Eski kayıtlar
    /// (alan yok) false okunur → eski davranış.
    #[serde(default)]
    pub autonomous: bool,
    /// Otonom döngünün o ana kadarki geçmişi (yalnız `autonomous` iken dolu).
    #[serde(default)]
    pub history: Vec<StepSnapshot>,
}

/// Bir onayın, verilmeden bekleyebileceği varsayılan süre. Eski bir
/// "git commit" onayının günler sonra hâlâ verilebilir olması istenmez.
pub const DEFAULT_APPROVAL_TTL_MINUTES: i64 = 60;

pub struct ApprovalStore {
    pending: RwLock<HashMap<Uuid, PendingApproval>>,
    /// V10 B3: varsa her değişiklik diske de yazılır ve açılışta geri
    /// yüklenir (Android süreci öldürse bile bekleyen onay kaybolmaz).
    persistence: Option<Arc<PersistenceEngine>>,
    ttl: Duration,
}

impl ApprovalStore {
    pub fn new() -> Self {
        Self {
            pending: RwLock::new(HashMap::new()),
            persistence: None,
            ttl: Duration::minutes(DEFAULT_APPROVAL_TTL_MINUTES),
        }
    }

    /// Kalıcı depo: önceki oturumdan kalan bekleyen onayları geri yükler.
    /// Okunamayan kayıtlar atlanır (loglanır). Süresi dolanları geri
    /// yüklemeyi `purge_expired()` çağıran taraf halleder.
    pub fn with_persistence(engine: Arc<PersistenceEngine>) -> Self {
        let mut map = HashMap::new();
        match engine.load_records(RecordKind::Approvals) {
            Ok(records) => {
                for (_key, bytes) in records {
                    match serde_json::from_slice::<PendingApproval>(&bytes) {
                        Ok(p) => {
                            map.insert(p.id, p);
                        }
                        Err(e) => warn!(err = %e, "Bekleyen onay kaydı okunamadı, atlanıyor"),
                    }
                }
            }
            Err(e) => error!(err = %e, "Bekleyen onaylar diskten yüklenemedi"),
        }
        Self {
            pending: RwLock::new(map),
            persistence: Some(engine),
            ttl: Duration::minutes(DEFAULT_APPROVAL_TTL_MINUTES),
        }
    }

    pub fn with_ttl(mut self, ttl: Duration) -> Self {
        self.ttl = ttl;
        self
    }

    pub fn is_expired(&self, approval: &PendingApproval) -> bool {
        Utc::now() - approval.created_at > self.ttl
    }

    /// Süresi dolan onayları bellekten VE diskten kaldırıp döndürür
    /// (çağıran, ilgili execution'ı "reddedildi/süresi doldu" işaretler).
    pub fn purge_expired(&self) -> Vec<PendingApproval> {
        let expired_ids: Vec<Uuid> = self
            .read_pending()
            .values()
            .filter(|p| self.is_expired(p))
            .map(|p| p.id)
            .collect();

        expired_ids.iter().filter_map(|id| self.take(id)).collect()
    }

    /// Lock zehirlenmişse (başka bir thread panic ettiyse) onay kuyruğu
    /// tüm süreci düşürmesin; HashMap ekle/çıkar işlemleri sırasında
    /// tutarsız ara durum bırakmadığı için içerik güvenle kullanılır.
    fn read_pending(&self) -> RwLockReadGuard<'_, HashMap<Uuid, PendingApproval>> {
        self.pending.read().unwrap_or_else(|poisoned| {
            warn!("ApprovalStore read lock zehirlenmiş — veri kurtarıldı");
            poisoned.into_inner()
        })
    }

    fn write_pending(&self) -> RwLockWriteGuard<'_, HashMap<Uuid, PendingApproval>> {
        self.pending.write().unwrap_or_else(|poisoned| {
            warn!("ApprovalStore write lock zehirlenmiş — veri kurtarıldı");
            poisoned.into_inner()
        })
    }

    fn persist(&self, approval: &PendingApproval) {
        let Some(engine) = &self.persistence else { return };
        match serde_json::to_vec(approval) {
            Ok(bytes) => {
                let written = engine
                    .put_record(RecordKind::Approvals, approval.id.as_bytes(), &bytes)
                    .and_then(|_| engine.flush_records());
                if let Err(e) = written {
                    error!(approval_id = %approval.id, err = %e,
                        "Bekleyen onay diske yazılamadı — uygulama kapanırsa kaybolur");
                }
            }
            Err(e) => error!(err = %e, "Bekleyen onay serileştirilemedi"),
        }
    }

    fn forget(&self, id: &Uuid) {
        let Some(engine) = &self.persistence else { return };
        let removed = engine
            .delete_record(RecordKind::Approvals, id.as_bytes())
            .and_then(|_| engine.flush_records());
        if let Err(e) = removed {
            // Silinemezse yeniden başlatmada kayıt "hayalet" olarak döner.
            error!(approval_id = %id, err = %e, "Onay kaydı diskten silinemedi");
        }
    }

    pub fn add(&self, approval: PendingApproval) {
        self.persist(&approval);
        self.write_pending().insert(approval.id, approval);
    }

    pub fn list(&self) -> Vec<PendingApproval> {
        self.read_pending().values().cloned().collect()
    }

    pub fn get(&self, id: &Uuid) -> Option<PendingApproval> {
        self.read_pending().get(id).cloned()
    }

    /// Kaydı çıkar ve döndür — onaylanınca ya da reddedilince
    /// çağrılır; iki kere işlenmesin diye kaldırma atomik.
    pub fn take(&self, id: &Uuid) -> Option<PendingApproval> {
        let taken = self.write_pending().remove(id);
        if taken.is_some() {
            self.forget(id);
        }
        taken
    }
}

impl Default for ApprovalStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: Uuid) -> PendingApproval {
        PendingApproval {
            id,
            context: AgentContext {
                agent_id: Uuid::new_v4(),
                execution_id: Uuid::new_v4(),
                workflow_id: None,
            },
            objective: "test".to_string(),
            tool_call: ToolCall {
                tool_name: "risky_tool".to_string(),
                arguments: vec![],
            },
            reason: "high risk".to_string(),
            remaining_steps: vec![],
            budget: AgentExecutionBudget {
                max_tokens: 100,
                max_steps: 5,
                max_runtime_seconds: 30,
            },
            created_at: Utc::now(),
            autonomous: false,
            history: vec![],
        }
    }

    #[test]
    fn add_and_list_roundtrip() {
        let store = ApprovalStore::new();
        let id = Uuid::new_v4();
        store.add(sample(id));
        assert_eq!(store.list().len(), 1);
        assert!(store.get(&id).is_some());
    }

    #[test]
    fn take_removes_and_returns_once() {
        let store = ApprovalStore::new();
        let id = Uuid::new_v4();
        store.add(sample(id));

        assert!(store.take(&id).is_some());
        assert!(store.take(&id).is_none(), "ikinci take None dönmeli");
        assert!(store.list().is_empty());
    }

    // ── B3: kalıcılık + TTL ─────────────────────────────────

    fn engine_at(path: &std::path::Path) -> Arc<PersistenceEngine> {
        Arc::new(PersistenceEngine::open(path.to_str().unwrap()).expect("db açılmalı"))
    }

    #[test]
    fn pending_approval_survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        {
            let store = ApprovalStore::with_persistence(engine_at(dir.path()));
            store.add(sample(id));
            assert_eq!(store.list().len(), 1);
        } // store + engine düşer → sled kilidi bırakılır

        let store = ApprovalStore::with_persistence(engine_at(dir.path()));
        let restored = store.get(&id).expect("onay yeniden açılışta geri gelmeli");
        assert_eq!(restored.tool_call.tool_name, "risky_tool");
        assert_eq!(restored.objective, "test");
    }

    #[test]
    fn taken_approval_does_not_come_back_after_reopen() {
        let dir = tempfile::tempdir().unwrap();
        let id = Uuid::new_v4();
        {
            let store = ApprovalStore::with_persistence(engine_at(dir.path()));
            store.add(sample(id));
            assert!(store.take(&id).is_some());
        }
        let store = ApprovalStore::with_persistence(engine_at(dir.path()));
        assert!(store.list().is_empty(), "alınan (işlenen) onay hayalet olarak dönmemeli");
    }

    #[test]
    fn purge_expired_removes_only_stale_from_memory_and_disk() {
        let dir = tempfile::tempdir().unwrap();
        let (stale_id, fresh_id) = (Uuid::new_v4(), Uuid::new_v4());
        {
            let store = ApprovalStore::with_persistence(engine_at(dir.path()))
                .with_ttl(Duration::seconds(30));
            let mut stale = sample(stale_id);
            stale.created_at = Utc::now() - Duration::seconds(120);
            store.add(stale);
            store.add(sample(fresh_id));

            let purged = store.purge_expired();
            assert_eq!(purged.len(), 1);
            assert_eq!(purged[0].id, stale_id);
            assert!(store.get(&stale_id).is_none());
            assert!(store.get(&fresh_id).is_some());
        }
        let store = ApprovalStore::with_persistence(engine_at(dir.path()));
        assert!(store.get(&stale_id).is_none(), "süresi dolan diskten de silinmeli");
        assert!(store.get(&fresh_id).is_some());
    }

    #[test]
    fn is_expired_uses_ttl_against_created_at() {
        let store = ApprovalStore::new().with_ttl(Duration::minutes(10));
        let mut p = sample(Uuid::new_v4());
        assert!(!store.is_expired(&p));
        p.created_at = Utc::now() - Duration::minutes(11);
        assert!(store.is_expired(&p));
    }
}
