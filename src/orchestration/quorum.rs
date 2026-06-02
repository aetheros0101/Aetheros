use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuorumState {
    pub total_nodes: usize,
    pub healthy_nodes: usize,
    pub required_quorum: usize,
}

impl QuorumState {
    pub fn has_quorum(&self) -> bool {
        self.healthy_nodes >= self.required_quorum
    }
}

pub struct QuorumPolicy {
    pub minimum_nodes: usize,
}

impl QuorumPolicy {
    pub fn satisfied(&self, available: usize) -> bool {
        available >= self.minimum_nodes
    }
}
