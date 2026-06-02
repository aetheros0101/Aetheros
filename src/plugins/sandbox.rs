use serde::{
    Deserialize,
    Serialize,
};

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct SandboxPolicy {
    pub max_memory_mb:
        usize,

    pub allow_network:
        bool,

    pub allow_filesystem:
        bool,

    pub networking:
        bool,
        
    pub process_spawn:
        bool,    
}
