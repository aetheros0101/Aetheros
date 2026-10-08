//! Symbol extraction: pluggable parsers + multi-language heuristics.
//!
//! Host applications can register [`SymbolParser`] implementations backed by
//! tree-sitter grammars (or any other engine). When no parser matches a
//! language, the built-in heuristic extractor is used.

mod heuristic;

pub use heuristic::extract_heuristic;

use crate::models::Symbol;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Language-specific symbol parser (tree-sitter, scrapers, etc.).
pub trait SymbolParser: Send + Sync {
    /// Language ids this parser handles (e.g. `["rust"]`).
    fn language_ids(&self) -> &[&str];

    /// Extract symbols from source text.
    fn extract(&self, path: &str, text: &str) -> Vec<Symbol>;
}

/// Registry of parsers. Thread-safe; shared across the workspace index.
#[derive(Default)]
pub struct SymbolRegistry {
    parsers: Vec<Arc<dyn SymbolParser>>,
    by_lang: HashMap<String, usize>,
}

impl SymbolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, parser: Arc<dyn SymbolParser>) {
        let idx = self.parsers.len();
        for lang in parser.language_ids() {
            self.by_lang.insert((*lang).to_string(), idx);
        }
        self.parsers.push(parser);
    }

    pub fn extract(&self, path: &str, text: &str, language: Option<&str>) -> Vec<Symbol> {
        if let Some(lang) = language {
            if let Some(&idx) = self.by_lang.get(lang) {
                return self.parsers[idx].extract(path, text);
            }
        }
        extract_heuristic(path, text, language)
    }
}

/// Shared registry handle used by the index.
pub type SharedSymbolRegistry = Arc<RwLock<SymbolRegistry>>;

pub fn new_shared_registry() -> SharedSymbolRegistry {
    Arc::new(RwLock::new(SymbolRegistry::new()))
}
