// ============================================================
// src/bridge/api.rs
//
// Flutter'ın doğrudan çağırdığı Rust fonksiyonları.
//
// flutter_rust_bridge_codegen bu dosyayı okur ve
// flutter_app/lib/src/rust/api/aetheros.dart dosyasını üretir.
//
// KURALLAR:
//   - pub async fn → Dart'ta Future<T> olur
//   - pub fn       → Dart'ta T olur (sync)
//   - Result<T, String> → Dart'ta throws String
//   - anyhow::Result<T> → Dart'ta throws AetherError
//
// ÖMÜR:
//   init_runtime() → [kullan] → shutdown_runtime()
//   init_runtime() çağrılmadan diğerleri RuntimeNotInitialized döner.
// ============================================================

use flutter_rust_bridge::frb;
use tracing::info;

use crate::bridge::state::{get_runtime, init_mobile_runtime};
use crate::bridge::types::{
    AgentStartResponse, AgentStatusResponse,
    ClusterNodeResponse, ClusterStatusResponse,
    LogRecord, MetricsSnapshot, ModuleUploadResponse,
    AuditEventResponse, NodeRegistrationResponse, PendingApprovalResponse,
    RuntimeInfo, TaskRequest, TaskStatusResponse,
    WorkflowStartResponse, WorkflowStatusResponse,
};
use crate::api::rest::router::WorkflowStepRequest;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{TaskDefinition, TaskMetadata, TaskState};
use crate::types::ids::TaskId;

// ── FRB init ─────────────────────────────────────────────
// FRB v2 zorunlu: uygulamanın en başında çağrılır.
// Platform kanallarını, panic handler'ı ve log entegrasyonunu
// kurar.

#[frb(init)]
pub fn init_app() {
    // FRB 2.x: setup_default_user_code_handler kaldırıldı,
    // #[frb(init)] attribute'u zaten gerekli altyapıyı kurar.

    // Android: tracing → logcat
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("AetherOS"),
        );
    }

    // Diğer platformlar: stdout
    #[cfg(not(target_os = "android"))]
    {
        let _ = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .try_init();
    }
}

// ── Runtime yaşam döngüsü ─────────────────────────────────

/// AetherOS runtime'ı başlat.
///
/// Flutter uygulaması açılırken (main() veya splash screen'de)
/// bir kez çağrılır. İkinci çağrı hata döner.
///
/// `db_path`: SQLite/sled veritabanı konumu
///   Android: `/data/data/<package>/databases/aetheros.db`
///   iOS:     `<documents>/aetheros.db`
///
/// `worker_count`: paralel WASM worker sayısı
///   Mobil için 2-4 önerilir.
pub async fn initialize_runtime(
    db_path: String,
    worker_count: u32,
) -> Result<(), String> {
    info!(db = %db_path, workers = worker_count, "initialize_runtime çağrıldı");

    // FRB kendi Tokio runtime'ını çalıştırıyor.
    // init_mobile_runtime() içinde tokio.block_on() var — aynı thread'de
    // çağrılırsa "Cannot start a runtime from within a runtime" paniği olur.
    //
    // Çözüm: spawn_blocking → blocking thread pool'da çalıştır.
    // Bu thread'lerde aktif Tokio context YOK, dolayısıyla block_on güvenli.
    let r = tokio::task::spawn_blocking(move || {
        init_mobile_runtime(db_path, worker_count as usize)
    })
    .await
    .map_err(|e| format!("Thread başlatma hatası: {e}"))?;

    r
}

/// Runtime'ın çalışıp çalışmadığını kontrol et.
pub fn is_runtime_ready() -> bool {
    get_runtime().is_some()
}

/// Runtime bilgilerini al (versiyon, backend, worker sayısı).
pub fn get_runtime_info() -> RuntimeInfo {
    let backend = if cfg!(feature = "backend-wasmi") {
        "wasmi".to_string()
    } else {
        "wasmtime".to_string()
    };

    RuntimeInfo {
        version:    env!("CARGO_PKG_VERSION").to_string(),
        is_running: get_runtime().is_some(),
        backend,
        worker_count: get_runtime().map(|_| 2u32).unwrap_or(0),
    }
}

// ── Task yönetimi ─────────────────────────────────────────

/// Task gönder → task_id döner.
///
/// WASM modülü yoksa (wasm_module_hash == "") task yine kabul
/// edilir; runtime Failed olarak işaretler.
///
/// Örnek (Dart):
/// ```dart
/// final taskId = await AetherApi.submitTask(TaskRequest(
///   wasmModuleHash: hash,
///   entrypoint: "run",
///   priority: "normal",
///   timeoutMs: 30000,
///   maxRetries: 3,
/// ));
/// ```
pub async fn submit_task(
    request: TaskRequest,
) -> Result<String, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    // priority string → TaskPriority
    let priority = parse_priority(&request.priority)?;

    // wasm_module_hash: hex → [u8;32]
    let wasm_module_hash = parse_hash(&request.wasm_module_hash)?;

    let timeout_ms = if request.timeout_ms == 0 {
        30_000
    } else {
        request.timeout_ms
    };

    let task_id = TaskId(uuid::Uuid::new_v4());

    let now = chrono::Utc::now();

    let task = TaskDefinition {
        id: task_id.clone(),
        parent: None,
        orchestration: None,
        priority,
        deadline: None,
        timeout_ms,
        retry_policy: RetryPolicy {
            max_attempts: request.max_retries.max(1),
            base_delay_ms: 100,
            max_delay_ms: 10_000,
            jitter: true,
        },
        metadata: TaskMetadata {
            labels: std::collections::HashMap::new(),
        },
        wasm_module_hash,
        entrypoint: request.entrypoint,
        state: TaskState::Created,
        created_at: now,
        updated_at: now,
    };

    rt.handle
        .submit(task)
        .await
        .map_err(|e| format!("Submit hatası: {e:?}"))?;

    info!(task_id = %task_id.0, "Task gönderildi");

    Ok(task_id.0.to_string())
}

