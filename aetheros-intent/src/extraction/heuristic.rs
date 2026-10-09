use crate::errors::{IntentError, Result};
use crate::extraction::model::IntentExtractor;
use crate::intent::{IntentKind, UserIntent};

#[derive(Debug, Default, Clone)]
pub struct HeuristicExtractor;

impl IntentExtractor for HeuristicExtractor {
    fn extract(&self, input: &str) -> Result<UserIntent> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err(IntentError::EmptyInput);
        }

        let lower = trimmed.to_ascii_lowercase();
        let (kind, confidence, mut tags) = classify(&lower);
        let mut intent = UserIntent::new(trimmed, kind, confidence);
        intent.tags.append(&mut tags);
        intent.language = detect_lang(trimmed);
        Ok(intent)
    }
}

fn classify(lower: &str) -> (IntentKind, f32, Vec<String>) {
    let mut tags = Vec::new();

    // Chore / docs önce (dar kapsam)
    let doc_keys = ["readme", "doküman", "documentation", "changelog", "license"];
    if doc_keys.iter().any(|k| lower.contains(k))
        || ((lower.contains("düzelt") || lower.contains("fix") || lower.contains("update"))
            && (lower.contains(".md") || lower.contains("doc")))
    {
        tags.push("docs".into());
        return (IntentKind::Docs, 0.9, tags);
    }

    let chore_keys = [
        "typo", "yazım", "format", "lint", "gitignore", "whitespace",
    ];
    if chore_keys.iter().any(|k| lower.contains(k)) {
        tags.push("chore".into());
        return (IntentKind::Chore, 0.85, tags);
    }

    let rules: &[(&[&str], IntentKind, f32)] = &[
        (
            &["bug", "hata", "crash", "broken", "düzelt", "fix"],
            IntentKind::Bugfix,
            0.85,
        ),
        (
            &["refactor", "yeniden düzenle", "temizle", "cleanup"],
            IntentKind::Refactor,
            0.8,
        ),
        (
            &["araştır", "research", "spike", "poc", "keşfet"],
            IntentKind::Research,
            0.75,
        ),
        (
            &["mimari", "architecture", "design system", "adr"],
            IntentKind::Architecture,
            0.8,
        ),
        (
            &[
                "ekle", "add", "implement", "özellik", "feature", "yeni", "build",
                "oluştur", "create", "site", "uygulama", "app",
            ],
            IntentKind::Feature,
            0.7,
        ),
    ];

    for (keys, kind, conf) in rules {
        if keys.iter().any(|k| lower.contains(k)) {
            tags.push(format!("{kind:?}").to_ascii_lowercase());
            return (*kind, *conf, tags);
        }
    }

    (IntentKind::General, 0.45, tags)
}

fn detect_lang(text: &str) -> Option<String> {
    let tr = text.chars().filter(|c| "çğıöşüÇĞİÖŞÜ".contains(*c)).count();
    Some(if tr > 0 { "tr".into() } else { "en".into() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn readme_is_docs() {
        let i = HeuristicExtractor.extract("README dosyasını düzelt").unwrap();
        assert_eq!(i.kind, IntentKind::Docs);
    }
}
