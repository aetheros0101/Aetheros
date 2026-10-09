use crate::intent::UserIntent;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AmbiguityLevel {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AmbiguityReport {
    pub level: AmbiguityLevel,
    pub reasons: Vec<String>,
    pub clarifying_questions: Vec<String>,
}

impl AmbiguityReport {
    pub fn is_blocking(&self) -> bool {
        matches!(self.level, AmbiguityLevel::High)
    }
}

pub fn analyze(intent: &UserIntent) -> AmbiguityReport {
    let text = intent.raw_text.trim();
    let mut reasons = Vec::new();
    let mut questions = Vec::new();
    let lower = text.to_lowercase();

    if text.split_whitespace().count() < 3 {
        reasons.push("input too short".into());
        questions.push("Daha fazla detay verebilir misiniz?".into());
    }

    if intent.confidence < 0.5 {
        reasons.push(format!("low confidence ({:.2})", intent.confidence));
        questions.push("Bu özellik, hata, doküman mı yoksa araştırma mı?".into());
    }

    let vague = ["bir", "şey", "stuff", "something", "yap", "do", "it"];
    let tokens: Vec<&str> = lower.split_whitespace().collect();
    if !tokens.is_empty() && tokens.iter().all(|t| vague.contains(t)) {
        reasons.push("vague wording".into());
        questions.push("Somut hedef veya dosya yolu yazar mısınız?".into());
    }

    let level = match reasons.len() {
        0 => AmbiguityLevel::None,
        1 => AmbiguityLevel::Low,
        2 => AmbiguityLevel::Medium,
        _ => AmbiguityLevel::High,
    };

    AmbiguityReport {
        level,
        reasons,
        clarifying_questions: questions,
    }
}
