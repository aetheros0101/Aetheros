use serde::{
    Deserialize,
    Serialize,
};

use crate::workflows::node::WorkflowNode;

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct WorkflowGraph {
    pub nodes:
        Vec<WorkflowNode>,
}
