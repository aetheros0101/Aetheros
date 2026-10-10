// ============================================================
// bridge/api/runtime.rs
//
// Runtime yaşam döngüsü ve FRB init.
// Sorumluluk: uygulama başlatma, runtime hazırlık kontrolü.
// ============================================================

use flutter_rust_bridge::frb;
use tracing::info;

use crate::bridge::state::{get_runtime, init_mobile_runtime};
use crate::bridge::types::RuntimeInfo;

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
pub async fn initialize_runtime(db_path: String, worker_count: u32) -> Result<(), String> {
    info!(db = %db_path, workers = worker_count, "initialize_runtime çağrıldı");

    // FRB kendi Tokio runtime'ını çalıştırıyor.
    // init_mobile_runtime() içinde tokio.block_on() var — aynı thread'de
    // çağrılırsa "Cannot start a runtime from within a runtime" paniği olur.
    //
    // Çözüm: spawn_blocking → blocking thread pool'da çalıştır.
    // Bu thread'lerde aktif Tokio context YOK, dolayısıyla block_on güvenli.
    tokio::task::spawn_blocking(move || init_mobile_runtime(db_path, worker_count as usize))
        .await
        .map_err(|e| format!("Thread başlatma hatası: {e}"))?
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
        version: env!("CARGO_PKG_VERSION").to_string(),
        is_running: get_runtime().is_some(),
        backend,
        worker_count: get_runtime().map(|_| 2u32).unwrap_or(0),
    }
}
