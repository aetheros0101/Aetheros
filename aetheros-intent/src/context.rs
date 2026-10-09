//! Immutable project context snapshot — workspace'e bağımlı değildir.

use serde::{Deserialize, Serialize};

/// Context Engine'den alınan anlık görüntü (DTO).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectContextSnapshot {
    pub workspace_root: Option<String>,
    pub project_language: Option<String>,
    pub framework_hints: Vec<String>,
    pub existing_paths: Vec<String>,
    pub rules: Vec<String>,
    /// Branch / HEAD özeti (Context Engine doldurur).
    #[serde(default)]
    pub git_head: Option<String>,
    #[serde(default)]
    pub open_files: Vec<String>,
}

impl ProjectContextSnapshot {
    pub fn empty() -> Self {
        Self::default()
    }
}

/// Geriye uyum.
pub type ExtractionContext = ProjectContextSnapshot;
