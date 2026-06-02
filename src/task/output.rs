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
pub struct TaskOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub return_value: Vec<u8>,
}
