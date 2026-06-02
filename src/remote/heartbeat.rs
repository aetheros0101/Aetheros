use chrono::{
    DateTime,
    Utc,
};

use serde::{
    Deserialize,
    Serialize,
};

use uuid::Uuid;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct Heartbeat {
    pub node_id: Uuid,

    pub active_executions:
            usize,
    
    pub memory_usage_mb:
            usize,
    
    pub cpu_usage_percent:
            f32,

    pub load_average:
                f32,        
    
    pub timestamp:
        DateTime<Utc>,
}
