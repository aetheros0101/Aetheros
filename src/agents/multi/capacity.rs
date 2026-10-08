//! Kapasite ve kota — global + tenant / profil bazlı.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCapacity {
    pub max_concurrent_tasks: usize,
    pub active_tasks: usize,
}

impl Default for AgentCapacity {
    fn default() -> Self {
        Self {
            max_concurrent_tasks: 4,
            active_tasks: 0,
        }
    }
}

impl AgentCapacity {
    pub fn available(&self) -> usize {
        self.max_concurrent_tasks.saturating_sub(self.active_tasks)
    }

    pub fn is_full(&self) -> bool {
        self.active_tasks >= self.max_concurrent_tasks
    }
}

/// Tenant / profil kotası.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuotaPolicy {
    /// Aynı anda en fazla kaç agent execution.
    pub max_concurrent: usize,
    /// Tenant başına üst sınır (None = global ile aynı).
    pub max_per_tenant: Option<usize>,
    /// Tek agent için max adım (budget ile hizalı, bilgilendirme).
    pub default_max_steps: usize,
    /// Saniye cinsinden max runtime.
    pub default_max_runtime_secs: usize,
}

impl Default for QuotaPolicy {
    fn default() -> Self {
        Self {
            max_concurrent: 8,
            max_per_tenant: Some(4),
            default_max_steps: 50,
            default_max_runtime_secs: 600,
        }
    }
}

impl QuotaPolicy {
    pub fn prod_locked() -> Self {
        Self {
            max_concurrent: 2,
            max_per_tenant: Some(1),
            default_max_steps: 20,
            default_max_runtime_secs: 120,
        }
    }

    pub fn developer() -> Self {
        Self {
            max_concurrent: 16,
            max_per_tenant: Some(8),
            default_max_steps: 100,
            default_max_runtime_secs: 1800,
        }
    }
}

/// Anlık kota kullanımı (manager tutar).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QuotaUsage {
    pub global_active: usize,
    /// tenant_id → active count
    pub per_tenant: std::collections::HashMap<String, usize>,
}

impl QuotaUsage {
    pub fn can_acquire(&self, policy: &QuotaPolicy, tenant: Option<&str>) -> Result<(), String> {
        if self.global_active >= policy.max_concurrent {
            return Err(format!(
                "global concurrent limit {} reached",
                policy.max_concurrent
            ));
        }
        if let (Some(limit), Some(t)) = (policy.max_per_tenant, tenant) {
            let n = self.per_tenant.get(t).copied().unwrap_or(0);
            if n >= limit {
                return Err(format!("tenant `{t}` concurrent limit {limit} reached"));
            }
        }
        Ok(())
    }

    pub fn acquire(&mut self, tenant: Option<&str>) {
        self.global_active = self.global_active.saturating_add(1);
        if let Some(t) = tenant {
            *self.per_tenant.entry(t.to_string()).or_insert(0) += 1;
        }
    }

    pub fn release(&mut self, tenant: Option<&str>) {
        self.global_active = self.global_active.saturating_sub(1);
        if let Some(t) = tenant
            && let Some(n) = self.per_tenant.get_mut(t)
        {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.per_tenant.remove(t);
            }
        }
    }
}
