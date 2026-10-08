// ============================================================
// src/remote/discovery.rs
//
// Sprint 10: Tam Node Discovery
//
// ÖNCE: Sadece DiscoveryRecord struct
//
// SONRA:
//   - StaticDiscovery: seed list'ten node'ları yükle
//   - DiscoveryService: periyodik node keşfi + cluster sync
//   - join() → kendi adresini cluster'a bildir
//   - leave() → cluster'dan temiz çıkış
//   - probe() → node'un canlı olup olmadığını kontrol et
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};
use uuid::Uuid;

use crate::remote::cluster::ClusterState;
use crate::remote::node::{NodeCapability, RemoteNode};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryRecord {
    pub address: String,
    pub transport: String,
}

/// Basit HTTP-tabanlı join payload.
#[derive(Debug, Serialize, Deserialize)]
pub struct JoinRequest {
    pub node_id: Uuid,
    pub address: String,
    pub capabilities: Vec<String>,
}

pub struct DiscoveryService {
    /// Kendi node kimliği
    self_id: Uuid,
    /// Kendi adres
    self_address: String,
    /// Seed node'lar (bootstrap)
    seeds: Vec<String>,
    /// Cluster state referansı
    cluster: Arc<ClusterState>,
    /// HTTP client
    client: reqwest::Client,
}

impl DiscoveryService {
    pub fn new(
        self_id: Uuid,
        self_address: impl Into<String>,
        seeds: Vec<String>,
        cluster: Arc<ClusterState>,
    ) -> Self {
        Self {
            self_id,
            self_address: self_address.into(),
            seeds,
            cluster,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(5))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Cluster'a katıl: her seed node'a join isteği gönder.
    pub async fn join(&self) {
        info!(
            node_id = %self.self_id,
            addr = %self.self_address,
            seeds = self.seeds.len(),
            "Joining cluster"
        );

        // Kendi node'umuzu cluster'a ekle
        let self_node = RemoteNode::new(
            self.self_address.clone(),
            vec![
                NodeCapability::WasmExecution,
                NodeCapability::WorkflowExecution,
                NodeCapability::AgentExecution,
            ],
        );
        self.cluster.register(self_node);

        // Seed node'lara bildir
        for seed in &self.seeds {
            match self.probe(seed).await {
                Ok(true) => {
                    info!(seed = %seed, "Seed node reachable");
                    // Seed'i cluster'a ekle
                    let seed_node =
                        RemoteNode::new(seed.clone(), vec![NodeCapability::WasmExecution]);
                    self.cluster.register(seed_node);
                }
                Ok(false) => {
                    warn!(seed = %seed, "Seed node unreachable");
                }
                Err(e) => {
                    warn!(seed = %seed, error = %e, "Seed probe failed");
                }
            }
        }

        info!(cluster_size = self.cluster.size(), "Cluster join complete");
    }

    /// Cluster'dan temiz ayrıl.
    pub async fn leave(&self) {
        info!(
            node_id = %self.self_id,
            "Leaving cluster"
        );
        self.cluster.remove(&self.self_id);
    }

    /// Node canlı mı? GET /health endpoint'ini kontrol et.
    pub async fn probe(&self, address: &str) -> Result<bool, String> {
        let url = format!("http://{}/health", address);

        match self.client.get(&url).send().await {
            Ok(resp) => Ok(resp.status().is_success()),
            Err(e) => {
                debug!(
                    addr = %address,
                    error = %e,
                    "Probe failed"
                );
                Ok(false)
            }
        }
    }

    /// Periyodik discovery döngüsü.
    /// Her `interval` saniyede bir seed'leri kontrol et.
    pub fn start(
        self: Arc<Self>,
        interval_secs: u64,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) {
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                tokio::select! {
                    _ = ticker.tick() => {
                        self.cluster.evict_stale_nodes();

                        // Sağlıksız seed'leri yeniden dene
                        for seed in &self.seeds {
                            if let Ok(true) = self.probe(seed).await
                                && !self
                                    .cluster
                                    .nodes()
                                    .iter()
                                    .any(|n| n.address == *seed)
                            {
                                info!(seed = %seed, "Seed node recovered");
                                let node = RemoteNode::new(
                                    seed.clone(),
                                    vec![NodeCapability::WasmExecution],
                                );
                                self.cluster.register(node);
                            }
                        }

                        debug!(
                            healthy = self.cluster.healthy_count(),
                            total = self.cluster.size(),
                            quorum = self.cluster.has_quorum(),
                            "Discovery tick"
                        );
                    }

                    _ = shutdown_rx.changed() => {
                        self.leave().await;
                        break;
                    }
                }
            }
        });
    }
}

/// Basit static discovery — env/config dosyasından seed'ler.
pub struct StaticDiscovery;

impl StaticDiscovery {
    /// AETHEROS_SEEDS="addr1:9000,addr2:9000,addr3:9000"
    pub fn from_env() -> Vec<String> {
        std::env::var("AETHEROS_SEEDS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }
}
