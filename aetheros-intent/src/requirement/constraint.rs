use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConstraint {
    pub id: Uuid,
    pub text: String,
    /// true = hard constraint (derleyici uymak zorunda).
    pub hard: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TechChoice {
    pub name: String,
    pub mandatory: bool,
}
