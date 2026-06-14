// ============================================================
// src/bridge/state.rs
//
// Mobil runtime'ın global yaşam döngüsü.
//
// NEDEN OnceLock?
//   Flutter uygulaması; init_runtime() → kullan → shutdown()
//   sırasını izler. İki kez init edilmemeli, shutdown sonrası
//   çağrı yapılmamalı. OnceLock bu garantiyi verir.
//
// NEDEN ayrı Tokio runtime?
//   Mobilde #[tokio::main] yok. Flutter'ın Dart event loop'u
//   farklı bir thread üzerinde çalışır. Rust tarafı kendi
//   tokio::runtime::Runtime örneğini yönetir; FRB bu runtime
//   üzerinden async çağrıları çalıştırır.
//
// THREAD GÜVENLİĞİ:
//   OnceLock<T>: T Send+Sync ise Send+Sync.
//   MobileRuntime: tüm alanlar Arc<> veya atomik → Send+Sync.
// ============================================================

use std::sync::{Arc, OnceLock};

use tokio::runtime::Runtime as TokioRuntime;
use tracing::info;

use crate::events::bus::EventBus;
use crate::logging::buffer::{log_collector, LogBuffer};
use crate::metrics::runtime::RuntimeMetrics;
use crate::persistence::engine::PersistenceEngine;
use crate::runtime::api::RuntimeHandle;
use crate::runtime::bootstrap::RuntimeBootstrap;
use crate::runtime::config::RuntimeConfig;
use crate::wasm::module_store::ModuleStore;

// ── Global state ─────────────────────────────────────────

static MOBILE_RUNTIME: OnceLock<MobileRuntime> = OnceLock::new();

pub struct MobileRuntime {
    /// AetherOS iç runtime handle — task submit için.
    pub handle: RuntimeHandle,

    /// Event bus — WebSocket veya yerel listener için.
    pub events: EventBus,

    /// Persistence engine — task sorgulama için.
    pub persistence: Arc<PersistenceEngine>,

    /// Runtime metrikleri — anlık görüntü için.
    pub metrics: Arc<RuntimeMetrics>,

    /// WASM modül deposu — upload_wasm_module() buraya yazar,
    /// WasmiEngine (worker'lar) aynı Arc'tan okur.
    pub module_store: Arc<ModuleStore>,

    /// Sprint 7: In-memory dairesel log tamponu.
    /// log_collector() tokio task'ı EventBus olaylarını buraya yazar.
    /// get_recent_logs() / get_task_logs() bridge fonksiyonları okur.
    pub log_buffer: LogBuffer,

    /// Tokio runtime — FRB bu üzerinden spawn eder.
    /// Option<> olması shutdown() sonrası temiz drop için.
    pub tokio: TokioRuntime,
}

// ── Public API ────────────────────────────────────────────

/// Runtime'ı başlat.
///
/// Flutter tarafından uygulama açılışında bir kez çağrılır.
/// İkinci çağrı AlreadyInitialized hatası döner.
pub fn init_mobile_runtime(
    db_path: String,
    worker_count: usize,
) -> Result<(), String> {
    if MOBILE_RUNTIME.get().is_some() {
        return Err("Runtime zaten başlatılmış".into());
    }

    // ── Tokio runtime ─────────────────────────────────────
    // multi_thread: worker sayısı = CPU çekirdeği (mobilde 4-8)
    // Mobil için sınırlandırmak istersen worker_threads(2) kullan.
    let tokio = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_count.max(2))
        .enable_all()
        .build()
        .map_err(|e| format!("Tokio başlatma hatası: {e}"))?;

    // ── AetherOS bootstrap ────────────────────────────────
    let config = RuntimeConfig {
        worker_count,
        task_channel_capacity:   512,
        event_channel_capacity:  1024,
        max_concurrent_tasks:    64,
        persistence_path:        db_path,
        shutdown_timeout:        std::time::Duration::from_secs(10),
    };

    let bootstrap = tokio
        .block_on(async {
            RuntimeBootstrap::build(config)
        })
        .map_err(|e| format!("Bootstrap hatası: {e}"))?;

    let handle       = bootstrap.runtime_handle();
    let runtime      = bootstrap.runtime();
    let events       = runtime.events();
    let persistence  = runtime.persistence();
    let module_store = runtime.module_store();

    // ── Startup: disk'ten ModuleStore'u yeniden doldur ───
    //
    // Runtime yeniden başladığında ModuleStore boş gelir (DashMap).
    // Önceki oturumlarda persist_module() ile kaydedilen binary'leri
    // sled'den okuyup ModuleStore'a yüklüyoruz.
    //
    // Bu sayede kullanıcı uygulamayı kapatıp açtığında WASM
    // modüllerini tekrar yüklemek zorunda kalmaz ve
    // "module not found in store" hatası ortadan kalkar.
    {
        let stored = persistence
            .load_all_modules()
            .map_err(|e| format!("Modül recovery hatası: {e:?}"))?;

        let recovered = stored.len();

        for (hash, binary) in stored {
            module_store
                .store_with_hash(hash, binary)
                .map_err(|e| format!("Modül yükleme hatası: {e:?}"))?;
        }

        info!(count = recovered, "WASM modülleri diskten yüklendi");
    }

    // ── Metrics collector ────────────────────────────────
    let metrics = Arc::new(RuntimeMetrics::new());
    metrics.clone().start_collecting(events.clone());

    // ── Sprint 7: Log tamponu + collector ────────────────
    //
    // LogBuffer: Arc<Mutex<VecDeque>> — max 500 entry.
    // log_collector() tokio task'ı EventBus'a subscribe olur,
    // gelen her TaskEvent'i LogEntry'ye çevirip buffer'a yazar.
    // get_recent_logs() / get_task_logs() bu buffer'ı okur.
    let log_buffer = LogBuffer::new();
    {
        let buf   = log_buffer.clone();
        let bus   = events.clone();
        tokio.spawn(async move {
            log_collector(bus, buf).await;
        });
    }

    // ── Runtime arka planda ──────────────────────────────
    tokio.spawn(async move {
        if let Err(e) = runtime.start().await {
            tracing::error!(error = ?e, "AetherOS runtime durdu");
        }
    });

    info!(
        workers = worker_count,
        "AetherOS mobile runtime başlatıldı"
    );

    let mobile = MobileRuntime {
        handle,
        events,
        persistence,
        metrics,
        module_store,
        log_buffer,
        tokio,
    };

    // OnceLock: set() başarısız olursa zaten başka bir thread
    // set etmiş demektir — AlreadyInitialized dön.
    MOBILE_RUNTIME
        .set(mobile)
        .map_err(|_| "Runtime zaten başlatılmış (race)".into())
}

/// Global runtime'ı al.
///
/// init_mobile_runtime() çağrılmadan önce kullanılırsa None döner.
/// Bridge fonksiyonları bu durumda "RuntimeNotInitialized" hatası verir.
pub fn get_runtime() -> Option<&'static MobileRuntime> {
    MOBILE_RUNTIME.get()
}

/// Tokio runtime üzerinden async blok çalıştır.
///
/// FRB zaten tokio context içinde çağırır; bu fonksiyon
/// doğrudan `await` kullanamayan yerlerde (sync context) işe yarar.
pub fn block_on<F, T>(fut: F) -> T
where
    F: std::future::Future<Output = T>,
{
    // FRB async context'te zaten tokio var; bu sadece fallback.
    // Normalde bridge/api.rs'deki `async fn`'ler doğrudan await kullanır.
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(fut)
    })
}
