// ============================================================
// src/remote/scheduler.rs
//
// Sprint 5: Tam RemoteScheduler
//
// ÖNCE: İlk healthy node'u seç (naif, load-unaware)
//
// SONRA:
//   - Capability filtering (WasmExecution, AgentExecution vb.)
//   - Load-balanced seçim (aktif execution sayısına göre)
//   - Heartbeat tabanlı freshness kontrolü (30s)
//   - Fallback: tüm node'lar dolu → None
// ============================================================

use std::time::Duration;

use chrono::Utc;
use uuid::Uuid;

use crate::remote::heartbeat::Heartbeat;
use crate::remote::node::{NodeCapability, RemoteNode};

/// Node seçim sonucu.
#[derive(Debug)]
pub struct NodeSelection {
    pub node_id: Uuid,
    pub address: String,
    pub load_score: f32,
}

pub struct RemoteScheduler;

impl RemoteScheduler {
    /// Capability'e göre uygun, en az yüklü node'u seç.
    ///
    /// Algoritma:
    ///   1. healthy == true filtrele
    ///   2. last_seen < 30s (stale node'ları ele)
    ///   3. İstenen capability'e sahip node'ları filtrele
    ///   4. Heartbeat'e göre load_score hesapla
    ///   5. En düşük score'lu node'u seç
    pub fn select_node(
        nodes: &[RemoteNode],
        required_capability: &NodeCapability,
        heartbeats: &[Heartbeat],
    ) -> Option<NodeSelection> {
        let now = Utc::now();
        let freshness_limit = Duration::from_secs(30);

        // Sağlıklı + güncel + capability'e sahip node'lar
        let candidates: Vec<&RemoteNode> = nodes
            .iter()
            .filter(|n| {
                n.healthy
                    && (now - n.last_seen).to_std().unwrap_or(Duration::MAX) < freshness_limit
                    && n.capabilities.contains(required_capability)
            })
            .collect();

        if candidates.is_empty() {
            return None;
        }

        // Her candidate için load score hesapla
        let mut best: Option<(f32, &RemoteNode)> = None;

        for node in candidates {
            let score = Self::load_score(node.node_id, heartbeats);

            match best {
                None => best = Some((score, node)),
                Some((best_score, _)) if score < best_score => {
                    best = Some((score, node));
                }
                _ => {}
            }
        }

        best.map(|(score, node)| NodeSelection {
            node_id: node.node_id,
            address: node.address.clone(),
            load_score: score,
        })
    }

    /// Basit seçim (geriye uyumluluk).
    pub fn select_any(nodes: &[RemoteNode]) -> Option<Uuid> {
        nodes.iter().find(|n| n.healthy).map(|n| n.node_id)
    }

    /// Load score: düşük = az yük = tercih edilir.
    /// cpu * 0.5 + memory_norm * 0.3 + active_exec_norm * 0.2
    fn load_score(node_id: Uuid, heartbeats: &[Heartbeat]) -> f32 {
        let hb = heartbeats.iter().find(|h| h.node_id == node_id);

        match hb {
            None => f32::MAX, // Heartbeat yok → en düşük öncelik
            Some(h) => {
                let cpu_score = h.cpu_usage_percent / 100.0;
                let mem_score = (h.memory_usage_mb as f32) / 16_384.0; // 16GB norm
                let exec_score = (h.active_executions as f32) / 100.0;

                cpu_score * 0.5 + mem_score * 0.3 + exec_score * 0.2
            }
        }
    }
}
