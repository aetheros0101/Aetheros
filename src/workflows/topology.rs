use std::collections::{HashMap, HashSet};

use uuid::Uuid;

use crate::workflows::execution_graph::WorkflowExecutionGraph;

pub struct WorkflowTopology;

impl WorkflowTopology {
    pub fn roots(graph: &WorkflowExecutionGraph) -> Vec<Uuid> {
        graph
            .nodes
            .iter()
            .filter(|node| node.dependencies.is_empty())
            .map(|node| node.id)
            .collect()
    }

    pub fn leaves(graph: &WorkflowExecutionGraph) -> Vec<Uuid> {
        let dependency_set: HashSet<Uuid> = graph.edges.iter().map(|edge| edge.from).collect();

        graph
            .nodes
            .iter()
            .filter(|node| !dependency_set.contains(&node.id))
            .map(|node| node.id)
            .collect()
    }

    pub fn adjacency(graph: &WorkflowExecutionGraph) -> HashMap<Uuid, Vec<Uuid>> {
        let mut adjacency = HashMap::new();

        for edge in &graph.edges {
            adjacency
                .entry(edge.from)
                .or_insert_with(Vec::new)
                .push(edge.to);
        }

        adjacency
    }
}
