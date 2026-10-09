use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AcceptanceCriterion {
    pub id: Uuid,
    pub text: String,
}
