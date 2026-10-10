//! RequirementSet → expand tasks → strategy optimize → TaskGraph.

use crate::compilation::dependency::DependencyKind;
use crate::compilation::requirement_tasks::expand_requirement_set;
use crate::compilation::strategy::CompileStrategy;
use crate::compilation::task_graph::{TaskGraph, TaskRole};
use crate::constraints::Constraints;
use crate::errors::Result;
use crate::requirement::RequirementSet;
use uuid::Uuid;

#[derive(Default)]
pub struct TaskGraphCompiler {
    pub constraints: Constraints,
}

impl TaskGraphCompiler {
    pub fn new(constraints: Constraints) -> Self {
        Self { constraints }
    }

    pub fn compile(&self, req: &RequirementSet) -> Result<TaskGraph> {
        crate::validation::validate_domains(req, &self.constraints)?;
        let strategy = CompileStrategy::select(req);
        let specs = expand_requirement_set(req);
        let mut specs = strategy.optimize(specs);
        specs.sort_by_key(|s| (s.seq, s.node.title.clone()));

        let mut g = TaskGraph::new();
        let mut prev_by_source: std::collections::HashMap<Option<Uuid>, Uuid> =
            std::collections::HashMap::new();
        let mut last_global: Option<Uuid> = None;
        let mut analyze_id: Option<Uuid> = None;
        let mut ids = Vec::new();

        for spec in specs {
            let source = spec.node.source_requirement;
            let role = spec.node.role;
            let id = g.add_node(spec.node);

            if role == TaskRole::Analyze {
                analyze_id = Some(id);
                g.root = Some(id);
            }
            if role != TaskRole::Analyze
                && let Some(a) = analyze_id
                && let Some(n) = g.nodes.get_mut(&id)
            {
                n.parent = Some(a);
            }

            if let Some(prev) = prev_by_source.get(&source) {
                g.add_edge(*prev, id, DependencyKind::FinishToStart)?;
            } else if let Some(prev) = last_global {
                if source.is_some() {
                    g.add_edge(prev, id, DependencyKind::Soft)?;
                } else if role != TaskRole::Analyze {
                    g.add_edge(prev, id, DependencyKind::FinishToStart)?;
                }
            }

            prev_by_source.insert(source, id);
            last_global = Some(id);
            ids.push(id);
        }

        if g.root.is_none() {
            g.root = ids.first().copied();
        }
        g.detect_cycle()?;
        Ok(g)
    }
}
