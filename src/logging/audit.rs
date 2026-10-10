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
use std::sync::{Arc, RwLock};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{error, warn};
use uuid::Uuid;

use crate::persistence::engine::{PersistenceEngine, RecordKind};

// ── Denetim izine yazılan metinlerin sınırları ──────────────
// Audit "ne çalıştırıldı?" sorusunu cevaplamalı (argümanlar), ama
// iz sınırsız büyüyüp sırları da saklamamalı. Bu yüzden argümanlar
// yazılırken sınırlanır ve sır-gibi görünenler maskelenir. NOT: bu
// bir sezgisel (heuristic) filtredir, tam bir sır tarayıcı DEĞİL.
const MAX_AUDIT_ARGS: usize = 32;
const MAX_AUDIT_ARG_CHARS: usize = 512;
const MAX_AUDIT_TEXT_CHARS: usize = 2_000;
/// Agent'ın kullanıcıya yazdığı cevap metni için üst sınır.
const MAX_ASSISTANT_TEXT_CHARS: usize = 4_000;
const OUTPUT_PREVIEW_CHARS: usize = 120;
const REDACTED: &str = "[REDACTED]";

/// `s`'nin ilk `max` karakterini döner; kesildiyse sonuna '…' ekler
/// (UTF-8 karakter sınırında güvenli).
pub fn clip(s: &str, max: usize) -> String {
    let mut it = s.chars();
    let head: String = it.by_ref().take(max).collect();
    if it.next().is_some() {
        format!("{head}…")
    } else {
        head
    }
}

fn looks_secret(s: &str) -> bool {
    let l = s.to_lowercase();
    l.contains("password=")
        || l.contains("passwd=")
        || l.contains("token=")
        || l.contains("secret=")
        || l.contains("api_key=")
        || l.contains("apikey=")
        || l.contains("authorization:")
        || l.starts_with("bearer ")
        || s.starts_with("sk-")
        || s.starts_with("AKIA")
        || s.contains("-----BEGIN")
}

/// Bir sonraki argümanın değeri sır olan bayraklar (`--password x`).
fn is_secret_flag(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "--password" | "--passwd" | "--token" | "--secret" | "--api-key" | "--apikey"
    )
}

/// Denetime yazılacak argüman listesini sınırlar ve sır-gibi
/// görünenleri maskeler. `arguments[0]` (program) olduğu gibi kalır.
pub fn sanitize_arguments(args: &[String]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut redact_next = false;
    for a in args.iter().take(MAX_AUDIT_ARGS) {
        if redact_next {
            out.push(REDACTED.to_string());
            redact_next = false;
            continue;
        }
        if is_secret_flag(a) {
            redact_next = true;
            out.push(clip(a, MAX_AUDIT_ARG_CHARS));
        } else if looks_secret(a) {
            out.push(REDACTED.to_string());
        } else {
            out.push(clip(a, MAX_AUDIT_ARG_CHARS));
        }
    }
    if args.len() > MAX_AUDIT_ARGS {
        out.push(format!("… (+{} argüman daha)", args.len() - MAX_AUDIT_ARGS));
    }
    out
}

/// Bir tool çıktısının denetim için KAYIPLI özeti: boyut + SHA-256
/// (içerik değişmediğini kanıtlar) + kısa, sır-süzgeçli önizleme.
/// Ham çıktı audit'e yazılmaz (hassas olabilir, çok büyük olabilir).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OutputSummary {
    pub bytes: usize,
    pub sha256: String,
    pub preview: String,
}

