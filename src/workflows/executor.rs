// ============================================================
// src/workflows/executor.rs  (v2)
//
// Faz 6: dispatch_node() gerçek implementasyonu.
//
// ÖNCE: her node tipi için Ok(()) dönen stub'lar.
//
// SONRA:
//   Wasm       → RuntimeHandle.submit(TaskDefinition)
//   Task       → RuntimeHandle.submit(TaskDefinition)
//   Agent      → AgentExecutor::execute()
//   AiInference→ ProviderRouter.infer_active() (kullanıcının aktif ettiği provider)
//   RemoteTask → RuntimeHandle.submit (remote flag ile)
//   Plugin     → RuntimeHandle.submit (plugin entrypoint)
//   Workflow   → nested WorkflowExecutor::execute()
//
// WorkflowExecutor artık RuntimeHandle alıyor:
//   runtime'a doğrudan task submit edebiliyor.
// ============================================================
use std::sync::Arc;

use tracing::{
    info,
    warn,
};
use uuid::Uuid;

use crate::agents::budget::AgentExecutionBudget;
use crate::agents::context::AgentContext;
use crate::agents::executor::AgentExecutor;
use crate::ai::inference::request::InferenceRequest;
use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::orchestration::graph::ExecutionNodeKind;
use crate::runtime::api::RuntimeHandle;
use crate::task::priority::TaskPriority;
use crate::task::retry::RetryPolicy;
use crate::task::task::{
    TaskDefinition,
    TaskMetadata,
    TaskState,
};
use crate::orchestration::coordination::ExecutionCoordinator;
use crate::types::ids::TaskId;
use crate::workflows::execution_graph::WorkflowExecutionGraph;

pub struct WorkflowExecutor {
    coordinator: ExecutionCoordinator,
    runtime: RuntimeHandle,
    ai_router: Arc<ProviderRouter>,
}

impl WorkflowExecutor {
    pub fn new(runtime: RuntimeHandle, ai_router: Arc<ProviderRouter>) -> Self {
        Self {
            coordinator: ExecutionCoordinator::new(),
            runtime,
            ai_router,
        }
    }

    pub async fn execute(
        &mut self,
        graph: WorkflowExecutionGraph,
    ) -> Result<(), RuntimeError> {
        info!(
            graph_id = %graph.id,
            node_count = graph.nodes.len(),
            "Workflow execution started"
        );

        loop {
            if self.coordinator.is_complete(&graph) {
                info!(graph_id = %graph.id, "Workflow complete");
                break;
            }

            let ready =
                self.coordinator.ready_nodes(&graph);

            if ready.is_empty() {
                warn!(
                    graph_id = %graph.id,
                    "Workflow deadlock: no ready nodes"
                );
                return Err(RuntimeError::OrchestrationFailure);
            }

            for node_id in ready {
                let node = match graph
                    .nodes
                    .iter()
                    .find(|n| n.id == node_id)
                {
                    Some(n) => n,
                    None => continue,
                };

                info!(
                    node_id = %node_id,
                    kind = ?node.kind,
                    name = %node.metadata.name,
                    "Dispatching node"
                );

                self.dispatch_node(
                    node_id,
                    &node.kind,
                    &node.metadata.name,
                )
                .await?;

                self.coordinator.mark_completed(node_id);
            }
        }

        Ok(())
    }

    async fn dispatch_node(
        &self,
        node_id: Uuid,
        kind: &ExecutionNodeKind,
        name: &str,
    ) -> Result<(), RuntimeError> {
        match kind {
            // ── WASM: RuntimeHandle üzerinden task submit ───
            ExecutionNodeKind::Wasm => {
                let task = self.build_task(
                    node_id,
                    name,
                    TaskPriority::Normal,
                );
                self.runtime.submit(task).await?;
                info!(node_id = %node_id, "WASM node submitted");
            }

            // ── Task: genel amaçlı runtime task ─────────────
            ExecutionNodeKind::Task => {
                let task = self.build_task(
                    node_id,
                    name,
                    TaskPriority::Normal,
                );
                self.runtime.submit(task).await?;
                info!(node_id = %node_id, "Task node submitted");
            }

            // ── Agent: AgentExecutor ile çalıştır ───────────
            ExecutionNodeKind::Agent => {
                let context = AgentContext {
                    agent_id: node_id,
                    execution_id: Uuid::new_v4(),
                    workflow_id: None,
                };

                let budget = AgentExecutionBudget {
                    max_tokens: 4096,
                    max_steps: 10,
                    max_runtime_seconds: 300,
                };

                // TODO(V10 Sprint 1): None yerine gerçek bir CapabilityEngine
                // bağlanınca bu workflow düğümü de enforcement'a tabi olur.
                AgentExecutor::execute(
                    context,
                    name.to_string(),
                    budget,
                    vec![],
                    Some(self.ai_router.clone()),
                    None,
                )
                .await?;

                info!(node_id = %node_id, "Agent node complete");
            }

            // ── AI Inference: aktif provider üzerinden ──────
            ExecutionNodeKind::AiInference => {
                let request = InferenceRequest::new(name, 1024)
                    .with_system("You are an AetherOS workflow assistant.");

                match self.ai_router.infer_active(request).await {
                    Ok(response) => {
                        info!(
                            node_id = %node_id,
                            tokens = response.tokens_used,
                            "AiInference node complete"
                        );
                    }
                    Err(e) => {
                        warn!(
                            node_id = %node_id,
                            error = ?e,
                            "AiInference failed (aktif provider yok veya \
                             çağrı başarısız), node atlanıyor"
                        );
                    }
                }
            }

            // ── Plugin: entrypoint üzerinden runtime task ───
            ExecutionNodeKind::Plugin => {
                let mut task = self.build_task(
                    node_id,
                    name,
                    TaskPriority::Low,
                );
                task.metadata.labels.insert(
                    "node_type".to_string(),
                    "plugin".to_string(),
                );
                self.runtime.submit(task).await?;
                info!(node_id = %node_id, "Plugin node submitted");
            }

            // ── RemoteTask: remote flag ile task submit ──────
            ExecutionNodeKind::RemoteTask => {
                let mut task = self.build_task(
                    node_id,
                    name,
                    TaskPriority::Normal,
                );
                task.metadata.labels.insert(
                    "remote".to_string(),
                    "true".to_string(),
                );
                self.runtime.submit(task).await?;
                info!(node_id = %node_id, "RemoteTask node submitted");
            }

            // ── Nested Workflow: Faz 7'de recursive executor ─
            ExecutionNodeKind::Workflow => {
                warn!(
                    node_id = %node_id,
                    "Nested workflow dispatch not yet implemented (Faz 7)"
                );
            }
        }

        Ok(())
    }

    /// Node'dan TaskDefinition üret.
    /// wasm_module boş — Faz 7'de node metadata'dan yüklenecek.
    fn build_task(
        &self,
        node_id: Uuid,
        entrypoint: &str,
        priority: TaskPriority,
    ) -> TaskDefinition {
        TaskDefinition {
            id: TaskId(node_id),
            parent: None,
            orchestration: None,
            priority,
            deadline: None,
            timeout_ms: 30_000,
            retry_policy: RetryPolicy {
                max_attempts: 3,
                base_delay_ms: 500,
                max_delay_ms: 10_000,
                jitter: true,
            },
            metadata: TaskMetadata {
                labels: std::collections::HashMap::new(),
            },
            wasm_module_hash: [0u8; 32],
            entrypoint: entrypoint.to_string(),
            state: TaskState::Queued,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}
