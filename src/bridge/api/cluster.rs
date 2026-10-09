// ============================================================
// bridge/api/cluster.rs
//
// Cluster durumu ve node kaydı.
// ============================================================

use crate::bridge::api::error::require_runtime;
use crate::bridge::types::{ClusterNodeResponse, ClusterStatusResponse, NodeRegistrationResponse};

// ── Cluster fonksiyonları (FRB) ───────────────────────────────

/// Cluster genel durumunu getir.
pub fn get_cluster_status() -> Result<ClusterStatusResponse, String> {
    let rt = require_runtime()?;

    let nodes: Vec<ClusterNodeResponse> = rt
        .cluster
        .nodes()
        .into_iter()
        .map(|n| {
            let hb = rt.cluster.heartbeat(&n.node_id);
            ClusterNodeResponse {
                node_id: n.node_id.to_string(),
                address: n.address.clone(),
                healthy: n.healthy,
                capabilities: n.capabilities.iter().map(|c| format!("{:?}", c)).collect(),
                cpu_percent: hb.as_ref().map(|h| h.cpu_usage_percent).unwrap_or(0.0),
                memory_mb: hb.as_ref().map(|h| h.memory_usage_mb as u64).unwrap_or(0),
                active_executions: hb.as_ref().map(|h| h.active_executions as u64).unwrap_or(0),
            }
        })
        .collect();

    Ok(ClusterStatusResponse {
        health: format!("{:?}", rt.cluster.health()),
        total: rt.cluster.size() as u64,
        healthy: rt.cluster.healthy_count() as u64,
        has_quorum: rt.cluster.has_quorum(),
        leader: rt.cluster.leader().map(|u| u.to_string()),
        nodes,
    })
}

/// Cluster'a yeni node kaydet.
pub fn register_node(
    address: String,
    capabilities: Vec<String>,
) -> Result<NodeRegistrationResponse, String> {
    let rt = require_runtime()?;

    use crate::remote::node::{NodeCapability, RemoteNode};

    let caps: Vec<NodeCapability> = capabilities
        .iter()
        .map(|c| match c.to_lowercase().as_str() {
            "wasm" => NodeCapability::WasmExecution,
            "workflow" => NodeCapability::WorkflowExecution,
            "agent" => NodeCapability::AgentExecution,
            "ai" => NodeCapability::AiInference,
            "plugin" => NodeCapability::PluginExecution,
            _ => NodeCapability::WasmExecution,
        })
        .collect();

    let node = RemoteNode::new(address.clone(), caps);
    let node_id = node.node_id.to_string();
    rt.cluster.register(node);

    Ok(NodeRegistrationResponse {
        node_id,
        address,
        status: "registered".into(),
    })
}
