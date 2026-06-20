// ============================================================
// src/orchestration/dispatcher.rs
//
// Adım B: ExecutionDispatcher gerçekleştirildi.
//   Local  (Task/Wasm)  → RuntimeHandle::submit()
//   Remote (RemoteTask) → RemoteScheduler::select_node()
//                         + HttpTransport::send(ExecuteTask)
//   Agent/Workflow/AI   → fire-and-forget log (Faz-2'de)
// ============================================================

use std::sync::Arc;

use async_trait::async_trait;
use tracing::{info, warn};

use crate::errors::runtime::RuntimeError;
use crate::orchestration::graph::{ExecutionNode, ExecutionNodeKind};
use crate::remote::cluster::ClusterState;
use crate::remote::node::NodeCapability;
use crate::remote::protocol::RemoteCommand;
use crate::remote::scheduler::RemoteScheduler;
use crate::remote::transport::RemoteTransport;
use crate::runtime::api::RuntimeHandle;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{TaskDefinition, TaskMetadata, TaskState};
use crate::types::ids::TaskId;

#[async_trait]
pub trait NodeDispatcher: Send + Sync {
    async fn dispatch(&self, node: ExecutionNode) -> Result<(), RuntimeError>;
}

pub struct ExecutionDispatcher<T>
where
    T: RemoteTransport,
{
    transport: Arc<T>,
    cluster:   Arc<ClusterState>,
    runtime:   RuntimeHandle,
}

impl<T> ExecutionDispatcher<T>
where
    T: RemoteTransport,
{
    pub fn new(
        transport: Arc<T>,
        cluster:   Arc<ClusterState>,
        runtime:   RuntimeHandle,
    ) -> Self {
        Self { transport, cluster, runtime }
    }
}

#[async_trait]
impl<T> NodeDispatcher for ExecutionDispatcher<T>
where
    T: RemoteTransport,
{
    async fn dispatch(&self, node: ExecutionNode) -> Result<(), RuntimeError> {
        match node.kind {
            // ── Local: Task / WASM ───────────────────────────────
            ExecutionNodeKind::Task | ExecutionNodeKind::Wasm => {
                let task_id = TaskId(node.id);

                let task = TaskDefinition {
                    id:             task_id,
                    parent:         None,
                    orchestration:  None,
                    priority:       TaskPriority::Normal,
                    deadline:       None,
                    timeout_ms:     30_000,
                    retry_policy:   RetryPolicy {
                        max_attempts:  3,
                        base_delay_ms: 500,
                        max_delay_ms:  30_000,
                        jitter:        true,
                    },
                    metadata: TaskMetadata {
                        labels: node.metadata.labels.clone(),
                    },
                    // ExecutionDispatcher'a hash bilinmiyor —
                    // caller'ın önceden ModuleStore'a yüklemiş olması beklenir.
                    // [0u8;32] → worker "no module" ile fail eder (beklenen).
                    wasm_module_hash: [0u8; 32],
                    entrypoint:       node.metadata.name.clone(),
                    state:            TaskState::Queued,
                    created_at:       chrono::Utc::now(),
                    updated_at:       chrono::Utc::now(),
                };

                info!(
                    node_id = %node.id,
                    kind    = "local",
                    "Dispatching local task"
                );

                self.runtime
                    .submit(task)
                    .await
                    .map_err(|_e| RuntimeError::OrchestrationFailure)?;
            }

            // ── Remote: RemoteTask ───────────────────────────────
            ExecutionNodeKind::RemoteTask => {
                let nodes     = self.cluster.nodes();
                let heartbeat = self.cluster.all_heartbeats();

                let selection = RemoteScheduler::select_node(
                    &nodes,
                    &NodeCapability::WasmExecution,
                    &heartbeat,
                );

                match selection {
                    Some(sel) => {
                        info!(
                            node_id  = %node.id,
                            target   = %sel.address,
                            load     = sel.load_score,
                            "Dispatching remote task"
                        );
                        self.transport
                            .send(RemoteCommand::ExecuteTask { task_id: node.id })
                            .await;
                    }
                    None => {
                        warn!(
                            node_id = %node.id,
                            "No healthy remote node — falling back to local"
                        );
                        // Fallback: cluster boş → local olarak çalıştır
                        self.dispatch(ExecutionNode {
                            kind: ExecutionNodeKind::Task,
                            ..node
                        })
                        .await?;
                    }
                }
            }

            // ── Agent, Workflow, AI, Plugin ──────────────────────
            // Faz-2'de AgentRuntime / WorkflowEngine entegrasyonu yapılacak.
            ExecutionNodeKind::Agent => {
                info!(node_id = %node.id, "Agent dispatch → Faz-2");
            }
            ExecutionNodeKind::Workflow => {
                info!(node_id = %node.id, "Workflow dispatch → Faz-2");
            }
            ExecutionNodeKind::AiInference => {
                info!(node_id = %node.id, "AI inference dispatch → Faz-2");
            }
            ExecutionNodeKind::Plugin => {
                info!(node_id = %node.id, "Plugin dispatch → Faz-2");
            }
        }

        Ok(())
    }
}
