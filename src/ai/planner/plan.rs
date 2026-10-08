use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiExecutionPlan {
    pub objective: String,

    pub steps: Vec<String>,
}
