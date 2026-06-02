// ============================================================
// src/worker/message.rs
//
// Faz 2 Düzeltmesi:
//
// [BUG #5] Execute variant artık OwnedSemaphorePermit taşıyor.
//
//   ÖNCE: Permit dispatcher'da alınır, task kanala gönderilince
//         drop edilirdi. Task henüz çalışmıyor, sadece worker
//         kanalında bekliyordu. Gerçek backpressure yoktu.
//
//   SONRA: Permit task ile birlikte Worker'a taşınır.
//         Worker execute_task() tamamlandığında permit drop olur.
//         Semaphore ancak o zaman bir slot serbest bırakır.
//
//   Debug: OwnedSemaphorePermit Debug implementasyonu yok.
//   Bu nedenle WorkerMessage'dan derive(Debug) kaldırıldı,
//   elle implementasyon eklendi.
// ============================================================

use tokio::sync::OwnedSemaphorePermit;

use crate::task::task::TaskDefinition;
use crate::types::ids::TaskId;

pub enum WorkerMessage {
    /// [BUG #5] Permit task ile birlikte taşınır.
    /// Worker bitince drop eder → semaphore slot serbest kalır.
    Execute(TaskDefinition, OwnedSemaphorePermit),

    Cancel {
        task_id: TaskId,
    },

    Shutdown,
}

// OwnedSemaphorePermit Debug implementasyonu olmadığı için
// WorkerMessage için elle Debug yazıyoruz.
impl std::fmt::Debug for WorkerMessage {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            Self::Execute(task, _permit) => {
                f.debug_tuple("Execute")
                    .field(&task.id)
                    .finish()
            }
            Self::Cancel { task_id } => {
                f.debug_struct("Cancel")
                    .field("task_id", task_id)
                    .finish()
            }
            Self::Shutdown => write!(f, "Shutdown"),
        }
    }
}