pub fn summarize_output(output: &str) -> OutputSummary {
    let hash: [u8; 32] = Sha256::digest(output.as_bytes()).into();
    let raw_preview = clip(output, OUTPUT_PREVIEW_CHARS).replace(['\n', '\r'], " ");
    let preview = if looks_secret(&raw_preview) {
        REDACTED.to_string()
    } else {
        raw_preview
    };
    OutputSummary {
        bytes: output.len(),
        sha256: hex::encode(hash),
        preview,
    }
}

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
        /// Değerlendirilen çağrının argümanları (terminal için
        /// `[program, arg…]`). `record()` içinde sınırlanır + maskelenir.
        #[serde(default)]
        arguments: Vec<String>,
    },
    /// Bir tool gerçekten invoke edildi (Governor'dan Allow aldıktan
    /// SONRA, ya da onay sonrası resume'da).
    ToolInvoked {
        tool_name: String,
        success: bool,
        error: Option<String>,
        /// Çalıştırılan çağrının argümanları (sınırlanmış + maskelenmiş).
        #[serde(default)]
        arguments: Vec<String>,
        /// Başarılıysa çıktının özeti (boyut + SHA-256 + önizleme).
        #[serde(default)]
        output: Option<OutputSummary>,
    },
    /// Execution, RequiresApproval nedeniyle duraklatıldı.
    ExecutionPaused {
        approval_id: Uuid,
        tool_name: String,
        reason: String,
        /// Onay bekleyen çağrının argümanları (sınırlanmış + maskelenmiş).
        #[serde(default)]
        arguments: Vec<String>,
    },
    /// Kullanıcı onayladı, execution kaldığı yerden devam ediyor.
    ExecutionResumed { approval_id: Uuid },
    /// Kullanıcı reddetti — execution kalıcı olarak bitti.
    ApprovalDenied { approval_id: Uuid, reason: String },
    /// Plan sonuna kadar hatasız tamamlandı.
    ExecutionCompleted,
    /// Execution bir hatayla bitti (retryable olmayan adım hatası vb.).
    ExecutionFailed { error: String },
    /// Agent'ın kullanıcıya yazdığı düz metin cevap (selamlama, soru
    /// yanıtı, iş bitince kısa özet). Araç çağrısı değildir.
    AssistantMessage { text: String },
}

