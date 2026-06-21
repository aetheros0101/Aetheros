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
    LogRecord, MetricsSnapshot, ModuleUploadResponse,
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
    name:        String,
    wat_source:  String,
    entrypoint:  String,
    timeout_ms:  u64,
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
