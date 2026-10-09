use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyKind {
    FinishToStart,
    Soft,
    ParentChild,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DependencyEdge {
    pub from: Uuid,
    pub to: Uuid,
    pub kind: DependencyKind,
}
