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
pub struct ExecutionGraph {
    pub id: Uuid,

    pub nodes:
        Vec<ExecutionNode>,

    pub edges:
        Vec<ExecutionEdge>,

    pub version:
        u64,        
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct ExecutionNode {
    pub id: Uuid,

    pub kind:
        ExecutionNodeKind,

    pub metadata:
        ExecutionMetadata,

    pub correlation_id:
            Option<String>,  

    pub retryable: bool,

    pub dependencies:
        Vec<Uuid>,          
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct ExecutionEdge {
    pub from: Uuid,
    pub to: Uuid,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub enum ExecutionNodeKind {
    Workflow,
    Agent,
    Task,
    Wasm,
    AiInference,
    Plugin,
    RemoteTask,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
    Default,
)]
pub struct ExecutionMetadata {
    pub name: String,

    pub labels:
        std::collections::HashMap<
            String,
            String,
        >,
}
