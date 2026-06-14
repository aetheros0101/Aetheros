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
    LogEntry, MetricsSnapshot, ModuleUploadResponse,
    RuntimeInfo, TaskRequest, TaskStatusResponse,
};
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

    let all = rt
        .persistence
        .load_all_tasks()
        .map_err(|e| format!("Persistence hatası: {e:?}"))?;

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
pub async fn upload_wasm_module(
    bytes: Vec<u8>,
) -> Result<ModuleUploadResponse, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let size = bytes.len() as u64;

    // 1) ModuleStore'a kaydet (RAM — hızlı lookup için)
    let hash_bytes = rt
        .module_store
        .store(bytes.clone())
        .map_err(|e| format!("Modül kaydedilemedi: {e}"))?;

    // 2) Sled'e kaydet (disk — restart sonrası recovery için)
    //
    // Bu adım olmazsa uygulama yeniden başladığında modül kaybolur
    // ve "invalid module: module not found in store" hatası çıkar.
    rt.persistence
        .persist_module(&hash_bytes, &bytes)
        .map_err(|e| format!("Modül diske kaydedilemedi: {e:?}"))?;

    let hash = crate::wasm::module_store::ModuleStore::hash_to_hex(&hash_bytes);

    info!(hash = %hash, size = size, "WASM modülü yüklendi (RAM + disk)");

    Ok(ModuleUploadResponse { hash, size })
}

// ── Modül durum kontrolü ──────────────────────────────────

/// Bir WASM modülünün runtime'da hazır olup olmadığını kontrol et.
///
/// Flutter, modül listesini SharedPreferences'tan gösterirken bu
/// fonksiyonla hangi modüllerin gerçekten kullanılabilir olduğunu
/// doğrulayabilir. Disk'ten yüklenmemiş veya hiç upload edilmemiş
/// hash'ler için false döner → UI "yeniden yükle" uyarısı gösterebilir.
pub fn check_module_exists(hash_hex: String) -> Result<bool, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let hash = parse_hash(&hash_hex)?;
    Ok(rt.module_store.contains(&hash))
}

/// Mevcut bir task'ı yeni bir task olarak yeniden gönder.
///
/// "Yeniden Dene" butonu için: orijinal task ayarlarını
/// (hash, entrypoint, priority, retry policy) kopyalayıp
/// yeni UUID ile kuyruğa ekler.
///
/// NOT: `InvalidModule` (Permanent) hatalarında bile çalışır —
/// kullanıcı modülü yeniden yükledikten sonra aynı task'ı
/// retry edebilir.
pub async fn resubmit_task(
    task_id: String,
) -> Result<String, String> {
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

    let now = chrono::Utc::now();
    let new_id = TaskId(uuid::Uuid::new_v4());

    // Orijinal task tanımını kopyala, sadece id ve timestamp yenile
    let new_task = TaskDefinition {
        id:         new_id.clone(),
        parent:     persisted.task.parent,
        orchestration: persisted.task.orchestration,
        priority:   persisted.task.priority,
        deadline:   None, // deadline sıfırla
        timeout_ms: persisted.task.timeout_ms,
        retry_policy: persisted.task.retry_policy,
        metadata:   persisted.task.metadata,
        wasm_module_hash: persisted.task.wasm_module_hash,
        entrypoint: persisted.task.entrypoint,
        state:      TaskState::Created,
        created_at: now,
        updated_at: now,
    };

    rt.handle
        .submit(new_task)
        .await
        .map_err(|e| format!("Resubmit hatası: {e:?}"))?;

    info!(
        original = %task_id,
        new      = %new_id.0,
        "Task yeniden gönderildi"
    );

    Ok(new_id.0.to_string())
}

// ── Log izleme ────────────────────────────────────────────

/// Son `limit` kadar log entry'yi yeniden eskiye sıralı döndür.
///
/// Flutter LogScreen, 2 saniyede bir bu fonksiyonu polling'le çeker.
/// Limit: 0 → varsayılan 100, max 500.
pub fn get_recent_logs(limit: u32) -> Result<Vec<LogEntry>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let n = match limit {
        0          => 100,
        n if n > 500 => 500,
        n          => n as usize,
    };

    Ok(rt.log_buffer
        .recent(n)
        .into_iter()
        .map(bridge_log_entry)
        .collect())
}

/// Belirli bir task'a ait log entry'lerini döndür.
///
/// Task detail modalında "Bu task'ın logları" için kullanılır.
/// limit: 0 → son 50 entry.
pub fn get_task_logs(
    task_id: String,
    limit: u32,
) -> Result<Vec<LogEntry>, String> {
    let rt = get_runtime()
        .ok_or_else(|| "RuntimeNotInitialized".to_string())?;

    let n = match limit {
        0          => 50,
        n if n > 200 => 200,
        n          => n as usize,
    };

    Ok(rt.log_buffer
        .by_task(&task_id, n)
        .into_iter()
        .map(bridge_log_entry)
        .collect())
}

/// `logging::buffer::LogEntry` → `bridge::types::LogEntry` dönüşümü.
fn bridge_log_entry(e: crate::logging::buffer::LogEntry) -> LogEntry {
    LogEntry {
        timestamp_ms: e.timestamp_ms,
        level:        e.level.to_owned(),
        task_id:      e.task_id,
        message:      e.message,
        event_type:   e.event_type.to_owned(),
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
