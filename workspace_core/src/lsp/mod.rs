//! LSP abstraction layer.
//!
//! Concrete language-server process clients plug into [`LanguageServer`].
//! The workspace keeps a diagnostics cache and forwards document sync events
//! after successful reads/writes. This module does not spawn processes by
//! itself — the host application does.

mod client;
pub use client::StdioLanguageServer;

use crate::error::{Result, WorkspaceError};
use crate::models::{CodeAction, Diagnostic, DiagnosticSeverity, TextEdit, TextRange};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, RwLock};

/// Minimal position (0-based, matching LSP).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Language server capability surface used by the coding agent.
pub trait LanguageServer: Send + Sync {
    fn language_ids(&self) -> &[&str];

    fn initialize(&mut self, root: &Path) -> Result<()>;

    fn shutdown(&mut self) -> Result<()>;

    fn did_open(&mut self, path: &str, language_id: &str, text: &str, version: u64) -> Result<()>;

    fn did_change(&mut self, path: &str, text: &str, version: u64) -> Result<()>;

    fn did_close(&mut self, path: &str) -> Result<()>;

    fn hover(&mut self, path: &str, pos: Position) -> Result<Option<String>>;

    fn goto_definition(&mut self, path: &str, pos: Position) -> Result<Vec<TextRange>>;

    fn references(&mut self, path: &str, pos: Position) -> Result<Vec<(String, TextRange)>>;

    fn completion(&mut self, path: &str, pos: Position) -> Result<Vec<CompletionItem>>;

    fn code_actions(&mut self, path: &str, range: TextRange) -> Result<Vec<CodeAction>>;

    /// Pull diagnostics if the server supports it; otherwise empty.
    fn diagnostics(&mut self, path: &str) -> Result<Vec<Diagnostic>>;
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionItem {
    pub label: String,
    pub kind: Option<String>,
    pub detail: Option<String>,
    pub insert_text: Option<String>,
    pub documentation: Option<String>,
}

/// In-memory diagnostics store shared across the workspace.
#[derive(Debug, Default)]
pub struct DiagnosticsStore {
    by_path: HashMap<String, Vec<Diagnostic>>,
}

impl DiagnosticsStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, path: &str, diags: Vec<Diagnostic>) {
        let key = path.replace('\\', "/");
        if diags.is_empty() {
            self.by_path.remove(&key);
        } else {
            self.by_path.insert(key, diags);
        }
    }

    pub fn get(&self, path: Option<&str>) -> Vec<Diagnostic> {
        match path {
            Some(p) => self
                .by_path
                .get(&p.replace('\\', "/"))
                .cloned()
                .unwrap_or_default(),
            None => self.by_path.values().flatten().cloned().collect(),
        }
    }

    pub fn clear(&mut self) {
        self.by_path.clear();
    }

    pub fn clear_path(&mut self, path: &str) {
        self.by_path.remove(&path.replace('\\', "/"));
    }
}

/// Registry of language servers keyed by language id.
pub struct LspRegistry {
    servers: Vec<Box<dyn LanguageServer>>,
    diagnostics: Arc<RwLock<DiagnosticsStore>>,
}

impl LspRegistry {
    pub fn new() -> Self {
        Self {
            servers: Vec::new(),
            diagnostics: Arc::new(RwLock::new(DiagnosticsStore::new())),
        }
    }

    pub fn diagnostics_store(&self) -> Arc<RwLock<DiagnosticsStore>> {
        self.diagnostics.clone()
    }

    pub fn register(&mut self, server: Box<dyn LanguageServer>) {
        self.servers.push(server);
    }

    pub fn initialize_all(&mut self, root: &Path) -> Result<()> {
        for s in &mut self.servers {
            s.initialize(root)?;
        }
        Ok(())
    }

    pub fn shutdown_all(&mut self) -> Result<()> {
        for s in &mut self.servers {
            s.shutdown()?;
        }
        Ok(())
    }