/// Task durumunu sorgula.
pub async fn get_task_status(
    task_id: String,
) -> Result<TaskStatusResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let uuid = uuid::Uuid::parse_str(&task_id)
        .map_err(|_| format!("Geçersiz task_id: {task_id}"))?;

    let tid = TaskId(uuid);

    let persisted = rt
        .persistence
        .load_task(&tid)
        .map_err(|e| format!("Persistence hatası: {e:?}"))?
        .ok_or_else(|| format!("Task bulunamadı: {task_id}"))?;

    let state_str = format!("{:?}", persisted.task.state);

    // Gerçek hata mesajı persistence'tan (worker.fail_task ile
    // kaydedildi). Eski kayıtlarda last_error=None olabilir —
    // bu durumda genel bir mesaja düş.
    let error_message = match &persisted.task.state {
        crate::task::task::TaskState::Failed => Some(
            persisted
                .last_error
                .clone()
                .unwrap_or_else(|| "WASM yürütme başarısız (detay yok)".to_string()),
        ),
        _ => None,
    };

    Ok(TaskStatusResponse {
        task_id: task_id,
        state: state_str,
        created_at: persisted.created_at.timestamp_millis(),
        updated_at: persisted.updated_at.timestamp_millis(),
        attempts: persisted.attempts,
        error_message,
    })
}

/// Tüm task'ları listele (son N tane).
pub async fn list_tasks(
    limit: u32,
) -> Result<Vec<TaskStatusResponse>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let all = match rt.persistence.load_all_tasks() {
        Ok(tasks) => tasks,
        Err(e) => {
            // Eski format kayıtlar MessagePack deserialize edemeyebilir.
            // Yutmak yerine logla — UI'da boş liste yerine hata göster.
            tracing::warn!(err = ?e, "load_all_tasks kısmen başarısız");
            return Err(format!("Task listesi yüklenemedi: {e:?}"));
        }
    };

    let responses: Vec<TaskStatusResponse> = all
        .into_iter()
        .rev()                              // En yeni önce
        .take(limit as usize)
        .map(|p| {
            let state_str = format!("{:?}", p.task.state);
            let error_message = match &p.task.state {
                crate::task::task::TaskState::Failed => Some(
                    p.last_error
                        .clone()
                        .unwrap_or_else(|| "WASM yürütme başarısız (detay yok)".to_string()),
                ),
                _ => None,
            };
            TaskStatusResponse {
                task_id:       p.task.id.0.to_string(),
                state:         state_str,
                created_at:    p.created_at.timestamp_millis(),
                updated_at:    p.updated_at.timestamp_millis(),
                attempts:      p.attempts,
                error_message,
            }
        })
        .collect();

    Ok(responses)
}

// ── WASM modül yönetimi ───────────────────────────────────

/// WASM modülü yükle → hash döner.
///
/// Flutter, dosyayı bytes olarak Rust'a verir.
/// Rust, ModuleStore'a kaydeder ve SHA-256 hash döner.
/// Sonraki task'larda bu hash kullanılır.
///
/// Örnek (Dart):
/// ```dart
/// final bytes = await File("my_module.wasm").readAsBytes();
/// final hash = await AetherApi.uploadWasmModule(bytes: bytes);
/// ```

/// WAT (WebAssembly Text Format) kaynak kodunu WASM binary'ye derle
/// ve ModuleStore'a kaydet.
///
/// Flutter script editor'ün "Derle & Yükle" butonu bu fonksiyonu çağırır.
/// Derleme Rust tarafında `wat::parse_str()` ile yapılır.
///
/// Başarı: ModuleUploadResponse { hash, size } döner.
/// Hata:  WAT sözdizimi hatası string olarak döner.
pub async fn compile_wat_to_wasm(
    name:         String,
    wat_source:   String,
    entrypoint:   String,
    _timeout_ms:  u64,   // Gelecekte execution timeout için — şimdilik WAT compile'da kullanılmıyor
) -> Result<ModuleUploadResponse, String> {
    // WAT → WASM binary (Rust tarafında, zero-dependency)
    let wasm_binary = wat::parse_str(&wat_source)
        .map_err(|e| format!("WAT derleme hatası: {e}"))?;

    info!(
        name       = %name,
        entrypoint = %entrypoint,
        bytes      = wasm_binary.len(),
        "WAT başarıyla WASM'a derlendi"
    );

    // Artık binary olarak upload_wasm_module'ün yaptığını tekrar et
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let size = wasm_binary.len() as u64;

    let hash_bytes = rt
        .module_store
        .store(wasm_binary.clone())
        .map_err(|e| format!("Modül kaydedilemedi: {e}"))?;

    let hash = crate::wasm::module_store::ModuleStore::hash_to_hex(&hash_bytes);

    // Diske kaydet
    let file_path = format!("{}/{}.wasm", rt.modules_dir, hash);
    if let Err(e) = std::fs::write(&file_path, &wasm_binary) {
        tracing::warn!(path = %file_path, err = %e, "WASM diske yazılamadı");
    } else {
        info!(hash = %hash, path = %file_path, "WAT→WASM derlendi ve diske kaydedildi");
    }

    Ok(ModuleUploadResponse { hash, size })
}

