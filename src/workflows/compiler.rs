// ============================================================
// src/workflows/compiler.rs
//
// Sprint 4: Tam WorkflowCompiler
//
// ÖNCE: Sadece struct → struct dönüşüm, eksik alanlar.
//
// SONRA:
//   - WorkflowDsl (JSON/YAML tanım) → WorkflowGraph
//   - WorkflowGraph → ExecutionGraph (tam alanlarla)
//   - Döngüsel bağımlılık tespiti (DFS)
//   - Duplicate node ID tespiti
//   - Orphan edge tespiti
//   - Node türü → ExecutionNodeKind mapping
// ============================================================

use std::collections::{HashMap, HashSet};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::orchestration::graph::{
    ExecutionEdge, ExecutionGraph, ExecutionMetadata, ExecutionNode, ExecutionNodeKind,
};
use crate::workflows::execution_graph::WorkflowExecutionGraph;
use crate::workflows::graph::WorkflowGraph;

// ── DSL: Kullanıcı JSON/YAML tanımı ─────────────────────

/// Kullanıcının yazdığı workflow tanımı.
///
/// JSON örneği:
/// ```json
/// {
///   "name": "my-workflow",
///   "steps": [
///     { "id": "step-1", "name": "fetch", "type": "wasm",
///       "entrypoint": "fetch_data", "depends_on": [] },
///     { "id": "step-2", "name": "process", "type": "agent",
///       "entrypoint": "process", "depends_on": ["step-1"] }
///   ]
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowDsl {
    pub name: String,
    pub version: Option<u64>,
    pub steps: Vec<StepDsl>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDsl {
    /// Bağımlılıklarda referans için kullanılan isim.
    pub id: String,
    pub name: String,
    /// "wasm" | "agent" | "ai" | "task" | "plugin" | "remote"
    #[serde(rename = "type")]
    pub kind: String,
    pub entrypoint: Option<String>,
    #[serde(default)]
    pub depends_on: Vec<String>,
    #[serde(default)]
    pub retryable: bool,
    #[serde(default)]
    pub labels: HashMap<String, String>,
}

// ── Hata ─────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum CompilerError {
    EmptyWorkflow,
    DuplicateStepId(String),
    UnknownDependency { step: String, dep: String },
    CyclicDependency(Vec<String>),
    UnknownNodeKind { step: String, kind: String },
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyWorkflow => {
                write!(f, "Workflow has no steps")
            }
            Self::DuplicateStepId(id) => {
                write!(f, "Duplicate step id: {}", id)
            }
            Self::UnknownDependency { step, dep } => {
                write!(f, "Step '{}' depends on unknown step '{}'", step, dep)
            }
            Self::CyclicDependency(cycle) => {
                write!(f, "Cyclic dependency: {:?}", cycle)
            }
            Self::UnknownNodeKind { step, kind } => {
                write!(f, "Step '{}' has unknown type '{}'", step, kind)
            }
        }
    }
}

// ── Compiler ─────────────────────────────────────────────

pub struct WorkflowCompiler;

impl WorkflowCompiler {
    /// WorkflowDsl (JSON/YAML kaynaklı) → ExecutionGraph.
    pub fn compile_dsl(dsl: &WorkflowDsl) -> Result<WorkflowExecutionGraph, CompilerError> {
        if dsl.steps.is_empty() {
            return Err(CompilerError::EmptyWorkflow);
        }

        // Step id → UUID mapping
        let id_map: HashMap<String, Uuid> = dsl
            .steps
            .iter()
            .map(|s| (s.id.clone(), Uuid::new_v4()))
            .collect();

        // Duplicate ID kontrolü
        let mut seen = HashSet::new();
        for step in &dsl.steps {
            if !seen.insert(&step.id) {
                return Err(CompilerError::DuplicateStepId(step.id.clone()));
            }
        }

        // Node'ları oluştur
        let mut nodes = Vec::new();
        for step in &dsl.steps {
            let kind =
                Self::map_kind(&step.kind).ok_or_else(|| CompilerError::UnknownNodeKind {
                    step: step.id.clone(),
                    kind: step.kind.clone(),
                })?;

            // Bağımlılıkları UUID'e çevir
            let mut deps = Vec::new();
            for dep_id in &step.depends_on {
                let dep_uuid = id_map.get(dep_id).copied().ok_or_else(|| {
                    CompilerError::UnknownDependency {
                        step: step.id.clone(),
                        dep: dep_id.clone(),
                    }
                })?;
                deps.push(dep_uuid);
            }

            let mut labels = step.labels.clone();
            labels.insert("step_id".to_string(), step.id.clone());
            if let Some(ep) = &step.entrypoint {
                labels.insert("entrypoint".to_string(), ep.clone());
            }

            nodes.push(ExecutionNode {
                id: *id_map.get(&step.id).unwrap(),
                kind,
                metadata: ExecutionMetadata {
                    name: step.name.clone(),
                    labels,
                },
                correlation_id: Some(step.id.clone()),
                retryable: step.retryable,
                dependencies: deps,
            });
        }

        // Edge'leri üret
        let edges: Vec<ExecutionEdge> = nodes
            .iter()
            .flat_map(|node| {
                node.dependencies.iter().map(move |dep| ExecutionEdge {
                    from: *dep,
                    to: node.id,
                })
            })
            .collect();

        let graph = ExecutionGraph {
            id: Uuid::new_v4(),
            version: dsl.version.unwrap_or(1),
            nodes,
            edges,
        };

        // Döngüsel bağımlılık kontrolü
        Self::detect_cycles(&graph)?;

        Ok(graph)
    }

