//! Incremental workspace index: file metadata, full-text + regex search, symbols.
//!
//! Full rebuild only on open / explicit refresh. Watcher and mutations use
//! per-path update/remove. Content is kept as Arc<str> for cheap sharing.

use crate::error::Result;
use crate::models::{FileMetadata, SearchMatch, SearchOptions, Symbol, SymbolQuery};
use ignore::gitignore::Gitignore;
use regex::RegexBuilder;
use std::collections::HashMap;
use std::fs;

use std::path::Path;
use std::sync::Arc;
use std::time::UNIX_EPOCH;
use walkdir::WalkDir;

pub const MAX_INDEXED_FILE_BYTES: u64 = 1024 * 1024;
pub const MAX_INDEXED_TOTAL_BYTES: usize = 64 * 1024 * 1024;

/// FNV-1a 64-bit — bağımlılıksız, hızlı içerik özeti.
pub fn content_hash(bytes: &[u8]) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut h = FNV_OFFSET;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(FNV_PRIME);
    }
    h
}

pub fn detect_language(path: &Path) -> Option<String> {
    let ext = path.extension()?.to_str()?.to_lowercase();
    let lang = match ext.as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" | "pyi" => "python",
        "go" => "go",
        "c" | "h" => "c",
        "cpp" | "cc" | "cxx" | "hpp" | "hxx" => "cpp",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "swift" => "swift",
        "rb" => "ruby",
        "php" => "php",
        "cs" => "csharp",
        "md" | "mdx" => "markdown",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" | "htm" => "html",
        "css" | "scss" => "css",
        "sql" => "sql",
        "sh" | "bash" | "zsh" => "shell",
        "xml" => "xml",
        "vue" => "vue",
        "svelte" => "svelte",
        _ => return None,
    };
    Some(lang.into())
}

fn mtime_ms(meta: &fs::Metadata) -> u64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n.starts_with('.'))
}

fn read_text_limited(path: &Path) -> Option<(String, fs::Metadata)> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_INDEXED_FILE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    let text = String::from_utf8(bytes).ok()?;
    Some((text, meta))
}

#[derive(Debug, Clone)]
struct IndexedFile {
    text: Arc<str>,
    size: u64,
    mtime_ms: u64,
    content_hash: u64,
    language: Option<String>,
    hidden: bool,
    /// Document version tracked by Workspace (0 until first write through API).
    version: u64,
    symbols: Vec<Symbol>,
}

pub struct WorkspaceIndex {
    files: HashMap<String, IndexedFile>,
    total_bytes: usize,
    /// path → sequential document version (survives re-index of same path).
    versions: HashMap<String, u64>,
    trigrams: crate::trigram::TrigramIndex,
    symbol_registry: Option<crate::symbols::SharedSymbolRegistry>,
}

impl Default for WorkspaceIndex {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            total_bytes: 0,
            versions: HashMap::new(),
            trigrams: crate::trigram::TrigramIndex::new(),
            symbol_registry: None,
        }
    }
}

impl WorkspaceIndex {
    pub fn set_symbol_registry(&mut self, reg: crate::symbols::SharedSymbolRegistry) {
        self.symbol_registry = Some(reg);
    }