pub async fn upload_wasm_module(
    bytes: Vec<u8>,
) -> Result<ModuleUploadResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let size = bytes.len() as u64;

    // ModuleStore'a kaydet → SHA-256 hash al (idempotent)
    let hash_bytes = rt
        .module_store
        .store(bytes.clone())
        .map_err(|e| format!("Modül kaydedilemedi: {e}"))?;

    let hash = crate::wasm::module_store::ModuleStore::hash_to_hex(&hash_bytes);

    // Diske kaydet → restart'ta otomatik restore edilir
    // {docDir}/modules/{hash}.wasm
    let file_path = format!("{}/{}.wasm", rt.modules_dir, hash);
    if let Err(e) = std::fs::write(&file_path, &bytes) {
        // Disk yazma hatası — in-memory yükleme başarılı, sadece persist olmayacak.
        // Uyarı log'la, hata döndürme (modül bu session'da çalışır).
        tracing::warn!(
            path = %file_path,
            err  = %e,
            "WASM modülü diske yazılamadı (session'da çalışır ama restart'ta kaybolur)"
        );
    } else {
        info!(hash = %hash, path = %file_path, "WASM modülü diske kaydedildi");
    }

    info!(hash = %hash, size = size, "WASM modülü yüklendi");

    Ok(ModuleUploadResponse { hash, size })
}

/// Belirtilen hash'e sahip modül runtime'da kayıtlı mı?
///
/// WasmModuleScreen açılışında, eski oturumdan kalan
/// meta-data'yı doğrulamak için her modül için çağrılır.
pub fn check_module_exists(hash_hex: String) -> Result<bool, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let hash = crate::wasm::module_store::ModuleStore::hex_to_hash(&hash_hex)
        .map_err(|e| format!("Geçersiz hash: {e}"))?;

    Ok(rt.module_store.contains(&hash))
}

// ── V10 Sprint 1b: Agent Tool Script Registry ──────────────
//
// Akış: önce upload_wasm_module() / compile_wat_to_wasm() ile binary
// yüklenir (hash döner), sonra bu hash burada isimlendirilerek agent
// tool'u olarak kaydedilir. start_agent() her çağrıda script_registry'
// deki TÜM script'leri agent'ın araç kutusuna (tools) ekler — hangi
// script'lerin çalışabileceğini CapabilityEngine (WasmExecution grant'ı)
// belirler, registry'de olmak tek başına yeterli değildir.

/// Daha önce upload edilmiş bir WASM modülünü, agent'ların
/// çağırabileceği isimlendirilmiş bir tool olarak kaydet.
pub fn register_agent_script(
    name: String,
    wasm_module_hash_hex: String,
    entrypoint: String,
    timeout_ms: u64,
) -> Result<(), String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let hash = crate::wasm::module_store::ModuleStore::hex_to_hash(&wasm_module_hash_hex)
        .map_err(|e| format!("Geçersiz hash: {e}"))?;

    let binary = rt
        .module_store
        .get(&hash)
        .map_err(|e| format!("Modül bulunamadı: {e}"))?;

    let script = crate::scripting::definition::ScriptDefinition::from_binary(
        name.clone(),
        (*binary).clone(),
        entrypoint,
        timeout_ms,
    );

    rt.script_registry.register(script);

    info!(name = %name, hash = %wasm_module_hash_hex, "Agent script kaydedildi");
    Ok(())
}

/// Şu an kayıtlı, agent'ların potansiyel olarak erişebileceği
/// script isimlerini listele (UI'da göstermek için).
pub fn list_agent_scripts() -> Result<Vec<String>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    Ok(rt
        .script_registry
        .list()
        .into_iter()
        .map(|s| s.name)
        .collect())
}

/// Capability adını (Flutter/FRB'den gelen string) enum'a çevirir.
/// `grant_agent_capability` ve `start_agent` aynı tabloyu kullanır.
pub(crate) fn parse_capability(
    name: &str,
) -> Result<crate::agents::capabilities::AgentCapability, String> {
    use crate::agents::capabilities::AgentCapability as C;
    match name {
        "wasm_execution"     => Ok(C::WasmExecution),
        "workflow_execution" => Ok(C::WorkflowExecution),
        "ai_reasoning"       => Ok(C::AiReasoning),
        "remote_execution"   => Ok(C::RemoteExecution),
        "terminal_execution" => Ok(C::TerminalExecution),
        other => Err(format!("Bilinmeyen capability: '{other}'")),
    }
}

/// Bir agent'a tek bir capability grant et.
///
/// `capability`: "wasm_execution" | "workflow_execution" | "ai_reasoning"
///               | "remote_execution" | "terminal_execution"
///
/// TERCİH EDİLEN YOL: `start_agent(..., capabilities)` — yetkiler agent
/// çalışmaya başlamadan ÖNCE, atomik verilir (yarış yok). Bu fonksiyon,
/// çalışan bir agent'a SONRADAN yetki eklemek içindir ve start_agent'ın
/// döndüğü agent_id ile, execution ilk tool adımına gelmeden önce
/// çağrılmazsa yarışa açıktır.
/// Bu yarış durumu kabul edilebilir (planning genelde tool adımından
/// önce bir miktar zaman alır) ama sağlam bir çözüm değil — gerçek
/// çözüm Approval Engine'in agent'ı "grant bekliyor" durumunda
/// başlatıp ilk adımdan önce durdurmasıdır (gelecek sprint).
pub fn grant_agent_capability(
    agent_id_hex: String,
    capability: String,
) -> Result<(), String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let agent_id = uuid::Uuid::parse_str(&agent_id_hex)
        .map_err(|e| format!("Geçersiz agent_id: {e}"))?;

    let cap = parse_capability(capability.as_str())?;

    rt.capability_engine.grant_capability(agent_id, cap);

    info!(agent_id = %agent_id, capability = %capability, "Agent capability grant edildi");
    Ok(())
}

// ── Task yeniden gönderme ──────────────────────────────────

