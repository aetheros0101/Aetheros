use serde::{
    Deserialize,
    Serialize,
};

use crate::task::task::{
    TaskDefinition,
    TaskState,
};
use crate::types::timestamps::Timestamp;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct PersistedTask {
    pub task: TaskDefinition,
    pub created_at: Timestamp,
    pub updated_at: Timestamp,
    pub attempts: u32,

    /// Son hata mesajı (WasmError::to_string()).
    /// Sadece TaskState::Failed durumunda dolu olur.
    /// #[serde(default)] — eski msgpack kayıtlarında bu alan
    /// yoktu, deserialize sırasında None'a düşer (geriye uyumlu).
    #[serde(default)]
    pub last_error: Option<String>,
}

impl PersistedTask {
    pub fn is_terminal(
        &self,
    ) -> bool {
        matches!(
            self.task.state,
            TaskState::Completed
                | TaskState::Failed
                | TaskState::Cancelled
        )
    }
}
