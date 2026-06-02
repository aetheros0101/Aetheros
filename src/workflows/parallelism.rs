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
pub struct ParallelismPolicy {
    pub max_parallel_nodes:
        usize,
}