/// Mevcut bir task'ı orijinal ayarlarıyla (hash, entrypoint,
/// priority, retry policy) yeni bir UUID altında yeniden kuyruğa ekler.
///
/// "Yeniden Dene" butonu için — sadece id ve zaman damgaları
/// yenilenir, deadline sıfırlanır.
pub async fn resubmit_task(task_id: String) -> Result<String, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let uuid = uuid::Uuid::parse_str(&task_id)
        .map_err(|_| format!("Geçersiz task_id: {task_id}"))?;

    let original = rt
        .persistence
        .load_task(&TaskId(uuid))
        .map_err(|e| format!("Persistence hatası: {e:?}"))?
        .ok_or_else(|| format!("Task bulunamadı: {task_id}"))?
        .task;

    let new_id = TaskId(uuid::Uuid::new_v4());
    let now = chrono::Utc::now();

    let resubmitted = TaskDefinition {
        id: new_id.clone(),
        parent: original.parent,
        orchestration: original.orchestration,
        priority: original.priority,
        deadline: None,
        timeout_ms: original.timeout_ms,
        retry_policy: original.retry_policy,
        metadata: original.metadata,
        wasm_module_hash: original.wasm_module_hash,
        entrypoint: original.entrypoint,
        state: TaskState::Created,
        created_at: now,
        updated_at: now,
    };

    rt.handle
        .submit(resubmitted)
        .await
        .map_err(|e| format!("Submit hatası: {e:?}"))?;

    info!(old_task_id = %task_id, new_task_id = %new_id.0, "Task yeniden gönderildi");

    Ok(new_id.0.to_string())
}

// ── Log izleme ───────────────────────────────────────────

/// Son `limit` kadar log entry döndür (yeniden eskiye sıralı).
///
/// LogScreen 2 saniyede bir bu fonksiyonu polling ile çeker.
/// limit: 0 → varsayılan 100.
pub fn get_recent_logs(limit: u32) -> Result<Vec<LogRecord>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let limit = if limit == 0 { 100 } else { limit as usize };

    Ok(rt
        .log_buffer
        .recent(limit)
        .into_iter()
        .map(to_bridge_log_entry)
        .collect())
}

/// Belirli bir task'a ait log entry'leri döndür.
///
/// Task detay modalındaki "Loglar" sekmesi için.
/// limit: 0 → varsayılan 50.
pub fn get_task_logs(task_id: String, limit: u32) -> Result<Vec<LogRecord>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let limit = if limit == 0 { 50 } else { limit as usize };

    Ok(rt
        .log_buffer
        .by_task(&task_id, limit)
        .into_iter()
        .map(to_bridge_log_entry)
        .collect())
}

/// logging::buffer::LogEntry (pub(crate), &'static str alanlı) →
/// bridge::types::LogRecord (pub, String alanlı, FRB-export edilebilir).
fn to_bridge_log_entry(e: crate::logging::buffer::LogEntry) -> LogRecord {
    LogRecord {
        timestamp_ms: e.timestamp_ms,
        level: e.level.to_string(),
        task_id: e.task_id,
        message: e.message,
        event_type: e.event_type.to_string(),
    }
}

// ── Metrikler ─────────────────────────────────────────────

/// Anlık metrik görüntüsü al.
pub async fn get_metrics() -> Result<MetricsSnapshot, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let snap = rt.metrics.snapshot();

    Ok(MetricsSnapshot {
        active_workers:  snap.active_workers,
        queued_tasks:    snap.queued_tasks,
        completed_tasks: snap.completed_tasks,
        failed_tasks:    snap.failed_tasks,
        retried_tasks:   snap.retried_tasks,
    })
}

// ── Yardımcı fonksiyonlar ─────────────────────────────────

fn parse_priority(s: &str) -> Result<TaskPriority, String> {
    match s.to_lowercase().as_str() {
        "critical" => Ok(TaskPriority::Critical),
        "high"     => Ok(TaskPriority::High),
        "normal"   => Ok(TaskPriority::Normal),
        "low"      => Ok(TaskPriority::Low),
        other      => Err(format!("Geçersiz priority: '{other}'")),
    }
}

fn parse_hash(hex_str: &str) -> Result<[u8; 32], String> {
    if hex_str.is_empty() {
        return Ok([0u8; 32]);  // Modülsüz task
    }

    let bytes = hex::decode(hex_str)
        .map_err(|_| format!("Geçersiz hex hash: '{hex_str}'"))?;

    if bytes.len() != 32 {
        return Err(format!(
            "Hash 32 byte olmalı, {} byte geldi",
            bytes.len()
        ));
    }

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}


// ── Agent fonksiyonları (FRB) ─────────────────────────────────

