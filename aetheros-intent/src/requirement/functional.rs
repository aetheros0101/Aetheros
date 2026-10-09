use crate::requirement::Priority;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FuncArea {
    Auth,
    Ui,
    Api,
    Data,
    Docs,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionalRequirement {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub area: FuncArea,
    pub priority: Priority,
}
