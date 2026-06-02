use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct FanOutPolicy {
    pub parallelism:
        usize,

    pub fail_fast:
        bool,
}