/// Agent başlat → execution_id döner.
///
/// `capabilities`: agent'a BAŞLANGIÇTA verilecek yetkiler
/// ("wasm_execution" | "workflow_execution" | "ai_reasoning" |
/// "remote_execution" | "terminal_execution"). Boş liste = hiç yetki
/// (capability gerektiren hiçbir tool çalışmaz). Yetkiler agent
/// spawn edilmeden ÖNCE verilir — `grant_agent_capability` ile sonradan
/// vermenin yarışı yoktur. Bilinmeyen bir ad varsa HİÇBİR şey
/// başlatılmaz/verilmez (kısmi durum bırakmaz).
pub async fn start_agent(
    objective:    String,
    max_steps:    usize,
    max_tokens:   usize,
    capabilities: Vec<String>,
) -> Result<AgentStartResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    // Önce HEPSİNİ doğrula (kısmi grant yok).
    let mut caps = Vec::new();
    for name in &capabilities {
        let cap = parse_capability(name)?;
        if !caps.contains(&cap) {
            caps.push(cap);
        }
    }

    let execution_id = uuid::Uuid::new_v4();
    let agent_id     = uuid::Uuid::new_v4();

    // Yetkiler, agent'ın execution'ı spawn edilmeden ÖNCE (B11).
    for cap in &caps {
        rt.capability_engine.grant_capability(agent_id, *cap);
    }
    if !caps.is_empty() {
        info!(agent_id = %agent_id, capabilities = ?capabilities, "Agent başlangıç yetkileri verildi");
    }
    let started_at   = chrono::Utc::now();
    let objective_c  = objective.clone();

    rt.agent_registry.insert(execution_id, crate::bridge::agent::AgentEntry {
        execution_id,
        agent_id,
        objective:   objective.clone(),
        status:      "running".into(),
        error:       None,
        started_at,
        finished_at: None,
        pending_approval_id: None,
    });

    let registry = rt.agent_registry.clone();
    let context  = crate::agents::context::AgentContext {
        agent_id, execution_id, workflow_id: None,
    };
    let budget = crate::agents::budget::AgentExecutionBudget {
        max_tokens, max_steps, max_runtime_seconds: 300,
    };
    let ai_router = rt.ai_router.clone();
    let capability_engine = rt.capability_engine.clone();
    let risk_engine = rt.risk_engine.clone();
    let approval_store = rt.approval_store.clone();
    let audit_log = rt.audit_log.clone();

    // V10 Sprint 1b: agent'ın araç kutusu artık boş değil — registry'de
    // kayıtlı her script bir ScriptTool olarak agent'a sunuluyor.
    // Hangisinin GERÇEKTEN çalışabileceğine CapabilityEngine karar verir
    // (bkz. grant_agent_capability) — burada listelenmek izin vermez.
    let mut tools: Vec<std::sync::Arc<dyn crate::types::agent_tool::AgentTool>> = rt
        .script_registry
        .list()
        .into_iter()
        .map(|def| {
            std::sync::Arc::new(crate::scripting::tool::ScriptTool::new(
                def,
                rt.script_engine.clone(),
            )) as std::sync::Arc<dyn crate::types::agent_tool::AgentTool>
        })
        .collect();
    // V10 Faz 1: terminal her agent'a sunuluyor — listede olmak izin
    // vermez, TerminalExecution capability + Governor onayı hâlâ şart.
    tools.push(std::sync::Arc::new(
        crate::agents::terminal_tool::TerminalAgentTool::new(),
    ));

    tokio::spawn(async move {
        let result      = crate::agents::executor::AgentExecutor
            ::execute(context, objective_c, budget, tools, Some(ai_router), Some(capability_engine), Some(risk_engine), Some(approval_store), Some(audit_log)).await;
        let finished_at = chrono::Utc::now();
        if let Some(mut entry) = registry.get_mut(&execution_id) {
            match result {
                Ok(crate::agents::runtime::AgentOutcome::Completed) => {
                    entry.status = "completed".into();
                    entry.finished_at = Some(finished_at);
                }
                Ok(crate::agents::runtime::AgentOutcome::PendingApproval { approval_id }) => {
                    // V10 Sprint 5: execution bitmedi, DURAKLADI — finished_at
                    // BİLEREK set edilmiyor, respond_to_approval() devam
                    // ettirene ya da kalıcı reddedene kadar "bitmemiş" sayılır.
                    entry.status = "pending_approval".into();
                    entry.pending_approval_id = Some(approval_id);
                }
                Err(e) => { entry.status = "failed".into(); entry.error = Some(format!("{e:?}")); entry.finished_at = Some(finished_at); }
            }
        }
    });

    Ok(AgentStartResponse {
        execution_id: execution_id.to_string(),
        agent_id:     agent_id.to_string(),
        status:       "running".into(),
    })
}

/// Agent execution durumunu sorgula.
pub fn get_agent_status(execution_id: String) -> Result<AgentStatusResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let uuid = uuid::Uuid::parse_str(&execution_id)
        .map_err(|_| format!("Geçersiz execution_id: {execution_id}"))?;

    rt.agent_registry.get(&uuid)
        .map(|e| AgentStatusResponse {
            execution_id: e.execution_id.to_string(),
            agent_id:     e.agent_id.to_string(),
            objective:    e.objective.clone(),
            status:       e.status.clone(),
            error:        e.error.clone(),
            started_at:   e.started_at.timestamp_millis(),
            finished_at:  e.finished_at.map(|t| t.timestamp_millis()),
            pending_approval_id: e.pending_approval_id.map(|id| id.to_string()),
        })
        .ok_or_else(|| format!("Agent bulunamadı: {execution_id}"))
}

// ── V10 Sprint 5: Approval Engine ──────────────────────────
//
// SecurityGovernor bir ToolCall için RequiresApproval dediğinde,
// AgentRuntime execution'ı invoke etmeden DURDURUP bir PendingApproval
// kaydı bırakır (bkz. agents::approval). Bu iki fonksiyon, "Bekleyen
// Onaylar" ekranının ihtiyaç duyduğu tüm akışı sağlar.

/// Onay bekleyen tüm execution'ları listele.
pub fn list_pending_approvals() -> Result<Vec<PendingApprovalResponse>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    // V10 B3: süresi dolmuş onaylar listede görünmesin (ve verilemesin).
    crate::bridge::state::expire_stale_approvals(
        &rt.approval_store, &rt.audit_log, &rt.agent_registry,
    );

    Ok(rt
        .approval_store
        .list()
        .into_iter()
        .map(|p| PendingApprovalResponse {
            id:           p.id.to_string(),
            execution_id: p.context.execution_id.to_string(),
            agent_id:     p.context.agent_id.to_string(),
            objective:    p.objective,
            tool_name:    p.tool_call.tool_name,
            arguments:    p.tool_call.arguments,
            reason:       p.reason,
            created_at:   p.created_at.timestamp_millis(),
        })
        .collect())
}

