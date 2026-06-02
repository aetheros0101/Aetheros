use async_trait::async_trait;

use crate::remote::cluster::ClusterState;

#[async_trait]
pub trait ClusterReplication:
    Send + Sync
{
    async fn replicate(
        &self,
        cluster:
            &ClusterState,
    );

    async fn synchronize(
            &self,
        );
}
