//! Gereksinim modeli — tek Intent değil, RequirementSet.

mod acceptance;
mod constraint;
mod functional;
mod non_functional;

pub use acceptance::AcceptanceCriterion;
pub use constraint::{ProjectConstraint, TechChoice};
pub use functional::{FuncArea, FunctionalRequirement};
pub use non_functional::{NonFunctionalRequirement, QualityAttribute};

use crate::context::ProjectContextSnapshot;
use crate::errors::{IntentError, Result};
use crate::intent::{IntentKind, UserIntent};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Priority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

/// Kapsam özeti.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scope {
    pub in_scope: Vec<String>,
    pub out_of_scope: Vec<String>,
}

/// Tercihler (zorunlu değil).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Preferences {
    pub items: Vec<String>,
}

/// Bilinmeyenler / netleştirilmesi gerekenler.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Unknowns {
    pub items: Vec<String>,
}

/// Varlıklar (domain nesneleri).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Entity {
    pub name: String,
    #[serde(default)]
    pub attributes: Vec<String>,
}

/// Tam gereksinim seti.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequirementSet {
    pub id: Uuid,
    pub source_intent_id: Uuid,
    pub summary: String,
    pub priority: Priority,
    pub scope: Scope,
    pub functional: Vec<FunctionalRequirement>,
    pub non_functional: Vec<NonFunctionalRequirement>,
    pub constraints: Vec<ProjectConstraint>,
    pub tech_choices: Vec<TechChoice>,
    pub preferences: Preferences,
    pub unknowns: Unknowns,
    pub entities: Vec<Entity>,
    pub acceptance: Vec<AcceptanceCriterion>,
    #[serde(default)]
    pub domain_tags: Vec<String>,
}

impl RequirementSet {
    pub fn is_minimal(&self) -> bool {
        self.functional.len() <= 1
            && self.entities.is_empty()
            && !self.domain_tags.iter().any(|t| {
                matches!(
                    t.as_str(),
                    "frontend" | "backend" | "database" | "api" | "fullstack"
                )
            })
    }

    pub fn needs_frontend(&self) -> bool {
        self.domain_tags
            .iter()
            .any(|t| t == "frontend" || t == "fullstack" || t == "ui")
            || self
                .functional
                .iter()
                .any(|f| f.area == functional::FuncArea::Ui)
    }

    pub fn needs_backend(&self) -> bool {
        self.domain_tags
            .iter()
            .any(|t| t == "backend" || t == "fullstack" || t == "api")
            || self.functional.iter().any(|f| {
                matches!(
                    f.area,
                    functional::FuncArea::Api
                        | functional::FuncArea::Data
                        | functional::FuncArea::Auth
                )
            })
    }

    pub fn needs_database(&self) -> bool {
        self.domain_tags.iter().any(|t| t == "database")
            || self
                .functional
                .iter()
                .any(|f| f.area == functional::FuncArea::Data)
            || !self.entities.is_empty()
    }

    pub fn needs_auth(&self) -> bool {
        self.functional
            .iter()
            .any(|f| f.area == functional::FuncArea::Auth)
            || self.summary.to_ascii_lowercase().contains("auth")
            || self.summary.to_ascii_lowercase().contains("giriş")
            || self.summary.to_ascii_lowercase().contains("login")
    }

    pub fn needs_testing(&self) -> bool {
        // docs/chore often skip heavy testing in strategy
        !self.domain_tags.iter().any(|t| t == "docs" || t == "chore")
    }
}

/// Requirement çıkarıcı portu.
pub trait RequirementExtractor: Send + Sync {
    fn extract(&self, intent: &UserIntent, ctx: &ProjectContextSnapshot) -> Result<RequirementSet>;
}

/// Sezgisel requirement çıkarıcı.
#[derive(Debug, Default, Clone)]
pub struct HeuristicRequirementExtractor;

impl RequirementExtractor for HeuristicRequirementExtractor {
    fn extract(&self, intent: &UserIntent, ctx: &ProjectContextSnapshot) -> Result<RequirementSet> {
        extract_heuristic(intent, ctx)
    }
}