/// Bekleyen bir onaya cevap ver.
///
/// `approved = true`  → onaylanan tool_call çalıştırılır, execution
///                       kalan adımlarla arka planda devam eder.
/// `approved = false` → execution kalıcı olarak reddedilmiş sayılır,
///                       hiçbir şey invoke edilmez.
pub async fn respond_to_approval(
    approval_id: String,
    approved: bool,
) -> Result<(), String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let id = uuid::Uuid::parse_str(&approval_id)
        .map_err(|e| format!("Geçersiz approval_id: {e}"))?;

    // V10 B3: TTL'i dolmuş onay verilemez — önce temizle.
    crate::bridge::state::expire_stale_approvals(
        &rt.approval_store, &rt.audit_log, &rt.agent_registry,
    );

    // take(): kaydı çıkarır — aynı onaya iki kere cevap verilemez.
    let pending = rt
        .approval_store
        .take(&id)
        .ok_or_else(|| format!("Onay kaydı bulunamadı (zaten işlenmiş ya da süresi dolmuş olabilir): {approval_id}"))?;

    let execution_id = pending.context.execution_id;

    if !approved {
        rt.audit_log.record(
            pending.context.agent_id,
            execution_id,
            crate::logging::audit::AuditEventKind::ApprovalDenied {
                approval_id: id,
                reason: pending.reason.clone(),
            },
        );
        if let Some(mut entry) = rt.agent_registry.get_mut(&execution_id) {
            entry.status = "denied".into();
            entry.error = Some(format!("Kullanıcı onayı reddetti: {}", pending.reason));
            entry.finished_at = Some(chrono::Utc::now());
            entry.pending_approval_id = None;
        }
        info!(approval_id = %id, execution_id = %execution_id, "Approval denied by user");
        return Ok(());
    }

    if let Some(mut entry) = rt.agent_registry.get_mut(&execution_id) {
        entry.status = "running".into();
        entry.pending_approval_id = None;
    }

    // Resume anında GÜNCEL registry'den taze tool listesi kur —
    // start_agent'taki ile aynı desen (bkz. yukarısı).
    let mut tools: Vec<std::sync::Arc<dyn crate::types::agent_tool::AgentTool>> = rt
        .script_registry
        .list()
        .into_iter()
        .map(|def| {
            std::sync::Arc::new(crate::scripting::tool::ScriptTool::new(
                def,
                rt.script_engine.clone(),
            )) as std::sync::Arc<dyn crate::types::agent_tool::AgentTool>
        })
        .collect();
    tools.push(std::sync::Arc::new(
        crate::agents::terminal_tool::TerminalAgentTool::new(),
    ));

    let ai_router         = rt.ai_router.clone();
    let capability_engine = rt.capability_engine.clone();
    let risk_engine       = rt.risk_engine.clone();
    let approval_store    = rt.approval_store.clone();
    let audit_log         = rt.audit_log.clone();
    let registry          = rt.agent_registry.clone();

    info!(approval_id = %id, execution_id = %execution_id, "Approval granted by user — resuming execution");

    tokio::spawn(async move {
        let result = crate::agents::runtime::AgentRuntime::resume(
            pending,
            tools,
            Some(ai_router),
            Some(capability_engine),
            Some(risk_engine),
            Some(approval_store),
            Some(audit_log),
        )
        .await;
        let finished_at = chrono::Utc::now();
        if let Some(mut entry) = registry.get_mut(&execution_id) {
            match result {
                Ok(crate::agents::runtime::AgentOutcome::Completed) => {
                    entry.status = "completed".into();
                    entry.finished_at = Some(finished_at);
                }
                Ok(crate::agents::runtime::AgentOutcome::PendingApproval { approval_id }) => {
                    entry.status = "pending_approval".into();
                    entry.pending_approval_id = Some(approval_id);
                }
                Err(e) => {
                    entry.status = "failed".into();
                    entry.error = Some(format!("{e:?}"));
                    entry.finished_at = Some(finished_at);
                }
            }
        }
    });

    Ok(())
}

// ── V10 Sprint 6: Audit Log ─────────────────────────────────

/// Özet satırı için argümanları ` [a b c]` biçiminde yazar (boşsa "").
/// Audit'e yazılırken zaten maskelenmiş/kırpılmış olarak gelir.
fn fmt_call_args(arguments: &[String]) -> String {
    if arguments.is_empty() {
        String::new()
    } else {
        format!(" [{}]", arguments.join(" "))
    }
}

/// `AuditEventKind`'ı ("governor_decision" gibi) kısa bir etikete ve
/// tek satırlık okunabilir bir özete çevirir.
fn describe_audit_event(kind: &crate::logging::audit::AuditEventKind) -> (&'static str, String) {
    use crate::logging::audit::AuditEventKind as K;
    match kind {
        K::GovernorDecision { tool_name, decision, reason, arguments } => (
            "governor_decision",
            match reason {
                Some(r) => format!("'{tool_name}'{} → {decision} ({r})", fmt_call_args(arguments)),
                None => format!("'{tool_name}'{} → {decision}", fmt_call_args(arguments)),
            },
        ),
        K::ToolInvoked { tool_name, success, error, arguments, .. } => (
            "tool_invoked",
            if *success {
                format!("'{tool_name}'{} çalıştı", fmt_call_args(arguments))
            } else {
                format!(
                    "'{tool_name}'{} başarısız: {}",
                    fmt_call_args(arguments),
                    error.clone().unwrap_or_default()
                )
            },
        ),
        K::ExecutionPaused { tool_name, reason, arguments, .. } => (
            "execution_paused",
            format!("'{tool_name}'{} onay bekliyor: {reason}", fmt_call_args(arguments)),
        ),
        K::ExecutionResumed { approval_id } => (
            "execution_resumed",
            format!("onay {approval_id} ile devam edildi"),
        ),
        K::ApprovalDenied { reason, .. } => (
            "approval_denied",
            format!("kullanıcı reddetti: {reason}"),
        ),
        K::ExecutionCompleted => ("execution_completed", "tamamlandı".to_string()),
        K::ExecutionFailed { error } => ("execution_failed", format!("hata: {error}")),
    }
}

/// Denetim kayıtlarını listele. `execution_id` verilirse sadece o
/// execution'a ait kayıtlar döner, verilmezse hepsi (en yeni en sonda).
pub fn list_audit_events(execution_id: Option<String>) -> Result<Vec<AuditEventResponse>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let events = match execution_id {
        Some(hex) => {
            let id = uuid::Uuid::parse_str(&hex)
                .map_err(|e| format!("Geçersiz execution_id: {e}"))?;
            rt.audit_log.list_for_execution(id)
        }
        None => rt.audit_log.list(),
    };

    Ok(events
        .into_iter()
        .map(|e| {
            let (kind_label, summary) = describe_audit_event(&e.kind);
            AuditEventResponse {
                id: e.id.to_string(),
                agent_id: e.agent_id.to_string(),
                execution_id: e.execution_id.to_string(),
                kind_label: kind_label.to_string(),
                summary,
                details_json: serde_json::to_string(&e.kind).unwrap_or_default(),
                created_at: e.timestamp.timestamp_millis(),
            }
        })
        .collect())
}

