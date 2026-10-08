// ============================================================
// src/worker/worker.rs  (v2)
//
// Faz 2 Düzeltmeleri:
//
// [BUG #3] Retry mekanizması entegre edildi.
//   Err branch artık RetryPolicy'e sorar:
//   - Retryable + attempts < max → TaskState::Retrying
//                                → delay bekle
//                                → attempts artır (persistence)
//                                → retry_queue'ya geri koy
//   - Permanent veya limit doldu → TaskState::Failed (Faz 1'den)
//
// [BUG #5] OwnedSemaphorePermit execute_task() içinde yaşar.
//   Fonksiyon dönerken (Ok/Err/Cancel) permit drop olur.
//   Semaphore ancak o zaman slot serbest bırakır.
// ============================================================

use std::sync::Arc;

use tokio::sync::{OwnedSemaphorePermit, mpsc};
use tokio::time::sleep;

use crate::errors::runtime::RuntimeError;
use crate::events::bus::{EventBus, SystemEvent};
use crate::events::task::TaskEvent;
use crate::persistence::engine::PersistenceEngine;
use crate::task::deadline::deadline_expired;
use crate::task::queue::PriorityTaskQueue;
use crate::task::task::{TaskDefinition, TaskState};
use crate::worker::cancellation::CancellationRegistry;
use crate::worker::executor::WorkerExecutor;
use crate::worker::message::WorkerMessage;
use crate::worker::state::WorkerState;

pub struct Worker {
    state: WorkerState,
    receiver: mpsc::Receiver<WorkerMessage>,
    executor: Arc<WorkerExecutor>,
    events: EventBus,
    cancellation: CancellationRegistry,
    persistence: Arc<PersistenceEngine>,
    /// [BUG #3] Retry için doğrudan queue referansı.
    /// Scheduler aracılığıyla değil, doğrudan push()
    /// yapılır — gereksiz arc zinciri kurulmaz.
    retry_queue: Arc<PriorityTaskQueue>,
}

impl Worker {
    pub fn new(
        receiver: mpsc::Receiver<WorkerMessage>,
        executor: Arc<WorkerExecutor>,
        events: EventBus,
        persistence: Arc<PersistenceEngine>,
        retry_queue: Arc<PriorityTaskQueue>,
    ) -> Self {
        Self {
            state: WorkerState::Starting,
            receiver,
            executor,
            events,
            cancellation: CancellationRegistry::new(),
            persistence,
            retry_queue,
        }
    }

    pub async fn run(mut self) -> Result<(), RuntimeError> {
        self.state = WorkerState::Idle;

        while let Some(message) = self.receiver.recv().await {
            match message {
                // [BUG #5] Permit execute_task'a taşınıyor.
                // Fonksiyon bitince drop → semaphore slot açılır.
                WorkerMessage::Execute(task, permit) => {
                    self.execute_task(task, permit).await?;
                }

                WorkerMessage::Cancel { task_id } => {
                    self.handle_cancellation(task_id).await?;
                }

                WorkerMessage::Shutdown => {
                    self.state = WorkerState::Stopping;
                    break;
                }
            }
        }

        self.state = WorkerState::Stopped;

        Ok(())
    }

