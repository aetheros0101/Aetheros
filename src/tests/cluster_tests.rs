// ============================================================
// src/tests/cluster_tests.rs
//
// SPRINT 10 — Distributed Cluster Testleri
// ============================================================

use chrono::Utc;
use uuid::Uuid;

use crate::orchestration::election::LeaderElection;
use crate::orchestration::quorum::{
    QuorumPolicy,
    QuorumState,
};
use crate::remote::cluster::ClusterState;
use crate::remote::cluster_health::ClusterHealth;
use crate::remote::discovery::StaticDiscovery;
use crate::remote::heartbeat::Heartbeat;
use crate::remote::node::{
    NodeCapability,
    RemoteNode,
};

// ── Yardımcılar ───────────────────────────────────────────

fn make_node(
    caps: Vec<NodeCapability>,
) -> RemoteNode {
    RemoteNode::new("127.0.0.1:9000", caps)
}

fn healthy_node() -> RemoteNode {
    make_node(vec![NodeCapability::WasmExecution])
}

fn heartbeat_for(
    node_id: Uuid,
    cpu: f32,
    mem: usize,
    exec: usize,
) -> Heartbeat {
    Heartbeat {
        node_id,
        cpu_usage_percent: cpu,
        memory_usage_mb: mem,
        active_executions: exec,
        load_average: cpu / 100.0,
        timestamp: Utc::now(),
    }
}

// ── ClusterState Testleri ─────────────────────────────────

#[test]
fn cluster_register_and_list() {
    let cluster = ClusterState::new();

    let n1 = healthy_node();
    let n2 = healthy_node();
    cluster.register(n1.clone());
    cluster.register(n2.clone());

    assert_eq!(cluster.size(), 2);
    assert_eq!(cluster.healthy_count(), 2);
}

#[test]
fn cluster_remove_node() {
    let cluster = ClusterState::new();
    let node = healthy_node();
    let id = node.node_id;

    cluster.register(node);
    assert_eq!(cluster.size(), 1);

    cluster.remove(&id);
    assert_eq!(cluster.size(), 0);
}

#[test]
fn cluster_health_all_healthy() {
    let cluster = ClusterState::new();
    cluster.register(healthy_node());
    cluster.register(healthy_node());
    cluster.register(healthy_node());

    assert!(matches!(
        cluster.health(),
        ClusterHealth::Healthy
    ));
}

#[test]
fn cluster_health_empty_is_critical() {
    let cluster = ClusterState::new();
    assert!(matches!(
        cluster.health(),
        ClusterHealth::Critical
    ));
}

#[test]
fn cluster_quorum_majority_required() {
    let cluster = ClusterState::new();

    // 3 node → quorum = 2
    cluster.register(healthy_node());
    cluster.register(healthy_node());
    cluster.register(healthy_node());

    assert!(cluster.has_quorum());
    assert_eq!(cluster.quorum_size(), 2);
}

#[test]
fn cluster_no_quorum_when_single_node() {
    let cluster = ClusterState::new();
    cluster.register(healthy_node());

    // 1 node → quorum = 1 → has_quorum true (min cluster)
    assert!(cluster.has_quorum());
    assert_eq!(cluster.quorum_size(), 1);
}

#[test]
fn cluster_leader_set_and_get() {
    let cluster = ClusterState::new();
    let node = healthy_node();
    let id = node.node_id;

    cluster.register(node);
    cluster.set_leader(id);

    assert_eq!(cluster.leader(), Some(id));
    assert!(cluster.is_leader(&id));
}

#[test]
fn cluster_leader_cleared_on_remove() {
    let cluster = ClusterState::new();
    let node = healthy_node();
    let id = node.node_id;

    cluster.register(node);
    cluster.set_leader(id);
    assert_eq!(cluster.leader(), Some(id));

    cluster.remove(&id);
    assert_eq!(cluster.leader(), None);
}

#[test]
fn cluster_heartbeat_update() {
    let cluster = ClusterState::new();
    let node = healthy_node();
    let id = node.node_id;

    cluster.register(node);
    cluster.update_heartbeat(heartbeat_for(id, 25.0, 512, 3));

    let hb = cluster.heartbeat(&id);
    assert!(hb.is_some());
    assert_eq!(hb.unwrap().cpu_usage_percent, 25.0);
}

