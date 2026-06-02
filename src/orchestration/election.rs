// ============================================================
// src/orchestration/election.rs
//
// Sprint 10: Tam Leader Election
//
// ÖNCE: elect() → nodes.first() (naif, deterministic)
//
// SONRA: Bully Algorithm benzeri seçim
//   1. Heartbeat'e göre en düşük yüklü sağlıklı node
//   2. Tie-break: UUID lexicographic (deterministic)
//   3. Quorum gerektiriyor — yoksa None
//   4. ElectionResult: kazanan + oylar + quorum durumu
// ============================================================

use uuid::Uuid;

use crate::remote::cluster::ClusterState;
use crate::remote::heartbeat::Heartbeat;
use crate::remote::node::RemoteNode;

#[derive(Debug, Clone)]
pub struct ElectionResult {
    pub winner: Uuid,
    pub votes: usize,
    pub quorum_met: bool,
    pub candidate_count: usize,
}

pub struct LeaderElection;

impl LeaderElection {
    /// Sağlıklı node'lar arasından leader seç.
    ///
    /// Algoritma:
    ///   1. Quorum yok → None
    ///   2. Sağlıklı node'ları al
    ///   3. Her node için load_score hesapla
    ///   4. En düşük yüklü = aday
    ///   5. Tie: UUID lexicographic (deterministic)
    ///
    /// Gerçek dağıtık sistemde bu kısmi — tam Raft/Paxos
    /// her node'un oy gondermesi gerektirir. Bu implementasyon
    /// coordinator tarafından çalıştırılan merkezi versiyondur.
    pub fn elect(
        cluster: &ClusterState,
    ) -> Option<ElectionResult> {
        // Quorum kontrolü
        if !cluster.has_quorum() {
            return None;
        }

        let candidates = cluster.healthy_nodes();
        let candidate_count = candidates.len();

        if candidates.is_empty() {
            return None;
        }

        let heartbeats = cluster.all_heartbeats();

        // Her aday için skor hesapla — düşük skor = iyi aday
        let winner = candidates
            .iter()
            .min_by(|a, b| {
                let score_a = Self::candidate_score(
                    a,
                    &heartbeats,
                );
                let score_b = Self::candidate_score(
                    b,
                    &heartbeats,
                );

                score_a
                    .partial_cmp(&score_b)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    // Tie-break: UUID lexicographic
                    .then(a.node_id.cmp(&b.node_id))
            })?;

        Some(ElectionResult {
            winner: winner.node_id,
            votes: candidate_count, // Merkezi seçimde tüm node'lar "oy vermiş" sayılır
            quorum_met: true,
            candidate_count,
        })
    }

    /// Eski API — geriye uyumluluk.
    /// Sadece winner UUID'i döndürür.
    pub fn elect_simple(nodes: Vec<Uuid>) -> Option<Uuid> {
        nodes.into_iter().min() // deterministic: en küçük UUID
    }

    /// Aday skoru: düşük = tercih edilir.
    /// cpu * 0.4 + mem_norm * 0.3 + exec_norm * 0.3
    fn candidate_score(
        node: &RemoteNode,
        heartbeats: &[Heartbeat],
    ) -> f32 {
        let hb = heartbeats
            .iter()
            .find(|h| h.node_id == node.node_id);

        match hb {
            None => f32::MAX, // Heartbeat yok → en kötü aday
            Some(h) => {
                let cpu = h.cpu_usage_percent / 100.0;
                let mem = h.memory_usage_mb as f32 / 32_768.0; // 32GB norm
                let exec = h.active_executions as f32 / 1000.0;

                cpu * 0.4 + mem * 0.3 + exec * 0.3
            }
        }
    }
}
