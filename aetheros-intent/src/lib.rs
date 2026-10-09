//! # aetheros-intent v0.4 — frozen architecture
//!
//! ```text
//! Natural Language
//!      → Intent Engine (heuristic | model | hybrid)
//!      → UserIntent → Ambiguity
//!      → Requirements (heuristic | model | hybrid)
//!      → Validation
//!      → Task Compilation (requirement_tasks → strategy.optimize → graph)
//!      → TaskGraph → CompiledTask
//!      → TaskLayerAdapter  →  existing Task Layer
//!      → Agent Planner / Runtime
//! ```
//!
//! Vendor LLM bağımlılığı yok: host `StructuredLlm` implement eder.

pub mod adapter;
pub mod ambiguity;
pub mod compilation;
pub mod constraints;
pub mod context;
pub mod errors;
pub mod extraction;
pub mod intent;
pub mod pipeline;
pub mod provider;
pub mod requirement;
pub mod validation;

pub use adapter::{export_to_task_layer, IdentityTaskAdapter, TaskLayerAdapter};
pub use ambiguity::{analyze as analyze_ambiguity, AmbiguityLevel, AmbiguityReport};
pub use compilation::{
    expand_requirement_set, CompileStrategy, CompiledTask, DependencyEdge, DependencyKind,
    TaskGraph, TaskGraphCompiler, TaskNode, TaskPriority, TaskRole, TaskSpec, TaskStatus,
    TaskVerification,
};
pub use constraints::Constraints;
pub use context::{ExtractionContext, ProjectContextSnapshot};
pub use errors::{IntentError, Result};
pub use extraction::{
    HeuristicExtractor, HybridIntentExtractor, IntentExtractor, ModelIntentExtractor,
};
pub use intent::{Intent, IntentKind, UserIntent};
pub use pipeline::{IntentPipeline, PipelineResult};
pub use provider::{
    parse_json, FailingLlm, StaticJsonLlm, StructuredLlm, StructuredRequest, StructuredResponse,
};
pub use requirement::{
    AcceptanceCriterion, Entity, FuncArea, FunctionalRequirement, HeuristicRequirementExtractor,
    HybridRequirementExtractor, ModelRequirementExtractor, NonFunctionalRequirement, Preferences,
    Priority, ProjectConstraint, QualityAttribute, RequirementExtractor, RequirementSet, Scope,
    TechChoice, Unknowns,
};
pub use validation::{validate_domains, validate_graph, validate_requirement_set, ValidationReport};

pub fn compile_intent(
    input: &str,
    constraints: Constraints,
) -> Result<(UserIntent, RequirementSet, TaskGraph, AmbiguityReport)> {
    let r = IntentPipeline::new(constraints).process(input)?;
    Ok((r.intent, r.requirements, r.graph, r.ambiguity))
}

pub fn compile_intent_with_context(
    input: &str,
    constraints: Constraints,
    ctx: &ProjectContextSnapshot,
) -> Result<(UserIntent, RequirementSet, TaskGraph, AmbiguityReport)> {
    let r = IntentPipeline::new(constraints).process_with_context(input, ctx)?;
    Ok((r.intent, r.requirements, r.graph, r.ambiguity))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn readme_not_backend() {
        let r = IntentPipeline::default()
            .process("README dosyasını düzelt")
            .unwrap();
        assert!(!r.graph.nodes.values().any(|n| n.role == TaskRole::Backend));
        assert!(r.graph.nodes.values().any(|n| n.role == TaskRole::Edit));
    }

    #[test]
    fn model_intent_via_static_llm() {
        let json = r#"{"kind":"docs","confidence":0.95,"tags":["readme"],"language":"tr"}"#;
        let llm = Arc::new(StaticJsonLlm {
            content: json.into(),
        });
        let model = ModelIntentExtractor::new(llm);
        let intent = model.extract("README fix").unwrap();
        assert_eq!(intent.kind, IntentKind::Docs);
        assert!(intent.confidence > 0.9);
    }

    #[test]
    fn hybrid_intent_falls_back_on_failing_llm() {
        let model = ModelIntentExtractor::new(Arc::new(FailingLlm));
        let hybrid = HybridIntentExtractor::default().with_model(Box::new(model));
        let intent = hybrid.extract("README dosyasını düzelt").unwrap();
        assert_eq!(intent.kind, IntentKind::Docs);
    }

    #[test]
    fn model_requirements_via_static_llm() {
        let json = r#"{
            "summary":"Auth system",
            "functional":[{"title":"Authentication","description":"login","area":"auth"}],
            "domain_tags":["frontend","backend"],
            "acceptance":["user can login"],
            "unknowns":[]
        }"#;
        let llm = Arc::new(StaticJsonLlm {
            content: json.into(),
        });
        let ext = ModelRequirementExtractor::new(llm);
        let intent = UserIntent::new("auth", IntentKind::Feature, 0.8);
        let set = ext
            .extract(&intent, &ProjectContextSnapshot::default())
            .unwrap();
        assert_eq!(set.functional.len(), 1);
        assert_eq!(set.functional[0].area, FuncArea::Auth);
    }

    #[test]
    fn adapter_identity_export() {
        let r = IntentPipeline::default()
            .process("API endpoint ekle")
            .unwrap();
        let ids = export_to_task_layer(&r.graph, &IdentityTaskAdapter).unwrap();
        assert_eq!(ids.len(), r.graph.nodes.len());
    }

    #[test]
    fn auth_expands_requirement_tasks() {
        let r = IntentPipeline::default()
            .process("Kullanıcı girişi auth ekle")
            .unwrap();
        assert!(r.graph.nodes.len() >= 5);
        assert!(r.graph.nodes.values().any(|n| n.source_requirement.is_some()));
    }
}
