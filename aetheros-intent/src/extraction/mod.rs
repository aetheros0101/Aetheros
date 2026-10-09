mod heuristic;
mod model;

pub use heuristic::HeuristicExtractor;
pub use model::{IntentExtractor, ModelIntentExtractor};

use crate::context::ProjectContextSnapshot;
use crate::errors::Result;
use crate::intent::UserIntent;

/// Heuristic fallback + optional model.
pub struct HybridIntentExtractor {
    pub heuristic: HeuristicExtractor,
    pub model: Option<Box<dyn IntentExtractor>>,
    pub model_confidence_floor: f32,
}

impl Default for HybridIntentExtractor {
    fn default() -> Self {
        Self {
            heuristic: HeuristicExtractor,
            model: None,
            model_confidence_floor: 0.55,
        }
    }
}

impl HybridIntentExtractor {
    pub fn with_model(mut self, model: Box<dyn IntentExtractor>) -> Self {
        self.model = Some(model);
        self
    }
}

impl IntentExtractor for HybridIntentExtractor {
    fn extract(&self, input: &str) -> Result<UserIntent> {
        self.extract_with_context(input, &ProjectContextSnapshot::default())
    }

    fn extract_with_context(
        &self,
        input: &str,
        ctx: &ProjectContextSnapshot,
    ) -> Result<UserIntent> {
        let heur = self.heuristic.extract_with_context(input, ctx)?;
        let Some(model) = &self.model else {
            return Ok(heur);
        };
        match model.extract_with_context(input, ctx) {
            Ok(m) if m.confidence >= self.model_confidence_floor => {
                if m.kind == crate::intent::IntentKind::General
                    && heur.kind != crate::intent::IntentKind::General
                {
                    let mut merged = m;
                    merged.kind = heur.kind;
                    merged.tags.extend(heur.tags);
                    Ok(merged)
                } else {
                    Ok(m)
                }
            }
            Ok(_) | Err(_) => Ok(heur),
        }
    }
}
