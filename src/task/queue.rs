// ============================================================
// src/task/queue.rs
//
// Optimizasyon #3: 4-Tier VecDeque Queue
//
// ÖNCE: Tek Mutex<BinaryHeap<QueueItem>>
//   - push: lock + O(log n) sift-up
//   - pop:  lock + O(log n) sift-down
//   - Her push/pop tüm kuyruğu kilitliyor
//   - Çift lock: Mutex<BinaryHeap> + Mutex<u64>
//
// SONRA: 4 × Mutex<VecDeque<TaskDefinition>>
//   - push: tier seç → lock → push_back O(1)
//   - pop:  critical → high → normal → low (öncelik sıralı)
//          İlk dolu tier'dan pop_front O(1)
//   - Lock contention 4'e bölünür:
//     Critical push ile Low push çakışmaz
//   - sequence: AtomicU64 → tek lock kaldırıldı
//   - QueueItem wrapper kaldırıldı → zero allocation
//
// BENCHMARK HEDEFİ:
//   push_pop_normal: 4.5µs → ~1µs
//   priority_ordering: 15µs → ~3µs
//   lock contention: 4x azalma
// ============================================================
use std::sync::atomic::AtomicU64;
use std::collections::VecDeque;

use tokio::sync::{
    Mutex,
    Notify,
};

use crate::errors::task::TaskError;
use crate::task::priority::TaskPriority;
use crate::task::task::TaskDefinition;

/// Her tier bir öncelik seviyesi için ayrı VecDeque.
/// VecDeque: FIFO — aynı öncelikte sıra korunur.
struct Tier {
    queue: Mutex<VecDeque<TaskDefinition>>,
}

impl Tier {
    fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    async fn push(&self, task: TaskDefinition) {
        self.queue.lock().await.push_back(task);
    }

    async fn pop(&self) -> Option<TaskDefinition> {
        self.queue.lock().await.pop_front()
    }

    async fn len(&self) -> usize {
        self.queue.lock().await.len()
    }
}

pub struct PriorityTaskQueue {
    critical: Tier,
    high: Tier,
    normal: Tier,
    low: Tier,
    notify: Notify,
    /// Lock-free sequence — FIFO garantisi için
    /// (şu an tier içinde VecDeque FIFO sağlıyor,
    ///  sequence ileride cross-tier FIFO için kullanılabilir)
    _sequence: AtomicU64,
}

impl PriorityTaskQueue {
    pub fn new() -> Self {
        Self {
            critical: Tier::new(),
            high:     Tier::new(),
            normal:   Tier::new(),
            low:      Tier::new(),
            notify:   Notify::new(),
            _sequence: AtomicU64::new(0),
        }
    }

    /// Task'ı önceliğine göre doğru tier'a ekle.
    ///
    /// O(1): lock + VecDeque::push_back
    /// Sadece ilgili tier kilitlenir — diğer tier'lar serbest.
    pub async fn push(
        &self,
        task: TaskDefinition,
    ) -> Result<(), TaskError> {
        match task.priority {
            TaskPriority::Critical => {
                self.critical.push(task).await;
            }
            TaskPriority::High => {
                self.high.push(task).await;
            }
            TaskPriority::Normal => {
                self.normal.push(task).await;
            }
            TaskPriority::Low => {
                self.low.push(task).await;
            }
        }

        // Bekleyen pop() varsa uyandır
        self.notify.notify_one();

        Ok(())
    }

    /// En yüksek öncelikli tier'dan task al.
    ///
    /// O(1): tier sırası sabit (4 kontrol) + pop_front
    ///
    /// Sıralama: Critical → High → Normal → Low
    /// Boşsa: Notify bekle (spin yok)
    pub async fn pop(
        &self,
    ) -> Result<TaskDefinition, TaskError> {
        loop {
            // Critical önce kontrol et
            if let Some(task) = self.critical.pop().await {
                return Ok(task);
            }

            if let Some(task) = self.high.pop().await {
                return Ok(task);
            }

            if let Some(task) = self.normal.pop().await {
                return Ok(task);
            }

            if let Some(task) = self.low.pop().await {
                return Ok(task);
            }

            // Tüm tier'lar boş — push gelene kadar bekle
            self.notify.notified().await;
        }
    }

    pub async fn len(&self) -> usize {
        self.critical.len().await
            + self.high.len().await
            + self.normal.len().await
            + self.low.len().await
    }

    pub async fn is_empty(&self) -> bool {
        self.len().await == 0
    }

    /// Tier bazlı dağılım — monitoring için.
    pub async fn distribution(
        &self,
    ) -> [usize; 4] {
        [
            self.critical.len().await,
            self.high.len().await,
            self.normal.len().await,
            self.low.len().await,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::helpers::make_task;

    #[tokio::test]
    async fn critical_pops_before_low() {
        let q = PriorityTaskQueue::new();
        q.push(make_task(TaskPriority::Low, "low")).await.unwrap();
        q.push(make_task(TaskPriority::Critical, "critical"))
            .await
            .unwrap();

        let first = q.pop().await.unwrap();
        assert_eq!(first.entrypoint, "critical");
    }

    #[tokio::test]
    async fn all_priorities_correct_order() {
        let q = PriorityTaskQueue::new();
        q.push(make_task(TaskPriority::Normal, "normal")).await.unwrap();
        q.push(make_task(TaskPriority::Low, "low")).await.unwrap();
        q.push(make_task(TaskPriority::Critical, "critical")).await.unwrap();
        q.push(make_task(TaskPriority::High, "high")).await.unwrap();

        let order: Vec<_> = vec![
            q.pop().await.unwrap().entrypoint,
            q.pop().await.unwrap().entrypoint,
            q.pop().await.unwrap().entrypoint,
            q.pop().await.unwrap().entrypoint,
        ];

        assert_eq!(
            order,
            vec!["critical", "high", "normal", "low"]
        );
    }

    #[tokio::test]
    async fn same_priority_is_fifo() {
        let q = PriorityTaskQueue::new();
        q.push(make_task(TaskPriority::Normal, "first")).await.unwrap();
        q.push(make_task(TaskPriority::Normal, "second")).await.unwrap();
        q.push(make_task(TaskPriority::Normal, "third")).await.unwrap();

        assert_eq!(q.pop().await.unwrap().entrypoint, "first");
        assert_eq!(q.pop().await.unwrap().entrypoint, "second");
        assert_eq!(q.pop().await.unwrap().entrypoint, "third");
    }

    #[tokio::test]
    async fn distribution_tracks_tiers() {
        let q = PriorityTaskQueue::new();
        q.push(make_task(TaskPriority::Critical, "c")).await.unwrap();
        q.push(make_task(TaskPriority::High, "h1")).await.unwrap();
        q.push(make_task(TaskPriority::High, "h2")).await.unwrap();
        q.push(make_task(TaskPriority::Low, "l")).await.unwrap();

        let dist = q.distribution().await;
        assert_eq!(dist, [1, 2, 0, 1]); // [crit, high, normal, low]
    }
}
