//! Existing AetherOS Task Layer sınırı.
//!
//! Intent crate Task Layer iç yapısını bilmez; host adaptör yazar.

use crate::compilation::task_graph::{CompiledTask, TaskGraph};
use crate::errors::Result;

/// Host implement eder: CompiledTask → native Task.
pub trait TaskLayerAdapter: Send + Sync {
    type TaskId;
    type Error: std::fmt::Display;

    fn import_compiled(
        &self,
        tasks: &[CompiledTask],
    ) -> std::result::Result<Vec<Self::TaskId>, Self::Error>;
}

/// Intent tarafı yardımcısı: graph → compiled → adapter.
pub fn export_to_task_layer<A: TaskLayerAdapter>(
    graph: &TaskGraph,
    adapter: &A,
) -> Result<Vec<A::TaskId>>
where
    A::Error: std::fmt::Display,
{
    let compiled = graph.to_compiled_tasks()?;
    adapter
        .import_compiled(&compiled)
        .map_err(|e| crate::errors::IntentError::Compile(e.to_string()))
}

/// Test adaptörü — id olarak CompiledTask.id döner.
#[derive(Debug, Default)]
pub struct IdentityTaskAdapter;

impl TaskLayerAdapter for IdentityTaskAdapter {
    type TaskId = uuid::Uuid;
    type Error = String;

    fn import_compiled(
        &self,
        tasks: &[CompiledTask],
    ) -> std::result::Result<Vec<Self::TaskId>, Self::Error> {
        Ok(tasks.iter().map(|t| t.id).collect())
    }
}
