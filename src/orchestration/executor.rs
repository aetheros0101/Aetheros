use std::collections::HashSet;

use futures::future::join_all;

use tokio::task::JoinHandle;

use uuid::Uuid;

use crate::errors::runtime::RuntimeError;

use crate::orchestration::coordination::ExecutionCoordinator;

use crate::orchestration::graph::{
    ExecutionGraph,
    ExecutionNode,
};

pub struct GraphExecutor;

impl GraphExecutor {
    pub async fn execute(
        graph: ExecutionGraph,
    ) -> Result<(), RuntimeError>
    {
        let mut coordinator =
            ExecutionCoordinator::new();

        let mut completed:
            HashSet<Uuid> =
                HashSet::new();

        loop {
            if coordinator.cancelled()
            {
                return Err(
                    RuntimeError::Shutdown,
                );
            }

            let ready =
                coordinator
                    .ready_nodes(
                        &graph,
                    );

            if ready.is_empty() {
                break;
            }

            let executable:
                Vec<
                    ExecutionNode,
                > = graph
                .nodes
                .iter()
                .filter(|node| {
                    ready.contains(
                        &node.id,
                    )
                })
                .cloned()
                .collect();

            let mut handles:
                Vec<
                    JoinHandle<
                        Result<
                            Uuid,
                            RuntimeError,
                        >,
                    >,
                > = Vec::new();

            for node in executable {
                let handle =
                    tokio::spawn(
                        async move {
                            Ok(node.id)
                        },
                    );

                handles.push(
                    handle,
                );
            }

            let results =
                join_all(handles)
                    .await;

            for result in results {
                let node_id =
                    result
                        .map_err(
                            |_| {
                                RuntimeError::TaskExecutionFailed {
                                    message:
                                        "execution join failure"
                                            .into(),
                                }
                            },
                        )??;

                completed.insert(
                    node_id,
                );

                coordinator
                    .mark_completed(
                        node_id,
                    );
            }
        }

        Ok(())
    }
}
