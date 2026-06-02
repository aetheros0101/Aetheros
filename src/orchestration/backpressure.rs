use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Copy,
    Serialize,
    Deserialize,
)]
pub struct BackpressurePolicy {
    pub max_queue_depth:
        usize,

    pub reject_threshold:
        usize,
}
