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

use crate::agents::approval::ApprovalStore;
use crate::ai::routing::router::ProviderRouter;
use crate::bridge::agent::{AgentRegistry, WorkflowRegistry};
use crate::events::bus::EventBus;
use crate::logging::audit::AuditLog;
use crate::logging::buffer::{LogBuffer, log_collector};
use crate::metrics::runtime::RuntimeMetrics;
use crate::persistence::engine::PersistenceEngine;
use crate::runtime::api::RuntimeHandle;
use crate::runtime::bootstrap::RuntimeBootstrap;
use crate::runtime::config::RuntimeConfig;
use crate::scripting::engine::ScriptEngine;
use crate::scripting::registry::ScriptRegistry;
use crate::security::capability_engine::CapabilityEngine;
use crate::security::risk_engine::RiskEngine;
use crate::wasm::module_store::ModuleStore;

// ── Backend import'ları (agent tool WASM execution için) ───
// runtime/runtime.rs ile aynı desen: derleme zamanında seçilir.
#[cfg(feature = "backend-wasmtime")]
use crate::wasm::engine::WasmEngine;
#[cfg(feature = "backend-wasmtime")]
use crate::wasm::sandbox::SandboxLimits;

#[cfg(feature = "backend-wasmi")]
use crate::wasm::wasmi_engine::{WasmiEngine, WasmiSandboxLimits};

// ── Global state ─────────────────────────────────────────

static MOBILE_RUNTIME: OnceLock<MobileRuntime> = OnceLock::new();

pub struct MobileRuntime {
    pub handle:            RuntimeHandle,
    pub events:            EventBus,
    pub persistence:       Arc<PersistenceEngine>,
    pub metrics:           Arc<RuntimeMetrics>,
    pub module_store:      Arc<ModuleStore>,
    pub log_buffer:        LogBuffer,
    /// WASM binary'lerinin kalıcı dizini: {docDir}/modules/
    pub modules_dir:       String,
    /// Agent execution kaydı (in-memory, Faz-2'de persist)
    pub agent_registry:    AgentRegistry,
    /// Workflow execution kaydı (in-memory, Faz-2'de persist)
    pub workflow_registry: WorkflowRegistry,
    /// Cluster state — local/remote dispatch
    pub cluster:           Arc<crate::remote::cluster::ClusterState>,
    /// AI provider registry + aktif provider seçimi.
    /// Boş başlar — Flutter, Ayarlar'da kayıtlı provider'ları
    /// initialize_runtime() sonrası configure_ai_provider() ile
    /// tek tek bu router'a kaydeder.
    pub ai_router:         Arc<ProviderRouter>,
    /// V10 Sprint 1: agent'lara verilmiş capability grant'leri.
    /// Deny-by-default: hiçbir agent, burada açıkça grant edilmemiş
    /// bir capability'yi gerektiren tool'u çalıştıramaz. Grant etme
    /// mekanizması (Ayarlar / Approval Engine) henüz yok — bu yüzden
    /// şu an tüm agent'lar en güvenli (kısıtlı) durumda başlıyor.
    pub capability_engine: Arc<CapabilityEngine>,
    /// V10 Sprint 3: her tool çağrısına tool-bazlı bir risk seviyesi
    /// atayan Risk Engine. Sadece değerlendirir — bloklama kararı
    /// AgentRuntime'ın (geçici) HighRiskPolicy'sinde.
    pub risk_engine:       Arc<RiskEngine>,
    /// V10 Sprint 5: RequiresApproval durumunda duraklatılan
    /// execution'ların kaydı. respond_to_approval() bunu okuyup
    /// AgentRuntime::resume() ile devam ettirir ya da kalıcı olarak
    /// reddeder.
    pub approval_store:    Arc<ApprovalStore>,
    /// V10 Sprint 6: Governor kararlarının, tool çağrılarının ve
    /// pause/resume olaylarının kaydedildiği denetim izi.
    pub audit_log:         Arc<AuditLog>,
    /// V10 Sprint 1b: agent'ların çağırabileceği isimlendirilmiş
    /// WASM script'lerinin kaydı — bkz. register_agent_script().
    pub script_registry:   Arc<ScriptRegistry>,
    /// ScriptTool'ların paylaştığı gerçek WASM çalıştırıcı.
    /// module_store ile aynı KV deposunu kullanır — task execution
    /// engine'inden bağımsız, ayrı bir Arc<dyn WasmExecutor> örneği.
    pub script_engine:     Arc<ScriptEngine>,
    pub tokio:             TokioRuntime,
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

    let tokio = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(worker_count.max(2))
        .enable_all()
        .build()
        .map_err(|e| format!("Tokio başlatma hatası: {e}"))?;

    let config = RuntimeConfig {
        worker_count,
        task_channel_capacity:  512,
        event_channel_capacity: 1024,
        max_concurrent_tasks:   64,
        persistence_path:       db_path.clone(),
        shutdown_timeout:       std::time::Duration::from_secs(10),
    };

    let bootstrap = tokio
        .block_on(async { RuntimeBootstrap::build(config) })
        .map_err(|e| format!("Bootstrap hatası: {e}"))?;

    let handle       = bootstrap.runtime_handle();
    let runtime      = bootstrap.runtime();
    let events       = runtime.events();
    let persistence  = runtime.persistence();
    let module_store = runtime.module_store();