    async fn execute_task(
        &mut self,
        task: TaskDefinition,
        // [BUG #5] Permit burada yaşar. return/drop anında
        // semaphore slot serbest kalır.
        _permit: OwnedSemaphorePermit,
    ) -> Result<(), RuntimeError> {
        let task_id = task.id;

        // ── Deadline kontrolü ────────────────────────────────
        // Deadline dolmuşsa task artık çalıştırılmamalı.
        // Hâlâ kalan deneme hakkı varsa Retrying → queue;
        // yoksa ya da retry policy izin vermiyorsa Failed.
        if deadline_expired(&task) {
            let attempts = self
                .persistence
                .load_task(&task_id)
                .ok()
                .flatten()
                .map(|p| p.attempts)
                .unwrap_or(0);

            if attempts < task.retry_policy.max_attempts {
                self.increment_attempts(task_id);
                self.update_state(task_id, TaskState::Retrying);
                let mut retried = task;
                retried.state = TaskState::Queued;
                let _ = self.retry_queue.push(retried).await;
            } else {
                self.fail_task(task_id, "deadline aşıldı, retry hakkı tükendi".to_string());
                self.events
                    .publish(SystemEvent::Task(TaskEvent::TaskFailed { task_id }));
            }
            return Ok(());
        }
        // ── Executing ────────────────────────────────────────
        self.state = WorkerState::Busy;
        self.update_state(task_id, TaskState::Executing);

        self.events
            .publish(SystemEvent::Task(TaskEvent::TaskStarted { task_id }));

        let token = self.cancellation.register(task_id);
        let execution = self.executor.execute(task.clone());

        // ── Execution + Cancel yarışı ─────────────────────────
        let result = tokio::select! {
            result = execution => result,

            _ = token.cancelled() => {
                self.cancellation.remove(&task_id);
                self.update_state(task_id, TaskState::Cancelled);
                self.events.publish(SystemEvent::Task(
                    TaskEvent::TaskCancelled { task_id },
                ));
                self.state = WorkerState::Idle;
                // _permit burada drop → slot serbest ✓
                return Ok(());
            }
        };

        self.cancellation.remove(&task_id);

        // ── Sonuç ─────────────────────────────────────────────
        match result {
            Ok(_) => {
                self.update_state(task_id, TaskState::Completed);
                self.events
                    .publish(SystemEvent::Task(TaskEvent::TaskCompleted { task_id }));
            }

            Err(ref wasm_err) => {
                // [BUG #3] Retry kararı
                let attempts = self
                    .persistence
                    .load_task(&task_id)
                    .ok()
                    .flatten()
                    .map(|p| p.attempts)
                    .unwrap_or(0);

                if task.retry_policy.should_retry(attempts, wasm_err) {
                    // ── Retry ────────────────────────────────
                    let delay = task.retry_policy.next_delay(attempts);

                    self.update_state(task_id, TaskState::Retrying);
                    self.increment_attempts(task_id);

                    self.events
                        .publish(SystemEvent::Task(TaskEvent::TaskRetried {
                            task_id,
                            attempt: attempts + 1,
                        }));

                    // _permit burada drop → semaphore slot serbest ✓
                    // Retry task yeni bir permit alarak çalışır.
                    drop(_permit);

                    sleep(delay).await;

                    let mut retried = task;
                    retried.state = TaskState::Queued;
                    let _ = self.retry_queue.push(retried).await;

                    self.state = WorkerState::Idle;
                    return Ok(());
                } else {
                    // ── Kalıcı başarısızlık ──────────────────
                    // wasm_err.to_string() → WasmError'ın thiserror
                    // #[error("...")] mesajı (örn. "missing entrypoint",
                    // "invalid module: module not found in store").
                    self.fail_task(task_id, wasm_err.to_string());
                    self.events
                        .publish(SystemEvent::Task(TaskEvent::TaskFailed { task_id }));
                }
            }
        }

        self.state = WorkerState::Idle;
        // _permit burada drop → slot serbest ✓
        Ok(())
    }

    async fn handle_cancellation(
        &mut self,
        task_id: crate::types::ids::TaskId,
    ) -> Result<(), RuntimeError> {
        self.cancellation.cancel(&task_id);
        Ok(())
    }

    fn update_state(&self, task_id: crate::types::ids::TaskId, state: TaskState) {
        if let Err(_e) = self.persistence.update_task_state(&task_id, state) {
            // Faz 3: tracing::warn!
        }
    }

    /// Task'ı Failed yap + gerçek hata mesajını persist et.
    /// UI bu mesajı TaskStatusResponse.error_message'da gösterir.
    fn fail_task(&self, task_id: crate::types::ids::TaskId, error: String) {
        if let Err(_e) = self.persistence.set_task_failed(&task_id, error) {
            // Faz 3: tracing::warn!
        }
    }

    /// Persistence'daki attempts sayacını artır.
    fn increment_attempts(&self, task_id: crate::types::ids::TaskId) {
        if let Ok(Some(mut persisted)) = self.persistence.load_task(&task_id) {
            persisted.attempts += 1;
            persisted.updated_at = chrono::Utc::now();
            let _ = self.persistence.persist_task(&persisted);
        }
    }
}
