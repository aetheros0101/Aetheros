//! Çoklu agent: manager, registry, kota, isolation.

pub mod capacity;
pub mod manager;
pub mod registry;
pub mod subscriptions;

pub use capacity::{AgentCapacity, QuotaPolicy, QuotaUsage};
pub use manager::{
    AgentDescriptor, AgentManager, ExecutionLease, IsolationScope, ManagerStats, SharedAgentManager,
};
pub use registry::AgentRegistry;
pub use subscriptions::AgentSubscription;