    fn find_mut(&mut self, language_id: &str) -> Option<&mut Box<dyn LanguageServer>> {
        self.servers
            .iter_mut()
            .find(|s| s.language_ids().contains(&language_id))
    }

    pub fn did_open(
        &mut self,
        path: &str,
        language_id: &str,
        text: &str,
        version: u64,
    ) -> Result<()> {
        if let Some(s) = self.find_mut(language_id) {
            s.did_open(path, language_id, text, version)?;
            // Pull diagnostics opportunistically
            if let Ok(diags) = s.diagnostics(path) {
                if let Ok(mut store) = self.diagnostics.write() {
                    store.set(path, diags);
                }
            }
        }
        Ok(())
    }

    pub fn did_change(&mut self, path: &str, language_id: &str, text: &str, version: u64) -> Result<()> {
        if let Some(s) = self.find_mut(language_id) {
            s.did_change(path, text, version)?;
            if let Ok(diags) = s.diagnostics(path) {
                if let Ok(mut store) = self.diagnostics.write() {
                    store.set(path, diags);
                }
            }
        }
        Ok(())
    }

    pub fn did_close(&mut self, path: &str, language_id: &str) -> Result<()> {
        if let Some(s) = self.find_mut(language_id) {
            s.did_close(path)?;
        }
        if let Ok(mut store) = self.diagnostics.write() {
            store.clear_path(path);
        }
        Ok(())
    }

    pub fn hover(&mut self, path: &str, language_id: &str, pos: Position) -> Result<Option<String>> {
        match self.find_mut(language_id) {
            Some(s) => s.hover(path, pos),
            None => Ok(None),
        }
    }

    pub fn goto_definition(
        &mut self,
        path: &str,
        language_id: &str,
        pos: Position,
    ) -> Result<Vec<TextRange>> {
        match self.find_mut(language_id) {
            Some(s) => s.goto_definition(path, pos),
            None => Ok(Vec::new()),
        }
    }

    pub fn completion(
        &mut self,
        path: &str,
        language_id: &str,
        pos: Position,
    ) -> Result<Vec<CompletionItem>> {
        match self.find_mut(language_id) {
            Some(s) => s.completion(path, pos),
            None => Ok(Vec::new()),
        }
    }

    pub fn code_actions(
        &mut self,
        path: &str,
        language_id: &str,
        range: TextRange,
    ) -> Result<Vec<CodeAction>> {
        match self.find_mut(language_id) {
            Some(s) => s.code_actions(path, range),
            None => Ok(Vec::new()),
        }
    }

    pub fn get_diagnostics(&self, path: Option<&str>) -> Result<Vec<Diagnostic>> {
        self.diagnostics
            .read()
            .map(|s| s.get(path))
            .map_err(|_| WorkspaceError::Lsp("diagnostics lock poisoned".into()))
    }
}

impl Default for LspRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Null server for tests / when no LSP is configured.
pub struct NullLanguageServer {
    langs: Vec<&'static str>,
}

