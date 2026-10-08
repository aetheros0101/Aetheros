use chrono::{DateTime, Utc};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditLog {
    pub actor: String,

    pub action: String,

    pub success: bool,

    pub timestamp: DateTime<Utc>,
}