fn extract_heuristic(intent: &UserIntent, ctx: &ProjectContextSnapshot) -> Result<RequirementSet> {
    let lower = intent.raw_text.to_ascii_lowercase();
    let mut domain_tags = intent.tags.clone();
    let mut functional = Vec::new();
    let mut non_functional = Vec::new();
    let mut constraints = Vec::new();
    let mut tech_choices = Vec::new();
    let mut entities = Vec::new();
    let preferences = Preferences::default();
    let mut unknowns = Unknowns::default();
    let mut acceptance = Vec::new();
    let mut scope = Scope::default();

    scope.in_scope.push(first_sentence(&intent.raw_text));

    // Tech from context + text
    for fw in &ctx.framework_hints {
        tech_choices.push(TechChoice {
            name: fw.clone(),
            mandatory: false,
        });
    }
    detect_tech(&lower, &mut tech_choices, &mut constraints);

    match intent.kind {
        IntentKind::Docs | IntentKind::Chore => {
            domain_tags.push("docs".into());
            if intent.kind == IntentKind::Chore {
                domain_tags.push("chore".into());
            }
            functional.push(FunctionalRequirement {
                id: Uuid::new_v4(),
                title: first_sentence(&intent.raw_text),
                description: intent.raw_text.clone(),
                area: functional::FuncArea::Docs,
                priority: Priority::Low,
            });
            acceptance.push(AcceptanceCriterion {
                id: Uuid::new_v4(),
                text: "Belirtilen dosya/metin güncellendi ve gözden geçirildi".into(),
            });
        }
        IntentKind::Bugfix => {
            domain_tags.push("bugfix".into());
            functional.push(FunctionalRequirement {
                id: Uuid::new_v4(),
                title: first_sentence(&intent.raw_text),
                description: intent.raw_text.clone(),
                area: infer_area(&lower),
                priority: Priority::High,
            });
            acceptance.push(AcceptanceCriterion {
                id: Uuid::new_v4(),
                text: "Hata yeniden üretilemiyor; regresyon testi geçiyor".into(),
            });
            if !lower.contains("test") {
                unknowns.items.push("Hatanın adımları net mi?".into());
            }
        }
        IntentKind::Research => {
            domain_tags.push("research".into());
            functional.push(FunctionalRequirement {
                id: Uuid::new_v4(),
                title: first_sentence(&intent.raw_text),
                description: intent.raw_text.clone(),
                area: functional::FuncArea::Other,
                priority: Priority::Low,
            });
            acceptance.push(AcceptanceCriterion {
                id: Uuid::new_v4(),
                text: "Kısa bulgu notu veya ADR taslağı üretildi".into(),
            });
        }
        IntentKind::Architecture | IntentKind::Refactor => {
            domain_tags.push("architecture".into());
            functional.push(FunctionalRequirement {
                id: Uuid::new_v4(),
                title: first_sentence(&intent.raw_text),
                description: intent.raw_text.clone(),
                area: functional::FuncArea::Other,
                priority: Priority::High,
            });
        }
        IntentKind::Feature | IntentKind::General => {
            // Sosyal / ürün sinyalleri
            let product_signals = [
                ("auth", "Authentication", functional::FuncArea::Auth),
                ("login", "Authentication", functional::FuncArea::Auth),
                ("giriş", "Authentication", functional::FuncArea::Auth),
                ("profile", "Profiles", functional::FuncArea::Ui),
                ("profil", "Profiles", functional::FuncArea::Ui),
                ("post", "Posts", functional::FuncArea::Data),
                ("feed", "Feed", functional::FuncArea::Ui),
                ("like", "Likes", functional::FuncArea::Data),
                ("comment", "Comments", functional::FuncArea::Data),
                ("yorum", "Comments", functional::FuncArea::Data),
            ];
            for (key, title, area) in product_signals {
                if lower.contains(key) {
                    functional.push(FunctionalRequirement {
                        id: Uuid::new_v4(),
                        title: title.into(),
                        description: format!("Detected from intent: {key}"),
                        area,
                        priority: Priority::Medium,
                    });
                    entities.push(Entity {
                        name: title.into(),
                        attributes: vec![],
                    });
                }
            }

            if functional.is_empty() {
                functional.push(FunctionalRequirement {
                    id: Uuid::new_v4(),
                    title: first_sentence(&intent.raw_text),
                    description: intent.raw_text.clone(),
                    area: infer_area(&lower),
                    priority: Priority::Medium,
                });
            }

            // Fullstack heuristics
            let big = [
                "sosyal",
                "social",
                "facebook",
                "site",
                "uygulama",
                "app",
                "platform",
                "marketplace",
                "e-ticaret",
                "ecommerce",
            ];
            if big.iter().any(|k| lower.contains(k)) {
                domain_tags.extend(
                    ["frontend", "backend", "database", "api", "fullstack"].map(str::to_string),
                );
                non_functional.push(NonFunctionalRequirement {
                    id: Uuid::new_v4(),
                    attribute: QualityAttribute::Security,
                    description: "Production-ready security baseline".into(),
                });
                non_functional.push(NonFunctionalRequirement {
                    id: Uuid::new_v4(),
                    attribute: QualityAttribute::Usability,
                    description: "Responsive UI".into(),
                });
            } else {
                match infer_area(&lower) {
                    functional::FuncArea::Ui => domain_tags.push("frontend".into()),
                    functional::FuncArea::Api => {
                        domain_tags.push("backend".into());
                        domain_tags.push("api".into());
                    }
                    functional::FuncArea::Data => {
                        domain_tags.push("backend".into());
                        domain_tags.push("database".into());
                    }
                    functional::FuncArea::Auth => {
                        domain_tags.extend(["frontend", "backend", "api"].map(str::to_string));
                    }
                    functional::FuncArea::Docs => domain_tags.push("docs".into()),
                    functional::FuncArea::Other => {}
                }
            }

            if acceptance.is_empty() {
                acceptance.push(AcceptanceCriterion {
                    id: Uuid::new_v4(),
                    text: format!(
                        "'{}' kullanıcı tarafından doğrulanabilir",
                        first_sentence(&intent.raw_text)
                    ),
                });
            }
        }
    }

    // Dedupe domain tags
    domain_tags.sort();
    domain_tags.dedup();

    if intent.confidence < 0.5 {
        unknowns
            .items
            .push("Intent confidence low — scope may be incomplete".into());
    }

    Ok(RequirementSet {
        id: Uuid::new_v4(),
        source_intent_id: intent.id,
        summary: first_sentence(&intent.raw_text),
        priority: priority_for(intent),
        scope,
        functional,
        non_functional,
        constraints,
        tech_choices,
        preferences,
        unknowns,
        entities,
        acceptance,
        domain_tags,
    })
}

