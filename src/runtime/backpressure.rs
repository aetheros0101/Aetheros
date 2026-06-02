use std::sync::Arc;

use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct BackpressureController {
    permits: Arc<Semaphore>,
}

impl BackpressureController {
    pub fn new(
        max_inflight: usize,
    ) -> Self {
        Self {
            permits: Arc::new(
                Semaphore::new(
                    max_inflight,
                ),
            ),
        }
    }

    pub async fn acquire(
        &self,
    ) -> tokio::sync::OwnedSemaphorePermit
    {
        self.permits
            .clone()
            .acquire_owned()
            .await
            .expect(
                "backpressure semaphore closed",
            )
    }

    pub fn available(
        &self,
    ) -> usize {
        self.permits
            .available_permits()
    }
}