/// Tüm agent execution'larını listele.
pub fn list_agents(limit: usize) -> Result<Vec<AgentStatusResponse>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let mut list: Vec<AgentStatusResponse> = rt.agent_registry
        .iter()
        .take(limit)
        .map(|e| AgentStatusResponse {
            execution_id: e.execution_id.to_string(),
            agent_id:     e.agent_id.to_string(),
            objective:    e.objective.clone(),
            status:       e.status.clone(),
            error:        e.error.clone(),
            started_at:   e.started_at.timestamp_millis(),
            finished_at:  e.finished_at.map(|t| t.timestamp_millis()),
            pending_approval_id: e.pending_approval_id.map(|id| id.to_string()),
        })
        .collect();

    // En yeni önce
    list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(list)
}

// ── Workflow fonksiyonları (FRB) ──────────────────────────────

/// Workflow başlat → workflow_id döner.
pub async fn start_workflow(
    name:  String,
    steps: Vec<WorkflowStepRequest>,
) -> Result<WorkflowStartResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let workflow_id = uuid::Uuid::new_v4();
    let started_at  = chrono::Utc::now();
    let name_c      = name.clone();

    use crate::workflows::compiler::{WorkflowDsl, StepDsl};
    let dsl_steps: Vec<StepDsl> = steps.into_iter().map(|s| StepDsl {
        id:         s.id,
        name:       s.name,
        kind:       s.kind,
        entrypoint: s.entrypoint,
        depends_on: s.depends_on,
        retryable:  s.retryable,
        labels:     Default::default(),
    }).collect();

    let dsl = WorkflowDsl { name: name.clone(), version: Some(1), steps: dsl_steps };

    rt.workflow_registry.insert(workflow_id, crate::bridge::agent::WorkflowEntry {
        workflow_id,
        name: name.clone(),
        status: "running".into(),
        error: None,
        started_at,
        finished_at: None,
    });

    let registry = rt.workflow_registry.clone();
    let runtime  = rt.handle.clone();
    let ai_router = rt.ai_router.clone();

    tokio::spawn(async move {
        let result      = crate::workflows::engine::WorkflowEngine::run_dsl(&dsl, runtime, ai_router).await;
        let finished_at = chrono::Utc::now();
        if let Some(mut entry) = registry.get_mut(&workflow_id) {
            match result {
                Ok(_)  => { entry.status = "completed".into(); entry.finished_at = Some(finished_at); }
                Err(e) => { entry.status = "failed".into(); entry.error = Some(format!("{e:?}")); entry.finished_at = Some(finished_at); }
            }
        }
    });

    Ok(WorkflowStartResponse {
        workflow_id: workflow_id.to_string(),
        name:        name_c,
        status:      "running".into(),
    })
}

/// Workflow durumu sorgula.
pub fn get_workflow_status(workflow_id: String) -> Result<WorkflowStatusResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let uuid = uuid::Uuid::parse_str(&workflow_id)
        .map_err(|_| format!("Geçersiz workflow_id: {workflow_id}"))?;

    rt.workflow_registry.get(&uuid)
        .map(|e| WorkflowStatusResponse {
            workflow_id: e.workflow_id.to_string(),
            name:        e.name.clone(),
            status:      e.status.clone(),
            error:       e.error.clone(),
            started_at:  e.started_at.timestamp_millis(),
            finished_at: e.finished_at.map(|t| t.timestamp_millis()),
        })
        .ok_or_else(|| format!("Workflow bulunamadı: {workflow_id}"))
}

/// Tüm workflow'ları listele.
pub fn list_workflows(limit: usize) -> Result<Vec<WorkflowStatusResponse>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let mut list: Vec<WorkflowStatusResponse> = rt.workflow_registry
        .iter()
        .take(limit)
        .map(|e| WorkflowStatusResponse {
            workflow_id: e.workflow_id.to_string(),
            name:        e.name.clone(),
            status:      e.status.clone(),
            error:       e.error.clone(),
            started_at:  e.started_at.timestamp_millis(),
            finished_at: e.finished_at.map(|t| t.timestamp_millis()),
        })
        .collect();

    list.sort_by(|a, b| b.started_at.cmp(&a.started_at));
    Ok(list)
}

// ── Cluster fonksiyonları (FRB) ───────────────────────────────

/// Cluster genel durumunu getir.
pub fn get_cluster_status() -> Result<ClusterStatusResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let nodes: Vec<ClusterNodeResponse> = rt.cluster.nodes()
        .into_iter()
        .map(|n| {
            let hb = rt.cluster.heartbeat(&n.node_id);
            ClusterNodeResponse {
                node_id:           n.node_id.to_string(),
                address:           n.address.clone(),
                healthy:           n.healthy,
                capabilities:      n.capabilities.iter()
                    .map(|c| format!("{:?}", c))
                    .collect(),
                cpu_percent:       hb.as_ref().map(|h| h.cpu_usage_percent).unwrap_or(0.0),
                memory_mb:         hb.as_ref().map(|h| h.memory_usage_mb as u64).unwrap_or(0),
                active_executions: hb.as_ref().map(|h| h.active_executions as u64).unwrap_or(0),
            }
        })
        .collect();

    Ok(ClusterStatusResponse {
        health:     format!("{:?}", rt.cluster.health()),
        total:      rt.cluster.size() as u64,
        healthy:    rt.cluster.healthy_count() as u64,
        has_quorum: rt.cluster.has_quorum(),
        leader:     rt.cluster.leader().map(|u| u.to_string()),
        nodes,
    })
}

