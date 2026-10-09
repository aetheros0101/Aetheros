use crate::context::ProjectContextSnapshot;
use crate::errors::Result;
use crate::intent::{IntentKind, UserIntent};
use crate::provider::{parse_json, StructuredLlm, StructuredRequest};
use serde::Deserialize;
use std::sync::Arc;

pub trait IntentExtractor: Send + Sync {
    fn extract(&self, input: &str) -> Result<UserIntent>;

    fn extract_with_context(
        &self,
        input: &str,
        _ctx: &ProjectContextSnapshot,
    ) -> Result<UserIntent> {
        self.extract(input)
    }
}

#[derive(Debug, Deserialize)]
struct IntentJson {
    kind: String,
    confidence: f32,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    language: Option<String>,
}

fn kind_from_str(s: &str) -> IntentKind {
    match s.trim().to_ascii_lowercase().as_str() {
        "feature" => IntentKind::Feature,
        "bugfix" | "bug" | "fix" => IntentKind::Bugfix,
        "refactor" => IntentKind::Refactor,
        "docs" | "doc" => IntentKind::Docs,
        "research" => IntentKind::Research,
        "architecture" => IntentKind::Architecture,
        "chore" => IntentKind::Chore,
        _ => IntentKind::General,
    }
}

/// LLM tabanlı intent çıkarıcı — host `StructuredLlm` bağlar.
pub struct ModelIntentExtractor {
    llm: Arc<dyn StructuredLlm>,
}

impl ModelIntentExtractor {
    pub fn new(llm: Arc<dyn StructuredLlm>) -> Self {
        Self { llm }
    }
}

impl IntentExtractor for ModelIntentExtractor {
    fn extract(&self, input: &str) -> Result<UserIntent> {
        self.extract_with_context(input, &ProjectContextSnapshot::default())
    }

    fn extract_with_context(
        &self,
        input: &str,
        ctx: &ProjectContextSnapshot,
    ) -> Result<UserIntent> {
        let system = r#"You classify software engineering intents.
Respond with ONLY JSON:
{"kind":"feature|bugfix|refactor|docs|research|architecture|chore|general","confidence":0.0-1.0,"tags":[],"language":"en|tr"}"#
            .to_string();
        let user = format!(
            "Input: {input}\nContext language: {:?}\nFrameworks: {:?}\nRules: {:?}",
            ctx.project_language, ctx.framework_hints, ctx.rules
        );
        let resp = self.llm.complete_structured(&StructuredRequest {
            system,
            user,
            schema_name: Some("user_intent".into()),
            temperature: Some(0.1),
        })?;
        let parsed: IntentJson = parse_json(&resp.content)?;
        let mut intent = UserIntent::new(input, kind_from_str(&parsed.kind), parsed.confidence);
        intent.tags = parsed.tags;
        intent.language = parsed.language;
        Ok(intent)
    }
}