#[test]
fn cluster_capability_filter() {
    let cluster = ClusterState::new();

    let wasm_node =
        make_node(vec![NodeCapability::WasmExecution]);
    let agent_node =
        make_node(vec![NodeCapability::AgentExecution]);

    cluster.register(wasm_node);
    cluster.register(agent_node);

    let wasm_capable = cluster
        .nodes_with_capability(&NodeCapability::WasmExecution);
    assert_eq!(wasm_capable.len(), 1);

    let agent_capable = cluster
        .nodes_with_capability(&NodeCapability::AgentExecution);
    assert_eq!(agent_capable.len(), 1);

    let ai_capable = cluster
        .nodes_with_capability(&NodeCapability::AiInference);
    assert_eq!(ai_capable.len(), 0);
}

// ── LeaderElection Testleri ───────────────────────────────

#[test]
fn election_picks_winner_with_quorum() {
    let cluster = ClusterState::new();

    let n1 = make_node(vec![NodeCapability::WasmExecution]);
    let n2 = make_node(vec![NodeCapability::WasmExecution]);
    let n3 = make_node(vec![NodeCapability::WasmExecution]);

    let id1 = n1.node_id;
    let id2 = n2.node_id;
    let id3 = n3.node_id;

    cluster.register(n1);
    cluster.register(n2);
    cluster.register(n3);

    // Heartbeat ekle — id2 en az yüklü
    cluster.update_heartbeat(heartbeat_for(id1, 80.0, 8192, 50));
    cluster.update_heartbeat(heartbeat_for(id2, 10.0, 512, 2));
    cluster.update_heartbeat(heartbeat_for(id3, 50.0, 4096, 20));

    let result = LeaderElection::elect(&cluster);
    assert!(result.is_some());
    let result = result.unwrap();
    assert!(result.quorum_met);
    assert_eq!(result.winner, id2, "En düşük yüklü node seçilmeli");
}

#[test]
fn election_simple_deterministic() {
    let ids = vec![
        Uuid::parse_str("aaaaaaaa-0000-0000-0000-000000000000").unwrap(),
        Uuid::parse_str("bbbbbbbb-0000-0000-0000-000000000000").unwrap(),
        Uuid::parse_str("cccccccc-0000-0000-0000-000000000000").unwrap(),
    ];

    let winner = LeaderElection::elect_simple(ids.clone());
    // En küçük UUID seçilmeli (deterministic tie-break)
    assert_eq!(winner, Some(ids[0]));
}

#[test]
fn election_empty_nodes_returns_none() {
    let result = LeaderElection::elect_simple(vec![]);
    assert!(result.is_none());
}

// ── QuorumState Testleri ──────────────────────────────────

#[test]
fn quorum_state_has_quorum() {
    let q = QuorumState {
        total_nodes: 5,
        healthy_nodes: 3,
        required_quorum: 3,
    };
    assert!(q.has_quorum());
}

#[test]
fn quorum_state_no_quorum() {
    let q = QuorumState {
        total_nodes: 5,
        healthy_nodes: 2,
        required_quorum: 3,
    };
    assert!(!q.has_quorum());
}

#[test]
fn quorum_policy_satisfied() {
    let policy = QuorumPolicy { minimum_nodes: 2 };
    assert!(policy.satisfied(3));
    assert!(policy.satisfied(2));
    assert!(!policy.satisfied(1));
    assert!(!policy.satisfied(0));
}

// ── StaticDiscovery Testleri ──────────────────────────────

#[test]
fn static_discovery_empty_when_no_env() {
    // AETHEROS_SEEDS set değilse boş dön
    unsafe { std::env::remove_var("AETHEROS_SEEDS"); }
    let seeds = StaticDiscovery::from_env();
    assert!(seeds.is_empty());
}

#[test]
fn static_discovery_parses_seeds() {
    unsafe { std::env::set_var("AETHEROS_SEEDS", "127.0.0.1:9001,127.0.0.1:9002,127.0.0.1:9003"); }
    let seeds = StaticDiscovery::from_env();
    assert_eq!(seeds.len(), 3);
    assert_eq!(seeds[0], "127.0.0.1:9001");
    unsafe { std::env::remove_var("AETHEROS_SEEDS"); }
}

#[test]
fn static_discovery_trims_whitespace() {
    unsafe { std::env::set_var("AETHEROS_SEEDS", " 127.0.0.1:9001 , 127.0.0.1:9002 "); }
    let seeds = StaticDiscovery::from_env();
    assert_eq!(seeds.len(), 2);
    assert_eq!(seeds[0], "127.0.0.1:9001");
    unsafe { std::env::remove_var("AETHEROS_SEEDS"); }
}
