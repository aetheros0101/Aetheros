// ============================================================
// src/runtime/dispatcher.rs  (v2)
//
// Faz 2 Düzeltmesi:
//
// [BUG #5] Permit artık task ile birlikte Worker'a gönderiliyor.
//
//   ÖNCE:
//     let _permit = self.backpressure.acquire().await;  ← alındı
//     worker.sender.send(Execute(task)).await?;
//     // loop sonunda _permit drop oldu — task henüz çalışmıyor!
//
//   SONRA:
//     let permit = self.backpressure.acquire().await;   ← alındı
//     worker.sender.send(Execute(task, permit)).await?;
//     // permit Worker içinde yaşıyor, execute bitince drop olur ✓
//
// shutdown_workers() Faz 1'den korundu.
// ============================================================

use std::sync::Arc;

use tokio::sync::RwLock;
use tokio::time::{Duration, sleep};

use crate::errors::runtime::RuntimeError;
use crate::runtime::backpressure::BackpressureController;
use crate::task::queue::PriorityTaskQueue;
use crate::worker::manager::{WorkerHandle, WorkerManager};
use crate::worker::message::WorkerMessage;

pub struct Dispatcher {
    queue: Arc<PriorityTaskQueue>,
    workers: Arc<RwLock<WorkerManager>>,
    backpressure: BackpressureController,
}

impl Dispatcher {
    pub fn new(
        queue: Arc<PriorityTaskQueue>,
        workers: Arc<RwLock<WorkerManager>>,
        backpressure: BackpressureController,
    ) -> Self {
        Self {
            queue,
            workers,
            backpressure,
        }
    }

    pub async fn run(&self) -> Result<(), RuntimeError> {
        loop {
            // [BUG #5] Permit alındı — artık task ile taşınacak.
            // Drop edilmeyecek, Worker'a devredilecek.
            let permit = self.backpressure.acquire().await;

            let task = self.queue.pop().await?;

            let worker = self.acquire_worker().await?;

            // [BUG #5] Permit task ile birlikte gönderiliyor.
            // Worker execute_task() tamamlayınca permit drop olur.
            // Semaphore ancak O ZAMAN bir slot açar. ✓
            worker
                .sender
                .send(WorkerMessage::Execute(task, permit))
                .await
                .map_err(|_| RuntimeError::WorkerSubsystemFailure)?;
        }
    }

    /// Faz 1'den: Tüm worker'lara Shutdown sinyali gönder.
    pub async fn shutdown_workers(&self) {
        let workers = self.workers.read().await;
        workers.send_shutdown_all().await;
    }

    async fn acquire_worker(&self) -> Result<WorkerHandle, RuntimeError> {
        loop {
            {
                let workers = self.workers.read().await;

                if let Some(worker) = workers.next_worker() {
                    return Ok(worker.clone());
                }
            }

            sleep(Duration::from_millis(10)).await;
        }
    }
}
