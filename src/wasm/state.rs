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
pub struct WasmExecutionState {
    pub fuel_consumed: u64,
    pub memory_used: usize,
}
