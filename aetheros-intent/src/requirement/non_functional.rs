use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityAttribute {
    Performance,
    Security,
    Reliability,
    Usability,
    Scalability,
    Maintainability,
    Accessibility,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NonFunctionalRequirement {
    pub id: Uuid,
    pub attribute: QualityAttribute,
    pub description: String,
}
