// ============================================================
// src/tests/workflow_tests.rs
//
// SPRINT 4 — Workflow Compiler & Validator Testleri
// ============================================================

use crate::workflows::compiler::{
    CompilerError,
    StepDsl,
    WorkflowCompiler,
    WorkflowDsl,
};
use crate::workflows::validation::WorkflowValidator;
use crate::workflows::topology::WorkflowTopology;

// ── Yardımcılar ───────────────────────────────────────────

fn simple_dsl(steps: Vec<StepDsl>) -> WorkflowDsl {
    WorkflowDsl {
        name: "test-workflow".to_string(),
        version: Some(1),
        steps,
    }
}

fn step(
    id: &str,
    name: &str,
    kind: &str,
    depends_on: Vec<&str>,
) -> StepDsl {
    StepDsl {
        id: id.to_string(),
        name: name.to_string(),
        kind: kind.to_string(),
        entrypoint: Some("main".to_string()),
        depends_on: depends_on
            .into_iter()
            .map(String::from)
            .collect(),
        retryable: true,
        labels: Default::default(),
    }
}

// ── Compiler Testleri ─────────────────────────────────────

#[test]
fn compile_single_node() {
    let dsl = simple_dsl(vec![
        step("s1", "fetch", "wasm", vec![]),
    ]);
    let result = WorkflowCompiler::compile_dsl(&dsl);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.nodes.len(), 1);
    assert_eq!(graph.edges.len(), 0);
}

#[test]
fn compile_linear_chain() {
    let dsl = simple_dsl(vec![
        step("s1", "step1", "wasm",  vec![]),
        step("s2", "step2", "agent", vec!["s1"]),
        step("s3", "step3", "task",  vec!["s2"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn compile_fan_out() {
    // s1 → s2, s1 → s3 (parallel)
    let dsl = simple_dsl(vec![
        step("s1", "root",    "wasm",  vec![]),
        step("s2", "branch-a","agent", vec!["s1"]),
        step("s3", "branch-b","task",  vec!["s1"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn compile_fan_in() {
    // s1, s2 → s3 (join)
    let dsl = simple_dsl(vec![
        step("s1", "a", "wasm",  vec![]),
        step("s2", "b", "wasm",  vec![]),
        step("s3", "c", "agent", vec!["s1", "s2"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    assert_eq!(graph.nodes.len(), 3);
    assert_eq!(graph.edges.len(), 2);
}

#[test]
fn compile_empty_returns_error() {
    let dsl = simple_dsl(vec![]);
    let err = WorkflowCompiler::compile_dsl(&dsl).unwrap_err();
    assert_eq!(err, CompilerError::EmptyWorkflow);
}

#[test]
fn compile_unknown_dependency_returns_error() {
    let dsl = simple_dsl(vec![
        step("s1", "step", "wasm", vec!["nonexistent"]),
    ]);
    let err = WorkflowCompiler::compile_dsl(&dsl).unwrap_err();
    assert!(matches!(
        err,
        CompilerError::UnknownDependency { .. }
    ));
}

#[test]
fn compile_unknown_kind_returns_error() {
    let dsl = simple_dsl(vec![
        step("s1", "step", "unknown_type", vec![]),
    ]);
    let err = WorkflowCompiler::compile_dsl(&dsl).unwrap_err();
    assert!(matches!(
        err,
        CompilerError::UnknownNodeKind { .. }
    ));
}

#[test]
fn compile_cyclic_dependency_detected() {
    // s1 → s2 → s1 (döngü)
    let dsl = simple_dsl(vec![
        step("s1", "a", "wasm",  vec!["s2"]),
        step("s2", "b", "agent", vec!["s1"]),
    ]);
    let err = WorkflowCompiler::compile_dsl(&dsl).unwrap_err();
    assert!(matches!(
        err,
        CompilerError::CyclicDependency(_)
    ));
}

#[test]
fn compile_all_node_kinds() {
    let dsl = simple_dsl(vec![
        step("s1", "wasm-step",   "wasm",    vec![]),
        step("s2", "agent-step",  "agent",   vec!["s1"]),
        step("s3", "ai-step",     "ai",      vec!["s2"]),
        step("s4", "task-step",   "task",    vec!["s3"]),
        step("s5", "plugin-step", "plugin",  vec!["s4"]),
        step("s6", "remote-step", "remote",  vec!["s5"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    assert_eq!(graph.nodes.len(), 6);
}

#[test]
fn compile_from_json() {
    let json = r#"{
        "name": "json-workflow",
        "version": 1,
        "steps": [
            {"id": "s1", "name": "fetch", "type": "wasm",
             "depends_on": [], "retryable": true},
            {"id": "s2", "name": "process", "type": "agent",
             "depends_on": ["s1"], "retryable": false}
        ]
    }"#;

    let result = WorkflowCompiler::from_json(json);
    assert!(result.is_ok());
    let graph = result.unwrap();
    assert_eq!(graph.nodes.len(), 2);
    assert_eq!(graph.version, 1);
}

#[test]
fn compile_json_invalid_returns_error() {
    let result = WorkflowCompiler::from_json("not json!!!");
    assert!(result.is_err());
}

// ── Validator Testleri ────────────────────────────────────

#[test]
fn validate_valid_graph_passes() {
    let dsl = simple_dsl(vec![
        step("s1", "a", "wasm",  vec![]),
        step("s2", "b", "agent", vec!["s1"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    assert!(WorkflowValidator::validate(&graph));
}

#[test]
fn validate_full_single_node_passes() {
    let dsl = simple_dsl(vec![
        step("s1", "solo", "wasm", vec![]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    // Tek node'lu graph isolated node hatasına düşmemeli
    assert!(WorkflowValidator::validate_full(&graph).is_ok());
}

// ── Topology Testleri ─────────────────────────────────────

#[test]
fn topology_roots_correct() {
    let dsl = simple_dsl(vec![
        step("s1", "root-1", "wasm",  vec![]),
        step("s2", "root-2", "wasm",  vec![]),
        step("s3", "child",  "agent", vec!["s1", "s2"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    let roots = WorkflowTopology::roots(&graph);
    assert_eq!(roots.len(), 2, "2 root node olmalı");
}

#[test]
fn topology_leaves_correct() {
    let dsl = simple_dsl(vec![
        step("s1", "a", "wasm",  vec![]),
        step("s2", "b", "agent", vec!["s1"]),
        step("s3", "c", "task",  vec!["s1"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    let leaves = WorkflowTopology::leaves(&graph);
    assert_eq!(leaves.len(), 2, "2 leaf node olmalı");
}

#[test]
fn topology_adjacency_correct() {
    let dsl = simple_dsl(vec![
        step("s1", "a", "wasm",  vec![]),
        step("s2", "b", "agent", vec!["s1"]),
    ]);
    let graph = WorkflowCompiler::compile_dsl(&dsl).unwrap();
    let adj = WorkflowTopology::adjacency(&graph);
    // s1'den s2'ye bir kenar olmalı
    let s1_id = graph.nodes.iter()
        .find(|n| n.metadata.name == "a")
        .unwrap().id;
    assert!(adj.contains_key(&s1_id));
    assert_eq!(adj[&s1_id].len(), 1);
}