    // ── Modules dizini ───────────────────────────────────
    // db_path = {docDir}/aetheros.db → modules_dir = {docDir}/modules/
    let modules_dir = {
        let p = std::path::Path::new(&db_path);
        let parent = p.parent().unwrap_or(std::path::Path::new("."));
        parent.join("modules").display().to_string()
    };
    std::fs::create_dir_all(&modules_dir)
        .map_err(|e| format!("Modules dizini oluşturulamadı: {e}"))?;

    // ── Önceki oturumdan kalan WASM binary'lerini restore et ─
    restore_modules_from_disk(&modules_dir, &module_store);

    // ── Agent Tool WASM Engine (V10 Sprint 1b) ───────────────
    // Task execution engine'inden bağımsız, ScriptTool'lar için ayrı
    // bir Arc<dyn WasmExecutor>. Aynı module_store'u paylaşır (aynı
    // hash → aynı binary), ama ayrı bir örnek — task worker'larının
    // fuel/timeout bütçesini agent script'leriyle karıştırmaz.
    #[cfg(feature = "backend-wasmtime")]
    let script_wasm_executor: Arc<dyn crate::wasm::WasmExecutor> = Arc::new(
        WasmEngine::new(
            SandboxLimits {
                memory_limit_bytes: 64 * 1024 * 1024,
                execution_timeout: std::time::Duration::from_secs(30),
                fuel_limit: 10_000_000,
            },
            module_store.clone(),
        )
        .map_err(|e| format!("Agent script WASM engine kurulamadı: {e}"))?,
    );

    #[cfg(feature = "backend-wasmi")]
    let script_wasm_executor: Arc<dyn crate::wasm::WasmExecutor> = Arc::new(WasmiEngine::new(
        WasmiSandboxLimits {
            fuel_limit: 10_000_000,
            execution_timeout: std::time::Duration::from_secs(30),
        },
        module_store.clone(),
    ));

    let script_engine = Arc::new(ScriptEngine::new(script_wasm_executor));
    let script_registry = Arc::new(ScriptRegistry::new());

    // ── Metrics collector ────────────────────────────────
    let metrics = Arc::new(RuntimeMetrics::new());
    // NOT: start_collecting() içeride bare tokio::spawn() kullanıyor,
    // ki bu ortam ("reactor") gerektirir. Bu noktada tokio.block_on()
    // zaten bitmiş olduğundan, çağıran taraf (örn. bir test) kendi
    // ortamını sağlamıyorsa "there is no reactor running" ile panikler.
    // Üretimde bu şimdiye dek hiç görünmedi çünkü Flutter Rust Bridge
    // her çağrıyı zaten kendi Tokio ortamında yapıyor — ama buna
    // ÖRTÜK olarak güvenmek kırılgan. `tokio.enter()` ile KENDİ
    // ortamımızı garanti ediyoruz, çağıranın ortamına bağımlı kalmadan.
    let _enter = tokio.enter();
    metrics.clone().start_collecting(events.clone());

    // ── Log collector ─────────────────────────────────────
    let log_buffer = LogBuffer::new();
    tokio.spawn(log_collector(events.clone(), log_buffer.clone()));

    // ── Runtime arka planda ──────────────────────────────
    tokio.spawn(async move {
        if let Err(e) = runtime.start().await {
            tracing::error!(error = ?e, "AetherOS runtime durdu");
        }
    });

    info!(workers = worker_count, modules_dir = %modules_dir, "AetherOS mobile runtime başlatıldı");

    let mobile = MobileRuntime {
        handle,
        events,
        persistence,
        metrics,
        module_store,
        log_buffer,
        modules_dir,
        agent_registry:    Arc::new(dashmap::DashMap::new()),
        workflow_registry: Arc::new(dashmap::DashMap::new()),
        cluster:           Arc::new(crate::remote::cluster::ClusterState::new()),
        ai_router:         Arc::new(ProviderRouter::new()),
        capability_engine: Arc::new(CapabilityEngine::new()),
        risk_engine:       Arc::new(RiskEngine::new()),
        approval_store:    Arc::new(ApprovalStore::new()),
        audit_log:         Arc::new(AuditLog::default()),
        script_registry,
        script_engine,
        tokio,
    };

    MOBILE_RUNTIME
        .set(mobile)
        .map_err(|_| "Runtime zaten başlatılmış (race)".into())
}

/// Diske kaydedilmiş WASM binary'lerini ModuleStore'a yükle.
/// Yalnızca startup'ta çağrılır.
fn restore_modules_from_disk(modules_dir: &str, store: &Arc<ModuleStore>) {
    let dir = match std::fs::read_dir(modules_dir) {
        Ok(d) => d,
        Err(e) => {
            tracing::warn!(dir = modules_dir, err = %e, "Modules dizini okunamadı");
            return;
        }
    };

    let mut count = 0usize;
    for entry in dir.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("wasm") {
            continue;
        }
        match std::fs::read(&path) {
            Ok(bytes) => match store.store(bytes) {
                Ok(hash) => {
                    info!(
                        hash = %crate::wasm::module_store::ModuleStore::hash_to_hex(&hash),
                        file = %path.display(),
                        "WASM modülü restore edildi"
                    );
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!(file = %path.display(), err = ?e, "Modül restore edilemedi");
                }
            },
            Err(e) => {
                tracing::warn!(file = %path.display(), err = %e, "Modül dosyası okunamadı");
            }
        }
    }

    if count > 0 {
        info!(count, "WASM modülleri diskten restore edildi");
    }
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
