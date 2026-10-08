// ============================================================
// src/worker/manager.rs  (v4)
//
// Faz 4 Düzeltmesi:
//
// [SORUN E] WorkerSupervisor entegre edildi.
//   spawn_workers() → WorkerSupervisor::spawn_initial()
//   Supervisor senders'ı tutar, dispatcher bunları kullanır.
//   start_supervision() arka plan izleme döngüsünü başlatır.
// ============================================================

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use tokio::sync::{mpsc, watch};

use crate::events::bus::EventBus;
use crate::persistence::engine::PersistenceEngine;
use crate::task::queue::PriorityTaskQueue;
use crate::wasm::WasmExecutor;
use crate::worker::message::WorkerMessage;
use crate::worker::supervisor::{WorkerSpawnParams, WorkerSupervisor};

#[derive(Clone)]
pub struct WorkerHandle {
    pub sender: mpsc::Sender<WorkerMessage>,
}

pub struct WorkerManager {
    /// Supervisor izleme döngüsü için shutdown sender
    shutdown_tx: watch::Sender<bool>,
    /// Dispatcher için round-robin index
    index: AtomicUsize,
    /// Worker sender'ları — dispatcher bunları kullanır
    senders: Vec<mpsc::Sender<WorkerMessage>>,
}

impl Default for WorkerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkerManager {
    pub fn new() -> Self {
        let (shutdown_tx, _) = watch::channel(false);
        Self {
            shutdown_tx,
            index: AtomicUsize::new(0),
            senders: Vec::new(),
        }
    }

    /// Worker'ları spawn et ve supervisor'ı arka planda başlat.
    pub fn spawn_workers(
        &mut self,
        count: usize,
        channel_capacity: usize,
        engine: Arc<dyn WasmExecutor>,
        events: EventBus,
        persistence: Arc<PersistenceEngine>,
        retry_queue: Arc<PriorityTaskQueue>,
    ) {
        let params = WorkerSpawnParams {
            channel_capacity,
            engine,
            events,
            persistence,
            retry_queue,
        };

        let mut supervisor = WorkerSupervisor::new(params, count);

        supervisor.spawn_initial();

        // Dispatcher için sender'ları al
        self.senders = supervisor.senders().to_vec();

        // Supervisor izleme döngüsünü arka planda başlat
        let shutdown_rx = self.shutdown_tx.subscribe();

        tokio::spawn(async move {
            supervisor.run(shutdown_rx).await;
        });
    }

    /// Round-robin worker seçimi.
    pub fn next_worker(&self) -> Option<WorkerHandle> {
        if self.senders.is_empty() {
            return None;
        }

        let index = self.index.fetch_add(1, Ordering::Relaxed);

        let sender = self.senders[index % self.senders.len()].clone();

        Some(WorkerHandle { sender })
    }

    /// Tüm worker'lara Shutdown mesajı gönder.
    pub async fn send_shutdown_all(&self) {
        // Önce supervisor'a shutdown sinyali ver
        let _ = self.shutdown_tx.send(true);

        // Sonra her worker'a mesaj gönder
        for sender in &self.senders {
            let _ = sender.send(WorkerMessage::Shutdown).await;
        }
    }
}
