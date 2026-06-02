// ============================================================
// src/tests/remote_tests.rs
//
// SPRINT 5 — Remote Scheduler & Node Testleri
// ============================================================

use chrono::Utc;
use uuid::Uuid;

use crate::remote::heartbeat::Heartbeat;
use crate::remote::node::{
    NodeCapability,
    RemoteNode,
};
use crate::remote::scheduler::RemoteScheduler;

// ── Yardımcılar ───────────────────────────────────────────

fn healthy_node(caps: Vec<NodeCapability>) -> RemoteNode {
    RemoteNode {
        node_id: Uuid::new_v4(),
        address: "127.0.0.1:9000".to_string(),
        healthy: true,
        last_seen: Utc::now(),
        capabilities: caps,
    }
}

fn unhealthy_node() -> RemoteNode {
    RemoteNode {
        node_id: Uuid::new_v4(),
        address: "127.0.0.1:9001".to_string(),
        healthy: false,
        last_seen: Utc::now(),
        capabilities: vec![NodeCapability::WasmExecution],
    }
}

fn heartbeat(node_id: Uuid, cpu: f32, mem: usize, exec: usize) -> Heartbeat {
    Heartbeat {
        node_id,
        active_executions: exec,
        memory_usage_mb: mem,
        cpu_usage_percent: cpu,
        load_average: cpu / 100.0,
        timestamp: Utc::now(),
    }
}

// ── RemoteNode Testleri ───────────────────────────────────

#[test]
fn node_new_is_healthy() {
    let node = RemoteNode::new(
        "localhost:9000",
        vec![NodeCapability::WasmExecution],
    );
    assert!(node.healthy);
    assert!(!node.capabilities.is_empty());
}

#[test]
fn node_mark_unhealthy() {
    let mut node = RemoteNode::new(
        "localhost:9000",
        vec![NodeCapability::WasmExecution],
    );
    node.mark_unhealthy();
    assert!(!node.healthy);
}

#[test]
fn node_touch_updates_timestamp() {
    let mut node = RemoteNode::new(
        "localhost:9000",
        vec![NodeCapability::WasmExecution],
    );
    let before = node.last_seen;
    std::thread::sleep(std::time::Duration::from_millis(5));
    node.touch();
    assert!(node.last_seen > before);
}

// ── RemoteScheduler Testleri ──────────────────────────────

#[test]
fn select_any_returns_healthy() {
    let nodes = vec![
        unhealthy_node(),
        healthy_node(vec![NodeCapability::WasmExecution]),
    ];

    let selected = RemoteScheduler::select_any(&nodes);
    assert!(selected.is_some());
    assert!(nodes
        .iter()
        .find(|n| n.node_id == selected.unwrap())
        .unwrap()
        .healthy);
}

#[test]
fn select_any_empty_returns_none() {
    let result = RemoteScheduler::select_any(&[]);
    assert!(result.is_none());
}

#[test]
fn select_any_all_unhealthy_returns_none() {
    let nodes = vec![unhealthy_node(), unhealthy_node()];
    assert!(RemoteScheduler::select_any(&nodes).is_none());
}

#[test]
fn select_node_filters_by_capability() {
    let wasm_node = healthy_node(vec![
        NodeCapability::WasmExecution,
    ]);
    let agent_node = healthy_node(vec![
        NodeCapability::AgentExecution,
    ]);

    let nodes = vec![wasm_node.clone(), agent_node];
    let hbs = vec![
        heartbeat(wasm_node.node_id, 10.0, 512, 1),
    ];

    // WasmExecution isteğinde wasm_node seçilmeli
    let sel = RemoteScheduler::select_node(
        &nodes,
        &NodeCapability::WasmExecution,
        &hbs,
    );
    assert!(sel.is_some());
    assert_eq!(sel.unwrap().node_id, wasm_node.node_id);
}

#[test]
fn select_node_no_matching_capability_returns_none() {
    let node = healthy_node(vec![
        NodeCapability::WasmExecution,
    ]);

    let sel = RemoteScheduler::select_node(
        &[node],
        &NodeCapability::AiInference, // yok
        &[],
    );
    assert!(sel.is_none());
}

#[test]
fn select_node_picks_least_loaded() {
    let node_a = healthy_node(vec![
        NodeCapability::WasmExecution,
    ]);
    let node_b = healthy_node(vec![
        NodeCapability::WasmExecution,
    ]);

    let hbs = vec![
        heartbeat(node_a.node_id, 80.0, 8192, 50), // yüklü
        heartbeat(node_b.node_id, 10.0, 512, 2),   // boşta
    ];

    let sel = RemoteScheduler::select_node(
        &[node_a.clone(), node_b.clone()],
        &NodeCapability::WasmExecution,
        &hbs,
    );

    assert!(sel.is_some());
    assert_eq!(
        sel.unwrap().node_id,
        node_b.node_id,
        "En az yüklü node seçilmeli"
    );
}

#[test]
fn select_node_unhealthy_skipped() {
    let bad = unhealthy_node();
    let good = healthy_node(vec![
        NodeCapability::WasmExecution,
    ]);

    let sel = RemoteScheduler::select_node(
        &[bad, good.clone()],
        &NodeCapability::WasmExecution,
        &[],
    );

    assert!(sel.is_some());
    assert_eq!(sel.unwrap().node_id, good.node_id);
}
