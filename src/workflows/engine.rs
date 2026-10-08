// ============================================================
// src/workflows/engine.rs
//
// Sprint 4: Tam WorkflowEngine
//
// ÖNCE: compile() stub — sadece compiler + validator çağrısı
//
// SONRA:
//   - compile()      → DSL/JSON → validate → ExecutionGraph
//   - from_json()    → JSON string → compile pipeline
//   - execute()      → WorkflowExecutor ile çalıştır
//   - state izleme   → WorkflowState güncellemeleri
// ============================================================

use std::sync::Arc;

use tracing::{info, warn};

use crate::ai::routing::router::ProviderRouter;
use crate::errors::runtime::RuntimeError;
use crate::runtime::api::RuntimeHandle;
use crate::workflows::compiler::{CompilerError, WorkflowCompiler, WorkflowDsl};
use crate::workflows::execution_graph::WorkflowExecutionGraph;
use crate::workflows::executor::WorkflowExecutor;
use crate::workflows::graph::WorkflowGraph;
use crate::workflows::state::WorkflowState;
use crate::workflows::validation::WorkflowValidator;

#[derive(Debug)]
pub enum WorkflowEngineError {
    Compiler(CompilerError),
    Validation(Vec<String>),
    Execution(RuntimeError),
    JsonParse(String),
}

impl std::fmt::Display for WorkflowEngineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Compiler(e) => write!(f, "Compile error: {}", e),
            Self::Validation(errs) => {
                write!(f, "Validation errors: {:?}", errs)
            }
            Self::Execution(e) => {
                write!(f, "Execution error: {:?}", e)
            }
            Self::JsonParse(e) => {
                write!(f, "JSON parse error: {}", e)
            }
        }
    }
}

pub struct WorkflowEngine;

impl WorkflowEngine {
    /// WorkflowGraph (struct) → validate → ExecutionGraph.
    /// Geriye dönük uyumluluk için korundu.
    pub fn compile(workflow: &WorkflowGraph) -> Result<WorkflowExecutionGraph, String> {
        let compiled = WorkflowCompiler::compile(workflow);

        if !WorkflowValidator::validate(&compiled) {
            return Err("invalid workflow graph".into());
        }

        Ok(compiled)
    }

    /// WorkflowDsl → validate → ExecutionGraph.
    pub fn compile_dsl(dsl: &WorkflowDsl) -> Result<WorkflowExecutionGraph, WorkflowEngineError> {
        info!(
            name = %dsl.name,
            steps = dsl.steps.len(),
            "Compiling workflow"
        );

        let graph = WorkflowCompiler::compile_dsl(dsl).map_err(WorkflowEngineError::Compiler)?;

        let validation = WorkflowValidator::validate_full(&graph);

        if let Err(errors) = validation {
            let messages: Vec<String> = errors.iter().map(|e| e.to_string()).collect();
            warn!(
                errors = ?messages,
                "Workflow validation failed"
            );
            return Err(WorkflowEngineError::Validation(messages));
        }

        info!(
            graph_id = %graph.id,
            node_count = graph.nodes.len(),
            "Workflow compiled successfully"
        );

        Ok(graph)
    }

    /// JSON string → compile → validate → ExecutionGraph.
    pub fn from_json(json: &str) -> Result<WorkflowExecutionGraph, WorkflowEngineError> {
        let dsl: WorkflowDsl = serde_json::from_str(json)
            .map_err(|e| WorkflowEngineError::JsonParse(e.to_string()))?;

        Self::compile_dsl(&dsl)
    }

    /// compile → validate → execute tam pipeline.
    pub async fn run_dsl(
        dsl: &WorkflowDsl,
        runtime: RuntimeHandle,
        ai_router: Arc<ProviderRouter>,
    ) -> Result<WorkflowState, WorkflowEngineError> {
        let graph = Self::compile_dsl(dsl)?;

        let mut executor = WorkflowExecutor::new(runtime, ai_router);

        executor
            .execute(graph)
            .await
            .map_err(WorkflowEngineError::Execution)?;

        Ok(WorkflowState::Completed)
    }

    /// JSON → execute tam pipeline.
    pub async fn run_json(
        json: &str,
        runtime: RuntimeHandle,
        ai_router: Arc<ProviderRouter>,
    ) -> Result<WorkflowState, WorkflowEngineError> {
        let dsl: WorkflowDsl = serde_json::from_str(json)
            .map_err(|e| WorkflowEngineError::JsonParse(e.to_string()))?;

        Self::run_dsl(&dsl, runtime, ai_router).await
    }
}