    /// WorkflowGraph (eski struct format) → ExecutionGraph.
    /// Geriye dönük uyumluluk.
    pub fn compile(graph: &WorkflowGraph) -> WorkflowExecutionGraph {
        let nodes = graph
            .nodes
            .iter()
            .map(|node| ExecutionNode {
                id: node.id,
                kind: ExecutionNodeKind::Task,
                metadata: ExecutionMetadata {
                    name: node.id.to_string(),
                    labels: Default::default(),
                },
                correlation_id: None,
                retryable: true,
                dependencies: node.dependencies.clone(),
            })
            .collect::<Vec<_>>();

        let edges = nodes
            .iter()
            .flat_map(|node| {
                node.dependencies.iter().map(move |dep| ExecutionEdge {
                    from: *dep,
                    to: node.id,
                })
            })
            .collect();

        ExecutionGraph {
            id: Uuid::new_v4(),
            version: 1,
            nodes,
            edges,
        }
    }

    /// JSON string → ExecutionGraph.
    pub fn from_json(json: &str) -> Result<WorkflowExecutionGraph, String> {
        let dsl: WorkflowDsl =
            serde_json::from_str(json).map_err(|e| format!("JSON parse error: {}", e))?;
        Self::compile_dsl(&dsl).map_err(|e| e.to_string())
    }

    // ── Yardımcılar ───────────────────────────────────────

    fn map_kind(kind: &str) -> Option<ExecutionNodeKind> {
        match kind.to_lowercase().as_str() {
            "wasm" => Some(ExecutionNodeKind::Wasm),
            "agent" => Some(ExecutionNodeKind::Agent),
            "ai" | "ai_inference" | "aiinference" => Some(ExecutionNodeKind::AiInference),
            "task" => Some(ExecutionNodeKind::Task),
            "plugin" => Some(ExecutionNodeKind::Plugin),
            "remote" | "remote_task" | "remotetask" => Some(ExecutionNodeKind::RemoteTask),
            "workflow" => Some(ExecutionNodeKind::Workflow),
            _ => None,
        }
    }

    /// DFS tabanlı döngü tespiti.
    fn detect_cycles(graph: &ExecutionGraph) -> Result<(), CompilerError> {
        let mut visited: HashSet<Uuid> = HashSet::new();
        let mut stack: HashSet<Uuid> = HashSet::new();
        let mut path: Vec<String> = Vec::new();

        for node in &graph.nodes {
            if !visited.contains(&node.id)
                && let Some(cycle) = Self::dfs(node.id, graph, &mut visited, &mut stack, &mut path)
            {
                return Err(CompilerError::CyclicDependency(cycle));
            }
        }

        Ok(())
    }

    fn dfs(
        current: Uuid,
        graph: &ExecutionGraph,
        visited: &mut HashSet<Uuid>,
        stack: &mut HashSet<Uuid>,
        path: &mut Vec<String>,
    ) -> Option<Vec<String>> {
        visited.insert(current);
        stack.insert(current);

        let name = graph
            .nodes
            .iter()
            .find(|n| n.id == current)
            .map(|n| n.metadata.name.clone())
            .unwrap_or_else(|| current.to_string());

        path.push(name);

        if let Some(node) = graph.nodes.iter().find(|n| n.id == current) {
            for &dep in &node.dependencies {
                if !visited.contains(&dep) {
                    if let Some(cycle) = Self::dfs(dep, graph, visited, stack, path) {
                        return Some(cycle);
                    }
                } else if stack.contains(&dep) {
                    return Some(path.clone());
                }
            }
        }

        stack.remove(&current);
        path.pop();
        None
    }
}
