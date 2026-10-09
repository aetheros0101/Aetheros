//! IntentPipeline — dondurulmuş sorumluluk sınırı.

use crate::ambiguity::{analyze as analyze_ambiguity, AmbiguityReport};
use crate::compilation::{TaskGraph, TaskGraphCompiler};
use crate::constraints::Constraints;
use crate::context::ProjectContextSnapshot;
use crate::errors::{IntentError, Result};
use crate::extraction::{HeuristicExtractor, HybridIntentExtractor, IntentExtractor};
use crate::intent::UserIntent;
use crate::requirement::{
    HybridRequirementExtractor, RequirementExtractor,
    RequirementSet,
};
use crate::validation::{validate_graph, validate_requirement_set};

#[derive(Debug, Clone)]
pub struct PipelineResult {
    pub intent: UserIntent,
    pub ambiguity: AmbiguityReport,
    pub requirements: RequirementSet,
    pub graph: TaskGraph,
}

pub struct IntentPipeline {
    pub intent_extractor: Box<dyn IntentExtractor>,
    pub requirement_extractor: Box<dyn RequirementExtractor>,
    pub compiler: TaskGraphCompiler,
    pub block_on_high_ambiguity: bool,
}

impl Default for IntentPipeline {
    fn default() -> Self {
        Self {
            intent_extractor: Box::new(HybridIntentExtractor::default()),
            requirement_extractor: Box::new(HybridRequirementExtractor::default()),
            compiler: TaskGraphCompiler::default(),
            block_on_high_ambiguity: true,
        }
    }
}

impl IntentPipeline {
    pub fn new(constraints: Constraints) -> Self {
        Self {
            compiler: TaskGraphCompiler::new(constraints),
            ..Default::default()
        }
    }

    pub fn with_intent_extractor(mut self, e: Box<dyn IntentExtractor>) -> Self {
        self.intent_extractor = e;
        self
    }

    pub fn with_requirement_extractor(mut self, e: Box<dyn RequirementExtractor>) -> Self {
        self.requirement_extractor = e;
        self
    }

    pub fn process(&self, input: &str) -> Result<PipelineResult> {
        self.process_with_context(input, &ProjectContextSnapshot::default())
    }

    pub fn process_with_context(
        &self,
        input: &str,
        ctx: &ProjectContextSnapshot,
    ) -> Result<PipelineResult> {
        let intent = self
            .intent_extractor
            .extract_with_context(input, ctx)
            .or_else(|_| HeuristicExtractor.extract(input))?;

        let ambiguity = analyze_ambiguity(&intent);
        if self.block_on_high_ambiguity && ambiguity.is_blocking() {
            return Err(IntentError::Ambiguous(
                ambiguity.clarifying_questions.join(" | "),
            ));
        }

        let requirements = self.requirement_extractor.extract(&intent, ctx)?;
        let report = validate_requirement_set(&requirements);
        if !report.ok {
            return Err(IntentError::InvalidRequirement(report.errors.join("; ")));
        }

        let graph = self.compiler.compile(&requirements)?;
        validate_graph(&graph, &self.compiler.constraints)?;

        Ok(PipelineResult {
            intent,
            ambiguity,
            requirements,
            graph,
        })
    }
}
