use crate::compilation::task_graph::{TaskGraph, TaskRole};
use crate::constraints::Constraints;
use crate::errors::{IntentError, Result};
use crate::requirement::RequirementSet;

#[derive(Debug, Clone, Default)]
pub struct ValidationReport {
    pub ok: bool,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}

pub fn validate_requirement_set(req: &RequirementSet) -> ValidationReport {
    let mut r = ValidationReport {
        ok: true,
        ..Default::default()
    };
    if req.summary.trim().is_empty() {
        r.ok = false;
        r.errors.push("summary empty".into());
    }
    if req.functional.is_empty() {
        r.ok = false;
        r.errors.push("no functional requirements".into());
    }
    if req.acceptance.is_empty() {
        r.warnings.push("no acceptance criteria".into());
    }
    if !req.unknowns.items.is_empty() {
        r.warnings.push(format!("{} unknowns", req.unknowns.items.len()));
    }
    r
}

/// Gereksinim kümesindeki alan etiketlerini `Constraints`'e karşı denetler.
pub fn validate_domains(req: &RequirementSet, constraints: &Constraints) -> Result<()> {
    for tag in &req.domain_tags {
        if !constraints.allows_domain(tag) {
            return Err(IntentError::Constraint(format!("domain `{tag}` not allowed")));
        }
    }
    Ok(())
}

pub fn validate_graph(graph: &TaskGraph, constraints: &Constraints) -> Result<ValidationReport> {
    let mut r = ValidationReport {
        ok: true,
        ..Default::default()
    };
    if graph.nodes.is_empty() {
        return Err(IntentError::Validation("empty task graph".into()));
    }
    graph.detect_cycle()?;

    // Kısıtlar (önceden hiçbiri uygulanmıyordu).
    let depth = graph.hierarchy_depth();
    if depth > constraints.max_depth {
        return Err(IntentError::Constraint(format!(
            "graph depth {depth} exceeds max_depth {}",
            constraints.max_depth
        )));
    }
    let width = graph.max_parallel_width()?;
    if width > constraints.max_parallel_branches {
        return Err(IntentError::Constraint(format!(
            "parallel width {width} exceeds max_parallel_branches {}",
            constraints.max_parallel_branches
        )));
    }
    for n in graph.nodes.values() {
        for tag in &n.domain_tags {
            if !constraints.allows_domain(tag) {
                return Err(IntentError::Constraint(format!(
                    "domain `{tag}` not allowed (task `{}`)",
                    n.title
                )));
            }
        }
    }

    let has_testing = graph.nodes.values().any(|n| n.role == TaskRole::Testing);
    let has_ver = graph.nodes.values().any(|n| n.role == TaskRole::Verification);
    let is_docs = graph.nodes.values().any(|n| n.role == TaskRole::Edit);

    if constraints.require_testing && !has_testing && !is_docs {
        r.warnings.push("no testing node".into());
    }
    if constraints.require_verification && !has_ver {
        r.ok = false;
        r.errors.push("verification node required".into());
    }
    if !r.ok {
        return Err(IntentError::Validation(r.errors.join("; ")));
    }
    Ok(r)
}
