// ============================================================
// src/worker/supervisor.rs  (YENİ)
//
// Faz 4 Eklemesi:
//
// [SORUN E] Worker JoinHandle'ları izlenmiyordu.
//   Worker panic alırsa JoinHandle resolve olur ama kimse
//   bakmadığı için task'lar sessizce kaybolur, sistem
//   farkında olmadan kapasitesini yitirir.
//
//   WorkerSupervisor:
//   - JoinSet üzerinden tüm worker'ları izler
//   - Beklenmedik çıkış (panic veya RuntimeError) → yeni
//     worker spawn eder, hedef kapasiteyi korur
//   - Normal çıkış (Shutdown mesajı sonrası Ok(())) →
//     respawn etmez (shutdown akışı)
//   - Shutdown sinyali gelince izlemeyi durdurur
// ============================================================

use std::sync::Arc;

use tokio::sync::watch;
use tokio::task::JoinSet;
use tracing::{
    error,
    info,
    warn,
};

use crate::errors::runtime::RuntimeError;
use crate::events::bus::EventBus;
use crate::persistence::engine::PersistenceEngine;
use crate::task::queue::PriorityTaskQueue;
use crate::wasm::WasmExecutor;
use crate::worker::executor::WorkerExecutor;
use crate::worker::message::WorkerMessage;
use crate::worker::worker::Worker;

/// Worker spawn için gereken tüm bağımlılıklar.
/// Supervisor, crashed worker'ı yeniden spawn etmek için
/// bu parametreleri tutar.
pub struct WorkerSpawnParams {
    pub channel_capacity: usize,
    /// Hangi backend (wasmtime / wasmi) olduğu önemli değil.
    /// Derleme zamanında seçilir, runtime'da trait üzerinden çalışır.
    pub engine: Arc<dyn WasmExecutor>,
    pub events: EventBus,
    pub persistence: Arc<PersistenceEngine>,
    pub retry_queue: Arc<PriorityTaskQueue>,
}

pub struct WorkerSupervisor {
    set: JoinSet<Result<(), RuntimeError>>,
    /// Her worker'ın gönderici ucu — dispatcher için.
    /// Supervisor, yeni worker spawn ettiğinde buraya ekler.
    senders: Vec<tokio::sync::mpsc::Sender<WorkerMessage>>,
    params: WorkerSpawnParams,
    target_count: usize,
}

impl WorkerSupervisor {
    pub fn new(
        params: WorkerSpawnParams,
        target_count: usize,
    ) -> Self {
        Self {
            set: JoinSet::new(),
            senders: Vec::with_capacity(target_count),
            params,
            target_count,
        }
    }

    /// İlk worker'ları spawn et.
    /// WorkerManager yerine Supervisor artık spawn eder.
    pub fn spawn_initial(&mut self) {
        for _ in 0..self.target_count {
            self.spawn_one();
        }
    }

    /// Dispatcher'ın task göndermesi için sender'ları al.
    pub fn senders(
        &self,
    ) -> &[tokio::sync::mpsc::Sender<WorkerMessage>] {
        &self.senders
    }

    /// Arka plan izleme döngüsünü başlat.
    ///
    /// Shutdown sinyali alınınca izlemeyi durdurur.
    /// Kalan worker'lar Shutdown mesajı aldıktan sonra
    /// temiz çıkar (WorkerManager.send_shutdown_all() ile).
    pub async fn run(
        mut self,
        mut shutdown_rx: watch::Receiver<bool>,
    ) {
        info!(
            target_count = self.target_count,
            "Worker supervisor started"
        );

        loop {
            tokio::select! {
                // Bir worker çıkış yaptı
                Some(result) = self.set.join_next() => {
                    match result {
                        // Worker panikledi
                        Err(join_err) => {
                            if join_err.is_panic() {
                                error!(
                                    "Worker panicked — respawning"
                                );
                            } else {
                                warn!(
                                    "Worker task cancelled — respawning"
                                );
                            }
                            // Shutdown değilse yeniden spawn et
                            if !*shutdown_rx.borrow() {
                                self.spawn_one();
                            }
                        }

                        // Worker RuntimeError döndürdü
                        Ok(Err(e)) => {
                            error!(
                                error = ?e,
                                "Worker exited with error — respawning"
                            );
                            if !*shutdown_rx.borrow() {
                                self.spawn_one();
                            }
                        }

                        // Temiz çıkış (Shutdown mesajı sonrası)
                        Ok(Ok(())) => {
                            info!("Worker exited cleanly");
                            // Shutdown değilse beklenmedik — respawn
                            if !*shutdown_rx.borrow() {
                                warn!(
                                    "Worker exited cleanly but \
                                     shutdown not requested — respawning"
                                );
                                self.spawn_one();
                            }
                        }
                    }
                }

                // Shutdown sinyali
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        info!("Worker supervisor shutting down");
                        // JoinSet abort: kalan worker'lara
                        // zaten Shutdown mesajı gönderildi (manager'dan)
                        self.set.abort_all();
                        break;
                    }
                }
            }
        }

        info!("Worker supervisor stopped");
    }

    /// Tek worker spawn et + sender'ı kaydet.
    fn spawn_one(&mut self) {
        let (sender, receiver) =
            tokio::sync::mpsc::channel(
                self.params.channel_capacity,
            );

        let executor = Arc::new(WorkerExecutor::new(
            self.params.engine.clone(),
        ));

        let worker = Worker::new(
            receiver,
            executor,
            self.params.events.clone(),
            self.params.persistence.clone(),
            self.params.retry_queue.clone(),
        );

        self.set.spawn(async move { worker.run().await });
        self.senders.push(sender);
    }
}
