// ============================================================
// src/runtime/runtime.rs
//
// Faz 1 Düzeltmeleri: (korunuyor)
// Koşullu Geçiş: WasmEngine → Arc<dyn WasmExecutor>
//
//   #[cfg(feature = "backend-wasmtime")]  → WasmEngine (server)
//   #[cfg(feature = "backend-wasmi")]     → WasmiEngine (Android)
//
//   Her iki yol da WasmExecutor trait'ini döndürür.
//   Geri kalan runtime kodu değişmez.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{
    AtomicU8,
    Ordering,
};

use tokio::select;
use tokio::sync::{
    mpsc,
    RwLock,
};

use crate::errors::runtime::RuntimeError;
use crate::errors::persistence::PersistenceError;use crate::events::bus::{
    EventBus,
    SystemEvent,
};
use crate::events::runtime::RuntimeEvent;
use crate::runtime::config::RuntimeConfig;
use crate::runtime::dispatcher::Dispatcher;
use crate::runtime::scheduler::Scheduler;
use crate::runtime::shutdown::ShutdownController;
use crate::runtime::backpressure::BackpressureController;
use crate::runtime::lifecycle::RuntimeState;
use crate::task::queue::PriorityTaskQueue;
use crate::task::task::TaskDefinition;
use crate::wasm::WasmExecutor;
use crate::worker::manager::WorkerManager;
use crate::persistence::engine::PersistenceEngine;
use crate::persistence::models::PersistedTask;
use crate::persistence::recovery::RecoveryEngine;
use crate::wasm::module_store::ModuleStore;

// ── Backend import'ları ───────────────────────────────────
#[cfg(feature = "backend-wasmtime")]
use crate::wasm::engine::WasmEngine;
#[cfg(feature = "backend-wasmtime")]
use crate::wasm::sandbox::SandboxLimits;

#[cfg(feature = "backend-wasmi")]
use crate::wasm::wasmi_engine::{WasmiEngine, WasmiSandboxLimits};

pub struct Runtime {
    config: RuntimeConfig,
    event_bus: EventBus,
    shutdown: ShutdownController,
    task_receiver: mpsc::Receiver<TaskDefinition>,
    scheduler: Arc<Scheduler>,
    dispatcher: Arc<Dispatcher>,
    persistence: Arc<PersistenceEngine>,
    module_store: Arc<ModuleStore>,
    state: Arc<AtomicU8>,
}

impl Runtime {
    pub fn new(
        config: RuntimeConfig,
        task_receiver: mpsc::Receiver<TaskDefinition>,
    ) -> Result<Self, RuntimeError> {
        let event_bus = EventBus::new(
            config.event_channel_capacity,
        );

        let queue = Arc::new(
            PriorityTaskQueue::new(),
        );

        let scheduler =
            Arc::new(Scheduler::new(queue.clone()));

        let persistence = Arc::new(
            PersistenceEngine::open(
                &config.persistence_path,
            )?,
        );

        // DÜZELTME (madde #7): ModuleStore artık `persistence` ile aynı
        // sled veritabanına write-through yapıyor ve açılışta mevcut
        // modülleri geri yüklüyor — restart sonrası "module not found
        // in store" ile başarısız olan recovered task'lar sorunu çözüldü.
        let module_store = Arc::new(
            crate::wasm::module_store::ModuleStore::with_persistence(
                Arc::clone(&persistence),
            )
            .map_err(|_| RuntimeError::Persistence(PersistenceError::StorageFailure))?
        );

        // ── WASM Engine — derleme zamanında seçilir ───────────
        //
        //   backend-wasmtime → JIT, Cranelift, server performansı
        //   backend-wasmi    → Interpreter, Android/Play Store uyumlu
        //
        //   Her iki yol da Arc<dyn WasmExecutor> döndürür.
        //   Geri kalan runtime kodu backend'i bilmez.

        #[cfg(feature = "backend-wasmtime")]
        let engine: Arc<dyn WasmExecutor> = Arc::new(
            WasmEngine::new(SandboxLimits {
                memory_limit_bytes: 64 * 1024 * 1024,
                execution_timeout: std::time::Duration::from_secs(30),
                fuel_limit: 10_000_000,
            }, module_store.clone())?
        );

        #[cfg(feature = "backend-wasmi")]
        let engine: Arc<dyn WasmExecutor> = Arc::new(
            WasmiEngine::new(
                WasmiSandboxLimits {
                    fuel_limit:        10_000_000,
                    execution_timeout: std::time::Duration::from_secs(30),
                },
                module_store.clone(),
            )
        );

        let mut manager = WorkerManager::new();

        manager.spawn_workers(
            config.worker_count,
            config.task_channel_capacity,
            engine,
            event_bus.clone(),
            persistence.clone(),
            queue.clone(),
        );

        let workers =
            Arc::new(RwLock::new(manager));

        let backpressure = BackpressureController::new(
            config.max_concurrent_tasks,
        );

        let dispatcher = Arc::new(
            Dispatcher::new(queue, workers, backpressure),
        );

        Ok(Self {
            state: Arc::new(AtomicU8::new(
                RuntimeState::Created as u8,
            )),
            config,
            event_bus,
            shutdown: ShutdownController::new(),
            task_receiver,
            scheduler,
            dispatcher,
            persistence,
            module_store,
        })
    }