fn priority_for(intent: &UserIntent) -> Priority {
    match intent.kind {
        IntentKind::Bugfix | IntentKind::Architecture => Priority::High,
        IntentKind::Feature | IntentKind::Refactor => Priority::Medium,
        IntentKind::Docs | IntentKind::Chore | IntentKind::Research | IntentKind::General => {
            Priority::Low
        }
    }
}

fn first_sentence(s: &str) -> String {
    let t = s.trim();
    let end = t.find(['.', '!', '?', '\n']).unwrap_or(t.len().min(100));
    t[..end].trim().to_string()
}

fn infer_area(lower: &str) -> functional::FuncArea {
    if lower.contains("readme") || lower.contains(".md") || lower.contains("doc") {
        return functional::FuncArea::Docs;
    }
    if lower.contains("ui") || lower.contains("frontend") || lower.contains("ekran") {
        return functional::FuncArea::Ui;
    }
    if lower.contains("api") || lower.contains("endpoint") {
        return functional::FuncArea::Api;
    }
    if lower.contains("db") || lower.contains("database") || lower.contains("sql") {
        return functional::FuncArea::Data;
    }
    if lower.contains("auth") || lower.contains("login") || lower.contains("giriş") {
        return functional::FuncArea::Auth;
    }
    functional::FuncArea::Other
}

fn detect_tech(lower: &str, tech: &mut Vec<TechChoice>, constraints: &mut Vec<ProjectConstraint>) {
    let pairs = [
        ("react", "React"),
        ("vue", "Vue"),
        ("angular", "Angular"),
        ("postgres", "PostgreSQL"),
        ("postgresql", "PostgreSQL"),
        ("mysql", "MySQL"),
        ("redis", "Redis"),
        ("rust", "Rust"),
        ("typescript", "TypeScript"),
        ("python", "Python"),
    ];
    for (k, name) in pairs {
        if lower.contains(k) {
            tech.push(TechChoice {
                name: name.into(),
                mandatory: true,
            });
            constraints.push(ProjectConstraint {
                id: Uuid::new_v4(),
                text: format!("Use {name}"),
                hard: true,
            });
        }
    }
}

/// Geriye uyum: eski tek Requirement tipi (ince sarmalayıcı).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[deprecated(note = "use RequirementSet")]
pub struct Requirement {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub priority: Priority,
    pub source_intent_id: Uuid,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,
    #[serde(default)]
    pub domain_tags: Vec<String>,
}

// ── Model + Hybrid requirement extractors (v0.4) ─────────────

use crate::provider::{StructuredLlm, StructuredRequest, parse_json};
use std::sync::Arc;