    pub fn build(root: &Path, ignore: &Gitignore) -> Result<Self> {
        let mut index = Self::default();
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !(e.file_type().is_dir() && e.file_name() == ".git"));

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if ignore.matched_path_or_any_parents(path, false).is_ignore() {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let rel = rel.to_string_lossy().replace('\\', "/");
            index.index_path(root, &rel, ignore, None);
        }
        Ok(index)
    }

    /// Incremental: re-read one relative path. Preserves document version.
    pub fn update(&mut self, root: &Path, relative: &str, ignore: &Gitignore) {
        let key = relative.replace('\\', "/");
        let prev_version = self
            .files
            .get(&key)
            .map(|f| f.version)
            .or_else(|| self.versions.get(&key).copied())
            .unwrap_or(0);
        self.remove(&key);
        self.index_path(root, &key, ignore, Some(prev_version));
    }

    fn index_path(
        &mut self,
        root: &Path,
        relative: &str,
        ignore: &Gitignore,
        force_version: Option<u64>,
    ) {
        let key = relative.replace('\\', "/");
        if key.split('/').any(|c| c.eq_ignore_ascii_case(".git")) {
            return;
        }
        let path = root.join(&key);
        if !path.is_file() || ignore.matched_path_or_any_parents(&path, false).is_ignore() {
            return;
        }
        let Some((text, meta)) = read_text_limited(&path) else {
            return;
        };
        if self.total_bytes + text.len() > MAX_INDEXED_TOTAL_BYTES {
            return;
        }

        let hash = content_hash(text.as_bytes());
        let language = detect_language(&path);
        let symbols = match &self.symbol_registry {
            Some(reg) => reg
                .read()
                .map(|r| r.extract(&key, &text, language.as_deref()))
                .unwrap_or_else(|_| {
                    crate::symbols::extract_heuristic(&key, &text, language.as_deref())
                }),
            None => crate::symbols::extract_heuristic(&key, &text, language.as_deref()),
        };
        let version = force_version
            .or_else(|| self.versions.get(&key).copied())
            .unwrap_or(0);

        self.total_bytes += text.len();
        self.trigrams.insert(&key, &text);
        self.files.insert(
            key.clone(),
            IndexedFile {
                text: Arc::from(text),
                size: meta.len(),
                mtime_ms: mtime_ms(&meta),
                content_hash: hash,
                language,
                hidden: is_hidden(&path),
                version,
                symbols,
            },
        );
        if version > 0 {
            self.versions.insert(key, version);
        }
    }

    pub fn remove(&mut self, relative: &str) {
        let key = relative.replace('\\', "/");
        self.trigrams.remove(&key);
        if let Some(old) = self.files.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(old.text.len());
            if old.version > 0 {
                self.versions.insert(key, old.version);
            }
        }
    }

    /// Bump document version after a successful write through the API.
    pub fn set_version(&mut self, relative: &str, version: u64) {
        let key = relative.replace('\\', "/");
        self.versions.insert(key.clone(), version);
        if let Some(f) = self.files.get_mut(&key) {
            f.version = version;
        }
    }

    pub fn bump_version(&mut self, relative: &str) -> u64 {
        let key = relative.replace('\\', "/");
        let next = self
            .files
            .get(&key)
            .map(|f| f.version)
            .or_else(|| self.versions.get(&key).copied())
            .unwrap_or(0)
            .saturating_add(1);
        self.set_version(&key, next);
        next
    }

    pub fn get_version(&self, relative: &str) -> u64 {
        let key = relative.replace('\\', "/");
        self.files
            .get(&key)
            .map(|f| f.version)
            .or_else(|| self.versions.get(&key).copied())
            .unwrap_or(0)
    }

    pub fn metadata(&self, relative: &str) -> Option<FileMetadata> {
        let key = relative.replace('\\', "/");
        let f = self.files.get(&key)?;
        Some(FileMetadata {
            path: key,
            size: f.size,
            mtime_ms: f.mtime_ms,
            content_hash: f.content_hash,
            language: f.language.clone(),
            hidden: f.hidden,
            version: f.version,
        })
    }

    pub fn all_metadata(&self) -> Vec<FileMetadata> {
        self.files
            .iter()
            .map(|(path, f)| FileMetadata {
                path: path.clone(),
                size: f.size,
                mtime_ms: f.mtime_ms,
                content_hash: f.content_hash,
                language: f.language.clone(),
                hidden: f.hidden,
                version: f.version,
            })
            .collect()
    }

    pub fn search(&self, opts: &SearchOptions) -> Result<Vec<SearchMatch>> {
        if opts.query.is_empty() || opts.max_results == 0 {
            return Ok(Vec::new());
        }
        let max = opts.max_results as usize;

        if opts.regex {
            return self.search_regex(opts, max);
        }

        let needle = if opts.case_sensitive {
            opts.query.clone()
        } else {
            opts.query.to_lowercase()
        };
        let mut out = Vec::with_capacity(max.min(64));

        // Trigram prefilter for queries >= 3 chars
        let candidate_set: Option<std::collections::HashSet<String>> = if opts.query.len() >= 3 {
            Some(self.trigrams.candidates(&opts.query).into_iter().collect())
        } else {
            None
        };

        for (path, file) in &self.files {
            if let Some(ref cs) = candidate_set {
                if !cs.contains(path) {
                    continue;
                }
            }
            if !opts.include_hidden && file.hidden {
                continue;
            }
            for (line_no, line) in file.text.lines().enumerate() {
                let hay_owned;
                let searchable = if opts.case_sensitive {
                    line
                } else {
                    hay_owned = line.to_lowercase();
                    hay_owned.as_str()
                };
                if let Some(pos) = searchable.find(&needle) {
                    let column = if opts.case_sensitive {
                        pos
                    } else {
                        line.char_indices().take_while(|(i, _)| *i < pos).count()
                    };
                    out.push(SearchMatch {
                        path: path.clone(),
                        line: line_no as u32 + 1,
                        column: column as u32 + 1,
                        preview: line.trim().chars().take(240).collect(),
                    });
                    if out.len() >= max {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }

    fn search_regex(&self, opts: &SearchOptions, max: usize) -> Result<Vec<SearchMatch>> {
        let re = RegexBuilder::new(&opts.query)
            .case_insensitive(!opts.case_sensitive)
            .multi_line(false)
            .build()?;

        let mut out = Vec::with_capacity(max.min(64));
        for (path, file) in &self.files {
            if !opts.include_hidden && file.hidden {
                continue;
            }
            for (line_no, line) in file.text.lines().enumerate() {
                if let Some(m) = re.find(line) {
                    out.push(SearchMatch {
                        path: path.clone(),
                        line: line_no as u32 + 1,
                        column: line[..m.start()].chars().count() as u32 + 1,
                        preview: line.trim().chars().take(240).collect(),
                    });
                    if out.len() >= max {
                        return Ok(out);
                    }
                }
            }
        }
        Ok(out)
    }

    pub fn symbols(&self, query: &SymbolQuery) -> Vec<Symbol> {
        let max = query.max_results as usize;
        if max == 0 {
            return Vec::new();
        }
        let needle = query.name.to_lowercase();
        let mut out = Vec::new();

        for (path, file) in &self.files {
            if let Some(prefix) = &query.path_prefix {
                if !path.starts_with(prefix.as_str()) {
                    continue;
                }
            }
            for sym in &file.symbols {
                if let Some(k) = query.kind {
                    if sym.kind != k {
                        continue;
                    }
                }
                if !needle.is_empty() && !sym.name.to_lowercase().contains(&needle) {
                    continue;
                }
                out.push(sym.clone());
                if out.len() >= max {
                    return out;
                }
            }
        }
        out
    }

    pub fn symbols_in_file(&self, relative: &str) -> Vec<Symbol> {
        self.files
            .get(&relative.replace('\\', "/"))
            .map(|f| f.symbols.clone())
            .unwrap_or_default()
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
    use tempfile::tempdir;

    fn no_ignore(root: &Path) -> Gitignore {
        GitignoreBuilder::new(root).build().unwrap()
    }

    #[test]
    fn incremental_update_preserves_version() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("a.rs"), "fn hello() {}\n").unwrap();
        let ign = no_ignore(root);
        let mut idx = WorkspaceIndex::build(root, &ign).unwrap();
        assert_eq!(idx.get_version("a.rs"), 0);
        idx.bump_version("a.rs");
        assert_eq!(idx.get_version("a.rs"), 1);

        fs::write(root.join("a.rs"), "fn hello() {}\nfn world() {}\n").unwrap();
        idx.update(root, "a.rs", &ign);
        assert_eq!(idx.get_version("a.rs"), 1);
        let syms = idx.symbols_in_file("a.rs");
        assert!(syms.iter().any(|s| s.name == "hello"));
        assert!(syms.iter().any(|s| s.name == "world"));
    }

    #[test]
    fn regex_search_works() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        fs::write(root.join("x.py"), "def foo_bar():\n    pass\n").unwrap();
        let ign = no_ignore(root);
        let idx = WorkspaceIndex::build(root, &ign).unwrap();
        let hits = idx
            .search(&SearchOptions {
                query: r"def\s+foo_\w+".into(),
                case_sensitive: false,
                max_results: 10,
                include_hidden: false,
                regex: true,
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].line, 1);
    }

    #[test]
    fn content_hash_stable() {
        assert_eq!(content_hash(b"hello"), content_hash(b"hello"));
        assert_ne!(content_hash(b"hello"), content_hash(b"world"));
    }
}