    pub async fn start(
        mut self,
    ) -> Result<(), RuntimeError> {
        // ── 1. Starting ───────────────────────────────────────
        self.set_state(RuntimeState::Starting);

        // ── 2. Recovery (Running'den ÖNCE yapılmalı) ─────────
        //
        // DÜZELTME: Orijinalde state Running'e çekildikten SONRA
        // recovery yapılıyordu. Dışarıya "hazırım" sinyali
        // vermeden önce tüm kurtarma adımları tamamlanmalı.
        let recovered_tasks =
            self.persistence.load_all_tasks()?;

        let recovered =
            RecoveryEngine::recoverable_tasks(recovered_tasks);

        for mut task in recovered {
            RecoveryEngine::mark_recovered(&mut task);
            self.scheduler.submit(task.task).await;
        }

        // ── 3. Dispatcher başlat ──────────────────────────────
        let dispatcher = self.dispatcher.clone();

        let dispatcher_handle =
            tokio::spawn(async move {
                dispatcher.run().await
            });

        // ── 4. Running — recovery + dispatcher hazır ─────────
        self.set_state(RuntimeState::Running);

        self.event_bus.publish(
            SystemEvent::Runtime(RuntimeEvent::RuntimeStarted),
        );

        // ── 5. Ana döngü ──────────────────────────────────────
        //
        // DÜZELTME: dispatcher_handle.abort() ve state=Stopped
        // orijinalde select! bloğundan SONRA ama LOOP İÇİNDE
        // yer alıyordu. Bu, Some(task) her eşleştiğinde (break
        // olmadığı için) dispatcher'ı öldürüyordu.
        //
        // Düzeltme: her branch açıkça `continue` veya `break`
        // kullanıyor; shutdown sekansı loop DIŞINDA.
        loop {
            select! {
                // Shutdown sinyali
                _ = self.shutdown.wait() => {
                    self.set_state(RuntimeState::Draining);

                    // Süregelen task'lara drain süresi tanı
                    let _ = self.shutdown
                        .wait_timeout(self.config.shutdown_timeout)
                        .await;

                    break;
                }

                // Gelen task
                maybe_task = self.task_receiver.recv() => {
                    match maybe_task {
                        Some(task) => {
                            let persisted = PersistedTask {
                                task: task.clone(),
                                created_at: chrono::Utc::now(),
                                updated_at: chrono::Utc::now(),
                                attempts: 0,
                                last_error: None,
                            };

                            self.persistence
                                .persist_task(&persisted)?;

                            self.scheduler.submit(task).await;

                            // DÜZELTME: açık continue — fall-through'u engeller.
                            // Orijinalde bu yoktu; loop gövdesindeki
                            // abort() her task'tan sonra çalışıyordu.
                            continue;
                        }

                        // Gönderici taraf düştü → graceful shutdown
                        None => break,
                    }
                }
            }
        }

        // ── 6. Graceful shutdown sekansı ──────────────────────
        //
        // DÜZELTME: Orijinalde bu blok loop içindeydi ve hem
        // yanlış yerde hem de eksikti (worker'lara Shutdown
        // mesajı hiç gönderilmiyordu).

        self.set_state(RuntimeState::Stopping);

        // Worker'lara düzgün Shutdown mesajı gönder.
        // dispatcher.shutdown_workers() → WorkerMessage::Shutdown
        // her worker kanalına iletilir; worker aktif task'ını
        // tamamladıktan sonra temiz çıkar.
        self.dispatcher.shutdown_workers().await;

        // Dispatcher loop'unu durdur ve tamamlanmasını bekle
        dispatcher_handle.abort();
        let _ = dispatcher_handle.await;

        // ── 7. Durduruldu ─────────────────────────────────────
        self.set_state(RuntimeState::Stopped);

        self.event_bus.publish(
            SystemEvent::Runtime(RuntimeEvent::RuntimeStopped),
        );

        Ok(())
    }

    // ── Yardımcı metotlar ─────────────────────────────────────

    /// Runtime'ı dışarıdan durdur (örn. sinyal handler'dan).
    pub fn shutdown(&self) {
        self.shutdown.cancel();
    }

    pub fn events(&self) -> EventBus {
        self.event_bus.clone()
    }

    pub fn config(&self) -> &RuntimeConfig {
        &self.config
    }

    pub fn persistence(&self) -> Arc<PersistenceEngine> {
        self.persistence.clone()
    }

    /// WASM modül deposu — upload edilen binary'lerin
    /// hash → bytes eşlemesi. Bridge katmanı (upload_wasm_module)
    /// bu handle üzerinden yeni modülleri kaydeder; WasmiEngine
    /// (worker'lar içinde) aynı Arc'ı paylaşır, böylece upload
    /// edilen modül execute sırasında bulunabilir.
    pub fn module_store(&self) -> Arc<ModuleStore> {
        self.module_store.clone()
    }

    pub fn state(&self) -> RuntimeState {
        RuntimeState::from_u8(
            self.state.load(Ordering::SeqCst),
        )
    }

    // State geçişini tek yerden yap — tekrar azaltır.
    fn set_state(&self, state: RuntimeState) {
        self.state.store(state as u8, Ordering::SeqCst);
    }
}