/// Cluster'a yeni node kaydet.
pub fn register_node(
    address:      String,
    capabilities: Vec<String>,
) -> Result<NodeRegistrationResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    use crate::remote::node::{NodeCapability, RemoteNode};

    let caps: Vec<NodeCapability> = capabilities.iter()
        .map(|c| match c.to_lowercase().as_str() {
            "wasm"     => NodeCapability::WasmExecution,
            "workflow" => NodeCapability::WorkflowExecution,
            "agent"    => NodeCapability::AgentExecution,
            "ai"       => NodeCapability::AiInference,
            "plugin"   => NodeCapability::PluginExecution,
            _          => NodeCapability::WasmExecution,
        })
        .collect();

    let node    = RemoteNode::new(address.clone(), caps);
    let node_id = node.node_id.to_string();
    rt.cluster.register(node);

    Ok(NodeRegistrationResponse { node_id, address, status: "registered".into() })
}

// ── AI Provider fonksiyonları (FRB) ───────────────────────────
//
// Kullanıcı Ayarlar ekranında bir provider için API key (Anthropic/
// OpenAI/Gemini) ya da host (Ollama) girip "kaydet"e bastığında
// Flutter bu fonksiyonu çağırır. Birden fazla provider kayıtlıysa
// hangisinin kullanılacağına set_active_ai_provider ile kullanıcı
// karar verir — otomatik fallback YOK (bkz. ai::routing::router).

/// Bir AI provider'ı yapılandır ve ProviderRouter'a kaydet.
///
/// provider_id: "anthropic" | "openai" | "gemini" | "ollama"
/// api_key:     Anthropic/OpenAI/Gemini için zorunlu, Ollama için yok sayılır
/// base_url:    sadece Ollama için (örn. "http://127.0.0.1:11434")
/// model:       opsiyonel — verilmezse provider'ın varsayılan modeli kullanılır
///
/// İlk kaydedilen provider otomatik aktif olur. Daha önce aynı
/// provider_id ile kayıt yapılmışsa (örn. key güncellendi) üzerine yazılır.
pub fn configure_ai_provider(
    provider_id: String,
    api_key:     Option<String>,
    base_url:    Option<String>,
    model:       Option<String>,
) -> Result<(), String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let provider = crate::ai::routing::router::build_provider(
        &provider_id,
        api_key,
        base_url,
        model,
    )
    .map_err(|e| e.to_string())?;

    rt.ai_router.register(provider);

    info!(provider_id = %provider_id, "AI provider yapılandırıldı");
    Ok(())
}

/// Kayıtlı bir provider'ı kaldır (kullanıcı key'i sildiğinde / bağlantıyı
/// kapattığında). Kaldırılan provider aktifse aktif seçim temizlenir.
pub fn remove_ai_provider(provider_id: String) -> Result<(), String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    rt.ai_router.unregister(&provider_id);
    Ok(())
}

/// Aktif (kullanılacak) provider'ı kullanıcı seçimine göre değiştir.
/// Birden fazla provider kayıtlıysa Ayarlar ekranındaki seçim burada
/// uygulanır.
pub fn set_active_ai_provider(provider_id: String) -> Result<(), String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    rt.ai_router.set_active(&provider_id)
        .map_err(|e| e.to_string())
}

/// Şu an aktif olan provider_id (varsa).
pub fn get_active_ai_provider() -> Option<String> {
    get_runtime().and_then(|rt| rt.ai_router.active_id())
}

/// Kayıtlı (yapılandırılmış) tüm provider id'leri.
/// Ayarlar ekranında "hangi modeller aktif" göstermek için.
pub fn list_ai_providers() -> Vec<String> {
    get_runtime()
        .map(|rt| rt.ai_router.registered_ids())
        .unwrap_or_default()
}

/// Verilen Ollama sunucusunda yüklü (pull edilmiş) modelleri listele.
/// Ayarlar ekranındaki Ollama model dropdown'ını doldurmak için —
/// runtime başlatılmış olmasına gerek yok, doğrudan HTTP çağrısı.
pub async fn list_ollama_models(base_url: String) -> Result<Vec<String>, String> {
    crate::ai::providers::ollama::OllamaProvider::list_models(&base_url)
        .await
        .map_err(|e| e.to_string())
}

/// Ayarlar ekranındaki "Bağlantıyı Test Et" butonu için: kayıtlı bir
/// provider'a küçük bir inference isteği gönderir, kısa bir çıktı
/// parçası döner (başarılıysa key/host geçerli demektir).
pub async fn test_ai_provider(provider_id: String) -> Result<String, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let provider = rt
        .ai_router
        .provider(&provider_id)
        .ok_or_else(|| "Provider henüz yapılandırılmadı".to_string())?;

    let request = crate::ai::inference::request::InferenceRequest::new(
        "Tek kelimeyle selam ver.",
        16,
    );

    let response = provider
        .infer(request)
        .await
        .map_err(|e| e.to_string())?;

    Ok(response.output)
}

/// Genel amaçlı, tek seferlik AI sohbet isteği — AKTİF provider üzerinden
/// çalışır. Flutter'daki AI Chat ekranı bunu kullanır: hangi provider'ın
/// yanıt vereceği kullanıcının Ayarlar'da seçtiği aktif modele bağlıdır
/// (Anthropic/OpenAI/Gemini/Ollama — kullanıcı hangisini aktif ettiyse).
pub async fn ai_chat(
    prompt: String,
    system_prompt: Option<String>,
    max_tokens: usize,
) -> Result<String, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let mut request = crate::ai::inference::request::InferenceRequest::new(prompt, max_tokens);
    if let Some(system) = system_prompt {
        request = request.with_system(system);
    }

    rt.ai_router
        .infer_active(request)
        .await
        .map(|r| r.output)
        .map_err(|e| e.to_string())
}
