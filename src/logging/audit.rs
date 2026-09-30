// ============================================================
// src/logging/audit.rs
//
// V10 Sprint 6: Audit Log
//
// Önceki AuditEvent tanımı (`event_type: String` + `timestamp`) hiçbir
// yerde kullanılmıyordu — ne yazan bir şey vardı ne de gerçek bir olay
// taşıyordu. Bu yeniden yazım, Governor'ın verdiği her kararı, her
// tool çağrısını ve her duraklama/devam olayını GERÇEKTEN kaydeden,
// sorgulanabilir bir defter.
//
// KAPSAM NOTU: ApprovalStore ile aynı desen — in-memory (Arc<RwLock<Vec>>),
// süreç ayakta olduğu sürece hayatta kalır. Gerçek bir denetim izi
// normalde diske/sled'e yazılır (uygulama kapansa bile hayatta kalır);
// bu, bilerek ayrı bırakılmış bir sertleştirme adımı. Aşırı büyümeyi
// önlemek için basit bir üst sınır (max_events) var — dolunca en eski
// kayıtlar düşer.
// ============================================================

use std::collections::VecDeque;
use std::sync::RwLock;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tek bir denetim olayının TÜRÜ ve o türe özgü ayrıntılar.
/// Serbest metin `event_type: String` yerine — yanlış yazılmış bir
/// tür ismi artık derleme zamanında yakalanır, çalışma zamanında değil.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AuditEventKind {
    /// SecurityGovernor bir tool çağrısını değerlendirdi.
    GovernorDecision {
        tool_name: String,
        /// "allow" | "deny" | "requires_approval"
        decision: String,
        reason: Option<String>,
    },
    /// Bir tool gerçekten invoke edildi (Governor'dan Allow aldıktan
    /// SONRA, ya da onay sonrası resume'da).
    ToolInvoked {
        tool_name: String,
        success: bool,
        error: Option<String>,
    },
    /// Execution, RequiresApproval nedeniyle duraklatıldı.
    ExecutionPaused {
        approval_id: Uuid,
        tool_name: String,
        reason: String,
    },
    /// Kullanıcı onayladı, execution kaldığı yerden devam ediyor.
    ExecutionResumed { approval_id: Uuid },
    /// Kullanıcı reddetti — execution kalıcı olarak bitti.
    ApprovalDenied { approval_id: Uuid, reason: String },
    /// Plan sonuna kadar hatasız tamamlandı.
    ExecutionCompleted,
    /// Execution bir hatayla bitti (retryable olmayan adım hatası vb.).
    ExecutionFailed { error: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub execution_id: Uuid,
    pub kind: AuditEventKind,
    pub timestamp: DateTime<Utc>,
}

pub struct AuditLog {
    events: RwLock<VecDeque<AuditEvent>>,
    max_events: usize,
}

impl AuditLog {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::new()),
            max_events,
        }
    }

    pub fn record(&self, agent_id: Uuid, execution_id: Uuid, kind: AuditEventKind) {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            agent_id,
            execution_id,
            kind,
            timestamp: Utc::now(),
        };

        let mut events = self.events.write().unwrap();
        if events.len() >= self.max_events {
            events.pop_front();
        }
        events.push_back(event);
    }

    /// Tüm kayıtlar, en yeni en sonda (kronolojik).
    pub fn list(&self) -> Vec<AuditEvent> {
        self.events.read().unwrap().iter().cloned().collect()
    }

    /// Tek bir execution'a ait kayıtlar — "bu agent tam olarak ne yaptı?"
    /// sorusunun cevabı.
    pub fn list_for_execution(&self, execution_id: Uuid) -> Vec<AuditEvent> {
        self.events
            .read()
            .unwrap()
            .iter()
            .filter(|e| e.execution_id == execution_id)
            .cloned()
            .collect()
    }

    pub fn count(&self) -> usize {
        self.events.read().unwrap().len()
    }
}

impl Default for AuditLog {
    fn default() -> Self {
        // Makul bir varsayılan — mobil cihazda sınırsız büyümesin.
        Self::new(10_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn record_and_list_roundtrip() {
        let log = AuditLog::new(100);
        let agent_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();

        log.record(
            agent_id,
            execution_id,
            AuditEventKind::ExecutionCompleted,
        );

        let events = log.list();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].agent_id, agent_id);
        assert_eq!(events[0].execution_id, execution_id);
        assert_eq!(events[0].kind, AuditEventKind::ExecutionCompleted);
    }

    #[test]
    fn list_for_execution_filters_correctly() {
        let log = AuditLog::new(100);
        let exec_a = Uuid::new_v4();
        let exec_b = Uuid::new_v4();
        let agent_id = Uuid::new_v4();

        log.record(agent_id, exec_a, AuditEventKind::ExecutionCompleted);
        log.record(agent_id, exec_b, AuditEventKind::ExecutionCompleted);
        log.record(agent_id, exec_a, AuditEventKind::ExecutionFailed {
            error: "x".to_string(),
        });

        assert_eq!(log.list_for_execution(exec_a).len(), 2);
        assert_eq!(log.list_for_execution(exec_b).len(), 1);
    }

    #[test]
    fn oldest_event_dropped_when_over_capacity() {
        let log = AuditLog::new(2);
        let agent_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();

        log.record(agent_id, execution_id, AuditEventKind::ExecutionPaused {
            approval_id: Uuid::new_v4(),
            tool_name: "first".to_string(),
            reason: "r".to_string(),
        });
        log.record(agent_id, execution_id, AuditEventKind::ExecutionResumed {
            approval_id: Uuid::new_v4(),
        });
        log.record(agent_id, execution_id, AuditEventKind::ExecutionCompleted);

        let events = log.list();
        assert_eq!(events.len(), 2, "kapasite aşılınca en eski düşmeli");
        // İlk olay ("first" ile ilgili ExecutionPaused) artık yok.
        assert!(!events.iter().any(|e| matches!(
            &e.kind,
            AuditEventKind::ExecutionPaused { tool_name, .. } if tool_name == "first"
        )));
    }
}