impl AuditEventKind {
    /// Yazmadan önce sınırlar: argümanları süzer (maskele + kırp),
    /// serbest metin alanlarını (hata/gerekçe) kırpar. Terminal stderr'i
    /// megabaytlarca olabilir; 10 000 olaylık tampon bunu kaldıramaz.
    pub(crate) fn bounded(self) -> Self {
        match self {
            AuditEventKind::GovernorDecision {
                tool_name,
                decision,
                reason,
                arguments,
            } => AuditEventKind::GovernorDecision {
                tool_name,
                decision,
                reason: reason.map(|r| clip(&r, MAX_AUDIT_TEXT_CHARS)),
                arguments: sanitize_arguments(&arguments),
            },
            AuditEventKind::ToolInvoked {
                tool_name,
                success,
                error,
                arguments,
                output,
            } => AuditEventKind::ToolInvoked {
                tool_name,
                success,
                error: error.map(|e| clip(&e, MAX_AUDIT_TEXT_CHARS)),
                arguments: sanitize_arguments(&arguments),
                output,
            },
            AuditEventKind::ExecutionPaused {
                approval_id,
                tool_name,
                reason,
                arguments,
            } => AuditEventKind::ExecutionPaused {
                approval_id,
                tool_name,
                reason: clip(&reason, MAX_AUDIT_TEXT_CHARS),
                arguments: sanitize_arguments(&arguments),
            },
            AuditEventKind::ApprovalDenied {
                approval_id,
                reason,
            } => AuditEventKind::ApprovalDenied {
                approval_id,
                reason: clip(&reason, MAX_AUDIT_TEXT_CHARS),
            },
            AuditEventKind::ExecutionFailed { error } => AuditEventKind::ExecutionFailed {
                error: clip(&error, MAX_AUDIT_TEXT_CHARS),
            },
            AuditEventKind::AssistantMessage { text } => AuditEventKind::AssistantMessage {
                text: clip(&text, MAX_ASSISTANT_TEXT_CHARS),
            },
            other => other,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub agent_id: Uuid,
    pub execution_id: Uuid,
    pub kind: AuditEventKind,
    pub timestamp: DateTime<Utc>,
}

/// Diskteki anahtar: zaman (µs, big-endian) + olay kimliği → sled anahtar
/// sırası kronolojik sıradır. Aynı mikrosaniyedeki iki olayın göreli sırası
/// kimliğe bağlıdır (üretimde önemsiz).
fn audit_key(event: &AuditEvent) -> Vec<u8> {
    let micros = event.timestamp.timestamp_micros().max(0) as u64;
    let mut key = Vec::with_capacity(24);
    key.extend_from_slice(&micros.to_be_bytes());
    key.extend_from_slice(event.id.as_bytes());
    key
}

pub struct AuditLog {
    events: RwLock<VecDeque<AuditEvent>>,
    max_events: usize,
    /// V10 B3: varsa her olay diske de yazılır, açılışta son `max_events`
    /// olay geri yüklenir. Yazma hatası execution'ı DURDURMAZ (loglanır).
    persistence: Option<Arc<PersistenceEngine>>,
}

impl AuditLog {
    pub fn new(max_events: usize) -> Self {
        Self {
            events: RwLock::new(VecDeque::new()),
            max_events,
            persistence: None,
        }
    }

    /// Kalıcı denetim izi: önceki oturumların son `max_events` olayını
    /// geri yükler; daha eski diskteki kayıtları siler.
    pub fn with_persistence(engine: Arc<PersistenceEngine>, max_events: usize) -> Self {
        let mut restored: VecDeque<AuditEvent> = VecDeque::new();
        let mut keys: Vec<Vec<u8>> = Vec::new();

        match engine.load_records(RecordKind::Audit) {
            Ok(records) => {
                for (key, bytes) in records {
                    match serde_json::from_slice::<AuditEvent>(&bytes) {
                        Ok(ev) => {
                            restored.push_back(ev);
                            keys.push(key);
                        }
                        Err(e) => warn!(err = %e, "Denetim kaydı okunamadı, atlanıyor"),
                    }
                }
            }
            Err(e) => error!(err = %e, "Denetim izi diskten yüklenemedi"),
        }

        // Sınırı aşan en eski kayıtları bellekten ve diskten at.
        while restored.len() > max_events {
            restored.pop_front();
            let old_key = keys.remove(0);
            if let Err(e) = engine.delete_record(RecordKind::Audit, &old_key) {
                warn!(err = %e, "Eski denetim kaydı silinemedi");
            }
        }

        Self {
            events: RwLock::new(restored),
            max_events,
            persistence: Some(engine),
        }
    }

    pub fn record(&self, agent_id: Uuid, execution_id: Uuid, kind: AuditEventKind) {
        let event = AuditEvent {
            id: Uuid::new_v4(),
            agent_id,
            execution_id,
            kind: kind.bounded(),
            timestamp: Utc::now(),
        };

        if let Some(engine) = &self.persistence {
            match serde_json::to_vec(&event) {
                Ok(bytes) => {
                    if let Err(e) = engine.put_record(RecordKind::Audit, &audit_key(&event), &bytes)
                    {
                        error!(err = %e, "Denetim olayı diske yazılamadı");
                    }
                }
                Err(e) => error!(err = %e, "Denetim olayı serileştirilemedi"),
            }
        }

        let mut events = self.events.write().unwrap();
        if events.len() >= self.max_events
            && let Some(old) = events.pop_front()
            && let Some(engine) = &self.persistence
        {
            let _ = engine.delete_record(RecordKind::Audit, &audit_key(&old));
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

        log.record(agent_id, execution_id, AuditEventKind::ExecutionCompleted);

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
        log.record(
            agent_id,
            exec_a,
            AuditEventKind::ExecutionFailed {
                error: "x".to_string(),
            },
        );

        assert_eq!(log.list_for_execution(exec_a).len(), 2);
        assert_eq!(log.list_for_execution(exec_b).len(), 1);
    }

    #[test]
    fn oldest_event_dropped_when_over_capacity() {
        let log = AuditLog::new(2);
        let agent_id = Uuid::new_v4();
        let execution_id = Uuid::new_v4();

        log.record(
            agent_id,
            execution_id,
            AuditEventKind::ExecutionPaused {
                approval_id: Uuid::new_v4(),
                tool_name: "first".to_string(),
                reason: "r".to_string(),
                arguments: vec![],
            },
        );
        log.record(
            agent_id,
            execution_id,
            AuditEventKind::ExecutionResumed {
                approval_id: Uuid::new_v4(),
            },
        );
        log.record(agent_id, execution_id, AuditEventKind::ExecutionCompleted);

        let events = log.list();
        assert_eq!(events.len(), 2, "kapasite aşılınca en eski düşmeli");
        // İlk olay ("first" ile ilgili ExecutionPaused) artık yok.
        assert!(!events.iter().any(|e| matches!(
            &e.kind,
            AuditEventKind::ExecutionPaused { tool_name, .. } if tool_name == "first"
        )));
    }

    // ── B2: argüman + çıktı özeti ───────────────────────────

    fn v(a: &[&str]) -> Vec<String> {
        a.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn sanitize_keeps_ordinary_arguments_verbatim() {
        let out = sanitize_arguments(&v(&["git", "commit", "-m", "mesaj"]));
        assert_eq!(out, v(&["git", "commit", "-m", "mesaj"]));
    }

    #[test]
    fn sanitize_redacts_secret_looking_values() {
        let out = sanitize_arguments(&v(&[
            "curl",
            "-H",
            "Authorization: Bearer abc",
            "x?token=123",
        ]));
        assert_eq!(out[0], "curl");
        assert_eq!(out[2], REDACTED);
        assert_eq!(out[3], REDACTED);
        assert!(!out.join(" ").contains("abc"));
        assert!(!out.join(" ").contains("123"));
    }

    #[test]
    fn sanitize_redacts_value_after_secret_flag() {
        let out = sanitize_arguments(&v(&["tool", "--password", "hunter2", "--verbose"]));
        assert_eq!(out, v(&["tool", "--password", REDACTED, "--verbose"]));
    }

    #[test]
    fn sanitize_caps_count_and_length() {
        let many: Vec<String> = (0..40).map(|i| format!("a{i}")).collect();
        let out = sanitize_arguments(&many);
        assert_eq!(out.len(), MAX_AUDIT_ARGS + 1, "32 argüman + 1 özet satırı");
        assert!(out.last().unwrap().contains("+8"));

        let long = "x".repeat(MAX_AUDIT_ARG_CHARS + 50);
        let out = sanitize_arguments(&[long]);
        assert_eq!(
            out[0].chars().count(),
            MAX_AUDIT_ARG_CHARS + 1,
            "kırpıldı + '…'"
        );
        assert!(out[0].ends_with('…'));
    }

    #[test]
    fn clip_is_utf8_safe() {
        assert_eq!(clip("çğşıöü", 3), "çğş…");
        assert_eq!(clip("çğş", 3), "çğş");
    }

    #[test]
    fn summarize_output_known_sha256_and_size() {
        let s = summarize_output("abc");
        assert_eq!(s.bytes, 3);
        assert_eq!(
            s.sha256,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(s.preview, "abc");
    }

    #[test]
    fn summarize_output_flattens_newlines_and_redacts_secret_preview() {
        assert_eq!(summarize_output("a\nb").preview, "a b");
        let s = summarize_output("API_KEY=abc\nrest");
        // "api_key=" deseni → önizleme maskelenir, boyut/hash yine doğru
        assert_eq!(s.preview, REDACTED);
        assert_eq!(s.bytes, 16);
    }

    #[test]
    fn record_sanitizes_arguments_and_clips_long_errors() {
        let log = AuditLog::new(10);
        let (a, e) = (Uuid::new_v4(), Uuid::new_v4());
        log.record(
            a,
            e,
            AuditEventKind::ToolInvoked {
                tool_name: "terminal".into(),
                success: false,
                error: Some("E".repeat(MAX_AUDIT_TEXT_CHARS + 500)),
                arguments: v(&["sh", "--token", "s3cret"]),
                output: None,
            },
        );
        match &log.list()[0].kind {
            AuditEventKind::ToolInvoked {
                error, arguments, ..
            } => {
                assert_eq!(arguments, &v(&["sh", "--token", REDACTED]));
                assert_eq!(
                    error.as_ref().unwrap().chars().count(),
                    MAX_AUDIT_TEXT_CHARS + 1
                );
            }
            other => panic!("beklenmeyen olay: {other:?}"),
        }
    }

    #[test]
    fn old_json_without_new_fields_still_deserializes() {
        // Kalıcılık (B3) geldiğinde diskte eski biçimli kayıtlar olacak.
        let old = r#"{"GovernorDecision":{"tool_name":"t","decision":"allow","reason":null}}"#;
        let k: AuditEventKind = serde_json::from_str(old).expect("eski biçim okunmalı");
        assert!(matches!(
            k,
            AuditEventKind::GovernorDecision { ref arguments, .. } if arguments.is_empty()
        ));
    }

    // ── B3: kalıcılık ───────────────────────────────────────

    fn engine_at(path: &std::path::Path) -> Arc<PersistenceEngine> {
        Arc::new(PersistenceEngine::open(path.to_str().unwrap()).expect("db açılmalı"))
    }

    fn pause_ms() {
        // Mikrosaniye-anahtar sırasının test boyunca belirgin olması için.
        std::thread::sleep(std::time::Duration::from_millis(3));
    }

    #[test]
    fn audit_events_survive_reopen_in_order() {
        let dir = tempfile::tempdir().unwrap();
        let (a, e) = (Uuid::new_v4(), Uuid::new_v4());
        {
            let log = AuditLog::with_persistence(engine_at(dir.path()), 100);
            log.record(
                a,
                e,
                AuditEventKind::ExecutionFailed {
                    error: "ilk".into(),
                },
            );
            pause_ms();
            log.record(a, e, AuditEventKind::ExecutionCompleted);
            pause_ms();
            log.record(
                a,
                e,
                AuditEventKind::ToolInvoked {
                    tool_name: "terminal".into(),
                    success: true,
                    error: None,
                    arguments: v(&["git", "status"]),
                    output: Some(summarize_output("abc")),
                },
            );
        }
        let log = AuditLog::with_persistence(engine_at(dir.path()), 100);
        let events = log.list();
        assert_eq!(events.len(), 3);
        assert!(
            matches!(&events[0].kind, AuditEventKind::ExecutionFailed { error } if error == "ilk")
        );
        assert_eq!(events[1].kind, AuditEventKind::ExecutionCompleted);
        match &events[2].kind {
            AuditEventKind::ToolInvoked {
                arguments, output, ..
            } => {
                assert_eq!(arguments, &v(&["git", "status"]));
                assert_eq!(output.as_ref().unwrap().bytes, 3);
            }
            other => panic!("beklenmeyen olay: {other:?}"),
        }
    }

    #[test]
    fn persisted_audit_respects_max_events_across_restarts() {
        let dir = tempfile::tempdir().unwrap();
        let (a, e) = (Uuid::new_v4(), Uuid::new_v4());
        {
            let log = AuditLog::with_persistence(engine_at(dir.path()), 2);
            for i in 0..3 {
                log.record(
                    a,
                    e,
                    AuditEventKind::ExecutionFailed {
                        error: format!("olay{i}"),
                    },
                );
                pause_ms();
            }
            assert_eq!(log.count(), 2, "bellekte sınır uygulanmalı");
        }
        let log = AuditLog::with_persistence(engine_at(dir.path()), 2);
        let events = log.list();
        assert_eq!(events.len(), 2, "diskte de yalnız son 2 kalmalı");
        assert!(
            matches!(&events[0].kind, AuditEventKind::ExecutionFailed { error } if error == "olay1")
        );
        assert!(
            matches!(&events[1].kind, AuditEventKind::ExecutionFailed { error } if error == "olay2")
        );
    }
}