#[derive(Debug, serde::Deserialize)]
struct ReqSetJson {
    summary: Option<String>,
    #[serde(default)]
    functional: Vec<FrJson>,
    #[serde(default)]
    domain_tags: Vec<String>,
    #[serde(default)]
    acceptance: Vec<String>,
    #[serde(default)]
    unknowns: Vec<String>,
}

#[derive(Debug, serde::Deserialize)]
struct FrJson {
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    area: Option<String>,
}

fn area_from(s: &str) -> functional::FuncArea {
    match s.to_ascii_lowercase().as_str() {
        "auth" => functional::FuncArea::Auth,
        "ui" => functional::FuncArea::Ui,
        "api" => functional::FuncArea::Api,
        "data" => functional::FuncArea::Data,
        "docs" => functional::FuncArea::Docs,
        _ => functional::FuncArea::Other,
    }
}

pub struct ModelRequirementExtractor {
    llm: Arc<dyn StructuredLlm>,
}

impl ModelRequirementExtractor {
    pub fn new(llm: Arc<dyn StructuredLlm>) -> Self {
        Self { llm }
    }
}

impl RequirementExtractor for ModelRequirementExtractor {
    fn extract(&self, intent: &UserIntent, ctx: &ProjectContextSnapshot) -> Result<RequirementSet> {
        let system = r#"Extract software requirements as JSON only:
{"summary":"...","functional":[{"title":"...","description":"...","area":"auth|ui|api|data|docs|other"}],"domain_tags":[],"acceptance":["..."],"unknowns":["..."]}"#.into();
        let user = format!(
            "Intent kind: {:?}
Text: {}
Frameworks: {:?}
Paths: {:?}",
            intent.kind, intent.raw_text, ctx.framework_hints, ctx.existing_paths
        );
        let resp = self.llm.complete_structured(&StructuredRequest {
            system,
            user,
            schema_name: Some("requirement_set".into()),
            temperature: Some(0.2),
        })?;
        let parsed: ReqSetJson = parse_json(&resp.content)?;
        let functional: Vec<FunctionalRequirement> = parsed
            .functional
            .into_iter()
            .map(|f| FunctionalRequirement {
                id: Uuid::new_v4(),
                title: f.title,
                description: f.description,
                area: area_from(f.area.as_deref().unwrap_or("other")),
                priority: Priority::Medium,
            })
            .collect();
        if functional.is_empty() {
            return Err(IntentError::RequirementExtraction(
                "model returned no functional requirements".into(),
            ));
        }
        Ok(RequirementSet {
            id: Uuid::new_v4(),
            source_intent_id: intent.id,
            summary: parsed.summary.unwrap_or_else(|| intent.raw_text.clone()),
            priority: Priority::Medium,
            scope: Scope {
                in_scope: vec![intent.raw_text.clone()],
                out_of_scope: vec![],
            },
            functional,
            non_functional: vec![],
            constraints: vec![],
            tech_choices: ctx
                .framework_hints
                .iter()
                .map(|n| TechChoice {
                    name: n.clone(),
                    mandatory: false,
                })
                .collect(),
            preferences: Preferences::default(),
            unknowns: Unknowns {
                items: parsed.unknowns,
            },
            entities: vec![],
            acceptance: parsed
                .acceptance
                .into_iter()
                .map(|text| AcceptanceCriterion {
                    id: Uuid::new_v4(),
                    text,
                })
                .collect(),
            domain_tags: parsed.domain_tags,
        })
    }
}

pub struct HybridRequirementExtractor {
    pub heuristic: HeuristicRequirementExtractor,
    pub model: Option<Box<dyn RequirementExtractor>>,
}

impl Default for HybridRequirementExtractor {
    fn default() -> Self {
        Self {
            heuristic: HeuristicRequirementExtractor,
            model: None,
        }
    }
}

impl HybridRequirementExtractor {
    pub fn with_model(mut self, model: Box<dyn RequirementExtractor>) -> Self {
        self.model = Some(model);
        self
    }
}

impl RequirementExtractor for HybridRequirementExtractor {
    fn extract(&self, intent: &UserIntent, ctx: &ProjectContextSnapshot) -> Result<RequirementSet> {
        let heur = self.heuristic.extract(intent, ctx)?;
        let Some(model) = &self.model else {
            return Ok(heur);
        };
        match model.extract(intent, ctx) {
            Ok(mut m) => {
                // Normalize: keep heuristic domain tags if model sparse
                if m.domain_tags.is_empty() {
                    m.domain_tags = heur.domain_tags.clone();
                }
                if m.acceptance.is_empty() {
                    m.acceptance = heur.acceptance.clone();
                }
                Ok(m)
            }
            Err(_) => Ok(heur),
        }
    }
}
