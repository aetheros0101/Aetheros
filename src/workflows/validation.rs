// ============================================================
// src/workflows/validation.rs
//
// Sprint 4: Tam WorkflowValidator
//
// ÖNCE: !graph.nodes.is_empty() — tek kontrol
//
// SONRA: Kapsamlı doğrulama
//   - Boş graph
//   - Root node var mı (bağımlılıksız en az 1 node)
//   - Tüm bağımlılıklar graph'ta mevcut
//   - Edge tutarlılığı
//   - Isolated node tespiti (bağlantısız node)
// ============================================================

use std::collections::HashSet;
use uuid::Uuid;

use crate::workflows::execution_graph::WorkflowExecutionGraph;
use crate::workflows::topology::WorkflowTopology;

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationError {
    EmptyGraph,
    NoRootNodes,
    UnresolvedDependency { node_id: Uuid, dep_id: Uuid },
    IsolatedNode(Uuid),
    DanglingEdge { from: Uuid, to: Uuid },
}

impl std::fmt::Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyGraph => write!(f, "Graph has no nodes"),
            Self::NoRootNodes => {
                write!(f, "Graph has no root nodes (possible cycle)")
            }
            Self::UnresolvedDependency { node_id, dep_id } => {
                write!(f, "Node {} depends on missing node {}", node_id, dep_id)
            }
            Self::IsolatedNode(id) => {
                write!(f, "Node {} is isolated (no edges)", id)
            }
            Self::DanglingEdge { from, to } => {
                write!(f, "Edge {}->{} references missing node", from, to)
            }
        }
    }
}

pub struct WorkflowValidator;

impl WorkflowValidator {
    /// Hızlı boolean kontrol (geriye dönük uyumluluk).
    pub fn validate(graph: &WorkflowExecutionGraph) -> bool {
        Self::validate_full(graph).is_ok()
    }

    /// Tam doğrulama — tüm hataları döndür.
    pub fn validate_full(graph: &WorkflowExecutionGraph) -> Result<(), Vec<ValidationError>> {
        let mut errors = Vec::new();

        // 1. Boş graph
        if graph.nodes.is_empty() {
            return Err(vec![ValidationError::EmptyGraph]);
        }

        let node_ids: HashSet<Uuid> = graph.nodes.iter().map(|n| n.id).collect();

        let edge_ids: HashSet<Uuid> = graph.edges.iter().flat_map(|e| [e.from, e.to]).collect();

        // 2. Root node kontrolü
        let roots = WorkflowTopology::roots(graph);
        if roots.is_empty() {
            errors.push(ValidationError::NoRootNodes);
        }

        // 3. Tüm bağımlılıklar graph'ta olmalı
        for node in &graph.nodes {
            for &dep in &node.dependencies {
                if !node_ids.contains(&dep) {
                    errors.push(ValidationError::UnresolvedDependency {
                        node_id: node.id,
                        dep_id: dep,
                    });
                }
            }
        }

        // 4. Dangling edge kontrolü
        for edge in &graph.edges {
            if !node_ids.contains(&edge.from) || !node_ids.contains(&edge.to) {
                errors.push(ValidationError::DanglingEdge {
                    from: edge.from,
                    to: edge.to,
                });
            }
        }

        // 5. Isolated node (tek node'lu graph hariç)
        if graph.nodes.len() > 1 {
            for node in &graph.nodes {
                if !edge_ids.contains(&node.id) {
                    errors.push(ValidationError::IsolatedNode(node.id));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}