impl NullLanguageServer {
    pub fn new(langs: &[&'static str]) -> Self {
        Self {
            langs: langs.to_vec(),
        }
    }
}

impl LanguageServer for NullLanguageServer {
    fn language_ids(&self) -> &[&str] {
        &self.langs
    }
    fn initialize(&mut self, _root: &Path) -> Result<()> {
        Ok(())
    }
    fn shutdown(&mut self) -> Result<()> {
        Ok(())
    }
    fn did_open(&mut self, _: &str, _: &str, _: &str, _: u64) -> Result<()> {
        Ok(())
    }
    fn did_change(&mut self, _: &str, _: &str, _: u64) -> Result<()> {
        Ok(())
    }
    fn did_close(&mut self, _: &str) -> Result<()> {
        Ok(())
    }
    fn hover(&mut self, _: &str, _: Position) -> Result<Option<String>> {
        Ok(None)
    }
    fn goto_definition(&mut self, _: &str, _: Position) -> Result<Vec<TextRange>> {
        Ok(Vec::new())
    }
    fn references(&mut self, _: &str, _: Position) -> Result<Vec<(String, TextRange)>> {
        Ok(Vec::new())
    }
    fn completion(&mut self, _: &str, _: Position) -> Result<Vec<CompletionItem>> {
        Ok(Vec::new())
    }
    fn code_actions(&mut self, _: &str, _: TextRange) -> Result<Vec<CodeAction>> {
        Ok(Vec::new())
    }
    fn diagnostics(&mut self, _: &str) -> Result<Vec<Diagnostic>> {
        Ok(Vec::new())
    }
}

/// Helper: map internal severity strings.
pub fn severity_from_lsp(n: i32) -> DiagnosticSeverity {
    match n {
        1 => DiagnosticSeverity::Error,
        2 => DiagnosticSeverity::Warning,
        3 => DiagnosticSeverity::Information,
        _ => DiagnosticSeverity::Hint,
    }
}

/// Apply a list of text edits to a document (sorted reverse so offsets stay valid).
pub fn apply_text_edits(content: &str, edits: &[TextEdit]) -> Result<String> {
    // Only support edits that target the same logical document; path is informational.
    let mut lines: Vec<String> = content.lines().map(|l| l.to_string()).collect();
    let had_trailing_newline = content.ends_with('\n');

    let mut sorted = edits.to_vec();
    sorted.sort_by(|a, b| {
        b.range
            .start_line
            .cmp(&a.range.start_line)
            .then(b.range.start_column.cmp(&a.range.start_column))
    });

    for edit in sorted {
        let start_line = edit.range.start_line.saturating_sub(1) as usize;
        let end_line = edit.range.end_line.saturating_sub(1) as usize;
        let start_col = edit.range.start_column.saturating_sub(1) as usize;
        let end_col = edit.range.end_column.saturating_sub(1) as usize;

        if start_line >= lines.len() {
            return Err(WorkspaceError::Lsp(format!(
                "edit start line {} out of range",
                edit.range.start_line
            )));
        }

        if start_line == end_line {
            let line = &lines[start_line];
            let chars: Vec<char> = line.chars().collect();
            if start_col > chars.len() || end_col > chars.len() {
                return Err(WorkspaceError::Lsp("edit column out of range".into()));
            }
            let mut new_line: String = chars[..start_col].iter().collect();
            new_line.push_str(&edit.new_text);
            new_line.extend(chars[end_col..].iter());
            // new_text may contain newlines
            let parts: Vec<String> = new_line.lines().map(|l| l.to_string()).collect();
            if parts.is_empty() {
                lines[start_line] = String::new();
            } else if parts.len() == 1 {
                lines[start_line] = parts[0].clone();
            } else {
                lines.splice(start_line..=start_line, parts);
            }
        } else {
            // Multi-line replace: simplify by joining range and replacing
            if end_line >= lines.len() {
                return Err(WorkspaceError::Lsp("edit end line out of range".into()));
            }
            let first_chars: Vec<char> = lines[start_line].chars().collect();
            let last_chars: Vec<char> = lines[end_line].chars().collect();
            let prefix: String = first_chars[..start_col.min(first_chars.len())]
                .iter()
                .collect();
            let suffix: String = if end_col <= last_chars.len() {
                last_chars[end_col..].iter().collect()
            } else {
                String::new()
            };
            let merged = format!("{}{}{}", prefix, edit.new_text, suffix);
            let parts: Vec<String> = if merged.is_empty() {
                vec![String::new()]
            } else {
                merged.lines().map(|l| l.to_string()).collect()
            };
            lines.splice(start_line..=end_line, parts);
        }
    }

    let mut result = lines.join("\n");
    if had_trailing_newline && !result.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}
