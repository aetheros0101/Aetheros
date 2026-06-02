use std::time::Duration;

#[derive(
    Debug,
    Clone,
)]
pub struct AiRetryPolicy {
    pub max_attempts:
        usize,

    pub backoff:
        Duration,
}
