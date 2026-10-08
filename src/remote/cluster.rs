// ============================================================
// src/remote/cluster.rs
//
// Sprint 10: Tam ClusterState
//
// ÖNCE: Sadece DashMap<Uuid, RemoteNode> — CRUD
//
// SONRA:
//   - Node ekleme/çıkarma + heartbeat güncelleme
//   - Stale node tespiti (30s timeout)
//   - Cluster health hesaplama
//   - Quorum kontrolü (n/2 + 1)
//   - Leader bilgisi
//   - Capability bazlı node filtreleme
// ============================================================

use std::collections::HashSet;
use std::time::Duration;

use chrono::Utc;
use dashmap::DashMap;
use tracing::{info, warn};
use uuid::Uuid;

use crate::remote::cluster_health::ClusterHealth;
use crate::remote::heartbeat::Heartbeat;
use crate::remote::node::{NodeCapability, RemoteNode};

/// Node'un stale sayılacağı süre.
const STALE_THRESHOLD: Duration = Duration::from_secs(30);

pub struct ClusterState {
    nodes: DashMap<Uuid, RemoteNode>,
    heartbeats: DashMap<Uuid, Heartbeat>,
    /// Mevcut leader (None = seçim bekleniyor)
    leader: DashMap<&'static str, Uuid>,
}

impl Default for ClusterState {
    fn default() -> Self {
        Self::new()
    }
}

impl ClusterState {
    pub fn new() -> Self {
        Self {
            nodes: DashMap::new(),
            heartbeats: DashMap::new(),
            leader: DashMap::new(),
        }
    }

    // ── Node Yönetimi ─────────────────────────────────────

    pub fn register(&self, node: RemoteNode) {
        info!(
            node_id = %node.node_id,
            addr = %node.address,
            "Node registered"
        );
        self.nodes.insert(node.node_id, node);
    }

    pub fn remove(&self, node_id: &Uuid) {
        if self.nodes.remove(node_id).is_some() {
            self.heartbeats.remove(node_id);
            warn!(node_id = %node_id, "Node removed from cluster");
        }

        // Önce değeri al, sonra sil — deadlock önlenir
        let is_leader = self
            .leader
            .get("current")
            .map(|l| *l == *node_id)
            .unwrap_or(false);

        drop(self.leader.get("current")); // referansı serbest bırak

        if is_leader {
            self.leader.remove("current");
            warn!("Leader node removed — election needed");
        }
    }

    pub fn update_heartbeat(&self, hb: Heartbeat) {
        let node_id = hb.node_id;

        // Node'un last_seen'ini güncelle
        if let Some(mut node) = self.nodes.get_mut(&node_id) {
            node.touch();
            node.healthy = true;
        }

        self.heartbeats.insert(node_id, hb);
    }

    // ── Node Sorguları ────────────────────────────────────

    pub fn nodes(&self) -> Vec<RemoteNode> {
        self.nodes.iter().map(|n| n.value().clone()).collect()
    }

    pub fn healthy_nodes(&self) -> Vec<RemoteNode> {
        let now = Utc::now();

        self.nodes
            .iter()
            .filter(|n| {
                n.healthy && (now - n.last_seen).to_std().unwrap_or(Duration::MAX) < STALE_THRESHOLD
            })
            .map(|n| n.value().clone())
            .collect()
    }

    pub fn nodes_with_capability(&self, cap: &NodeCapability) -> Vec<RemoteNode> {
        self.healthy_nodes()
            .into_iter()
            .filter(|n| n.capabilities.contains(cap))
            .collect()
    }

    pub fn node_ids(&self) -> HashSet<Uuid> {
        self.nodes.iter().map(|n| *n.key()).collect()
    }

    pub fn get_node(&self, id: &Uuid) -> Option<RemoteNode> {
        self.nodes.get(id).map(|n| n.clone())
    }

    pub fn heartbeat(&self, id: &Uuid) -> Option<Heartbeat> {
        self.heartbeats.get(id).map(|h| h.clone())
    }

    pub fn all_heartbeats(&self) -> Vec<Heartbeat> {
        self.heartbeats.iter().map(|h| h.value().clone()).collect()
    }

    // ── Stale Node Temizleme ──────────────────────────────

    /// Stale node'ları unhealthy olarak işaretle.
    /// Periyodik olarak çağrılmalı (health check loop).
    pub fn evict_stale_nodes(&self) {
        let now = Utc::now();

        for mut node in self.nodes.iter_mut() {
            let stale = (now - node.last_seen).to_std().unwrap_or(Duration::MAX) > STALE_THRESHOLD;

            if stale && node.healthy {
                node.mark_unhealthy();
                warn!(
                    node_id = %node.node_id,
                    addr = %node.address,
                    "Node marked stale"
                );
            }
        }
    }

    // ── Cluster Health ────────────────────────────────────

    pub fn health(&self) -> ClusterHealth {
        let total = self.nodes.len();
        let healthy = self.healthy_nodes().len();

        if total == 0 {
            return ClusterHealth::Critical;
        }

        let ratio = healthy as f32 / total as f32;

        if ratio >= 0.8 {
            ClusterHealth::Healthy
        } else if ratio >= 0.5 {
            ClusterHealth::Degraded
        } else {
            ClusterHealth::Critical
        }
    }

    // ── Quorum ────────────────────────────────────────────

    /// n/2 + 1 quorum var mı?
    pub fn has_quorum(&self) -> bool {
        let total = self.nodes.len();
        let healthy = self.healthy_nodes().len();
        let required = total / 2 + 1;
        healthy >= required
    }

    pub fn quorum_size(&self) -> usize {
        self.nodes.len() / 2 + 1
    }

    // ── Leader ────────────────────────────────────────────

    pub fn set_leader(&self, node_id: Uuid) {
        info!(leader = %node_id, "Cluster leader set");
        self.leader.insert("current", node_id);
    }

    pub fn leader(&self) -> Option<Uuid> {
        self.leader.get("current").map(|l| *l)
    }

    pub fn is_leader(&self, node_id: &Uuid) -> bool {
        self.leader().map(|l| l == *node_id).unwrap_or(false)
    }

    // ── Stats ─────────────────────────────────────────────

    pub fn size(&self) -> usize {
        self.nodes.len()
    }

    pub fn healthy_count(&self) -> usize {
        self.healthy_nodes().len()
    }
}
