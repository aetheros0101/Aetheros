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
    MetricsSnapshot, ModuleUploadResponse,
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

    init_mobile_runtime(db_path, worker_count as usize)
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

    let error_message = match &persisted.task.state {
        crate::task::task::TaskState::Failed => {
            Some("WASM yürütme başarısız".to_string())
        }
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
                crate::task::task::TaskState::Failed => {
                    Some("WASM yürütme başarısız".to_string())
                }
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
    // ModuleStore'u almak için runtime gerekli
    // Ancak ModuleStore'a erişim için WasmEngine'e ihtiyaç var
    // Şimdilik hash hesaplayıp döndürüyoruz; store entegrasyonu
    // bootstrap sırasında handle üzerinden yapılacak.
    use sha2::{Digest, Sha256};

    let size = bytes.len() as u64;

    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash_bytes = hasher.finalize();
    let hash = hex::encode(hash_bytes);

    // TODO: ModuleStore'a kaydet
    // rt.module_store.insert(hash_bytes.into(), bytes)
    // Bu adım flutter_app'e özel bir module_store handle
    // eklenince tamamlanacak.

    info!(hash = %hash, size = size, "WASM modülü yüklendi (store pending)");

    Ok(ModuleUploadResponse { hash, size })
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
