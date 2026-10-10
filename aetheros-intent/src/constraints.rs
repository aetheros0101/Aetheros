use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Constraints {
    #[serde(default = "default_max_depth")]
    pub max_depth: usize,
    #[serde(default = "default_max_parallel")]
    pub max_parallel_branches: usize,
    #[serde(default = "default_true")]
    pub require_verification: bool,
    #[serde(default = "default_true")]
    pub require_testing: bool,
    #[serde(default)]
    pub denied_domains: Vec<String>,
    #[serde(default)]
    pub allowed_domains: Vec<String>,
    pub timebox_minutes: Option<u32>,
}

fn default_max_depth() -> usize {
    8
}
fn default_max_parallel() -> usize {
    4
}
fn default_true() -> bool {
    true
}

impl Default for Constraints {
    fn default() -> Self {
        Self {
            max_depth: 8,
            max_parallel_branches: 4,
            require_verification: true,
            require_testing: true,
            denied_domains: vec![],
            allowed_domains: vec![],
            timebox_minutes: None,
        }
    }
}

impl Constraints {
    pub fn allows_domain(&self, domain: &str) -> bool {
        if self
            .denied_domains
            .iter()
            .any(|d| d.eq_ignore_ascii_case(domain))
        {
            return false;
        }
        if self.allowed_domains.is_empty() {
            return true;
        }
        self.allowed_domains
            .iter()
            .any(|d| d.eq_ignore_ascii_case(domain))
    }
}
