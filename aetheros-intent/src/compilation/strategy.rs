//! Strategy — task *üretmez*; üretilmiş TaskSpec listesini sadeleştirir / sıralar.

use crate::compilation::requirement_tasks::TaskSpec;
use crate::compilation::task_graph::TaskRole;
use crate::requirement::RequirementSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompileStrategy {
    DocsOrChore,
    Research,
    FocusedChange,
    FeatureSlice,
    FullstackProduct,
    Architecture,
}

impl CompileStrategy {
    pub fn select(req: &RequirementSet) -> Self {
        if req.domain_tags.iter().any(|t| t == "docs" || t == "chore") {
            return CompileStrategy::DocsOrChore;
        }
        if req.domain_tags.iter().any(|t| t == "research") {
            return CompileStrategy::Research;
        }
        if req.domain_tags.iter().any(|t| t == "architecture")
            && !req.domain_tags.iter().any(|t| t == "fullstack")
        {
            return CompileStrategy::Architecture;
        }
        if req.domain_tags.iter().any(|t| t == "fullstack")
            || (req.needs_frontend() && req.needs_backend() && req.needs_database())
        {
            return CompileStrategy::FullstackProduct;
        }
        if req.needs_frontend() || req.needs_backend() || req.needs_auth() {
            return CompileStrategy::FeatureSlice;
        }
        CompileStrategy::FocusedChange
    }

    /// Üretilmiş task listesini stratejiye göre filtrele (üretim yok).
    pub fn optimize(&self, specs: Vec<TaskSpec>) -> Vec<TaskSpec> {
        match self {
            CompileStrategy::DocsOrChore => specs
                .into_iter()
                .filter(|s| {
                    matches!(
                        s.node.role,
                        TaskRole::Analyze
                            | TaskRole::Edit
                            | TaskRole::Verification
                            | TaskRole::Custom
                    )
                })
                .collect(),
            CompileStrategy::Research => specs
                .into_iter()
                .filter(|s| {
                    matches!(
                        s.node.role,
                        TaskRole::Analyze
                            | TaskRole::Research
                            | TaskRole::Verification
                            | TaskRole::Custom
                    )
                })
                .collect(),
            // Diğerleri: requirement_tasks çıktısını koru
            _ => specs,
        }
    }
}
