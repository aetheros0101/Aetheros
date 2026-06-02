// ============================================================
// src/remote/transport.rs
//
// Sprint 10: HTTP tabanlı RemoteTransport implementasyonu
//
// ÖNCE: trait tanımı, implementasyon yok
//
// SONRA: HttpTransport
//   send()      → tek node'a POST /remote/command
//   broadcast() → tüm cluster node'larına paralel send()
//   fire_and_forget: hata loglana, panic yok
// ============================================================

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use tracing::{
    debug,
    warn,
};

use crate::remote::cluster::ClusterState;
use crate::remote::protocol::RemoteCommand;

#[async_trait]
pub trait RemoteTransport: Send + Sync {
    async fn send(&self, command: RemoteCommand);
    async fn broadcast(&self, command: RemoteCommand);
}

pub struct HttpTransport {
    cluster: Arc<ClusterState>,
    client: reqwest::Client,
}

impl HttpTransport {
    pub fn new(cluster: Arc<ClusterState>) -> Self {
        Self {
            cluster,
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
        }
    }

    /// Belirli adrese komut gönder.
    async fn send_to(
        &self,
        address: &str,
        command: &RemoteCommand,
    ) -> bool {
        let url =
            format!("http://{}/remote/command", address);

        match self
            .client
            .post(&url)
            .json(command)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                debug!(
                    addr = %address,
                    cmd = ?command,
                    "Command delivered"
                );
                true
            }
            Ok(resp) => {
                warn!(
                    addr = %address,
                    status = %resp.status(),
                    "Command rejected by node"
                );
                false
            }
            Err(e) => {
                warn!(
                    addr = %address,
                    error = %e,
                    "Failed to send command"
                );
                false
            }
        }
    }
}

#[async_trait]
impl RemoteTransport for HttpTransport {
    /// En uygun (en düşük yüklü) node'a gönder.
    async fn send(&self, command: RemoteCommand) {
        let heartbeats = self.cluster.all_heartbeats();

        // Scheduler'dan en uygun node'u al
        let nodes = self.cluster.healthy_nodes();
        if nodes.is_empty() {
            warn!("No healthy nodes to send command");
            return;
        }

        // En düşük yüklü node (heartbeat yoksa ilk healthy)
        let target = heartbeats
            .iter()
            .filter_map(|hb| {
                nodes
                    .iter()
                    .find(|n| n.node_id == hb.node_id)
                    .map(|n| (hb.cpu_usage_percent, n))
            })
            .min_by(|a, b| {
                a.0.partial_cmp(&b.0)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|(_, n)| n)
            .unwrap_or(&nodes[0]);

        self.send_to(&target.address, &command).await;
    }

    /// Tüm healthy node'lara paralel gönder.
    async fn broadcast(&self, command: RemoteCommand) {
        let nodes = self.cluster.healthy_nodes();

        if nodes.is_empty() {
            warn!("No healthy nodes to broadcast");
            return;
        }

        let client = self.client.clone();
        let mut handles = Vec::new();

        for node in nodes {
            let cmd = command.clone();
            let addr = node.address.clone();
            let c = client.clone();

            handles.push(tokio::spawn(async move {
                let url =
                    format!("http://{}/remote/command", addr);
                let _ = c.post(&url).json(&cmd).send().await;
            }));
        }

        // Tüm broadcast'lerin tamamlanmasını bekle
        futures::future::join_all(handles).await;
    }
}
