use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo {
    pub root: String,
    pub name: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind {
    File,
    Directory,
    Symlink,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry {
    pub path: String,
    pub name: String,
    pub kind: EntryKind,
    pub size: u64,
    pub hidden: bool,
    pub ignored: bool,
}

/// Editör / agent tarafında kullanılan belge modeli.
/// `version` monotonic document version (her başarılı yazmada artar).
/// `content_hash` içerik bütünlüğü için FNV-1a 64-bit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDocument {
    pub path: String,
    pub content: String,
    pub size: u64,
    pub readonly: bool,
    pub version: u64,
    pub content_hash: u64,
    pub language: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub mtime_ms: u64,
    pub content_hash: u64,
    pub language: Option<String>,
    pub hidden: bool,
    pub version: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub query: String,
    pub case_sensitive: bool,
    pub max_results: u32,
    pub include_hidden: bool,
    /// true ise query bir regex olarak derlenir.
    pub regex: bool,
}

impl Default for SearchOptions {
    fn default() -> Self {
        Self {
            query: String::new(),
            case_sensitive: false,
            max_results: 1000,
            include_hidden: false,
            regex: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch {
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResult {
    pub path: String,
    pub kind: EntryKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchEventKind {
    Created,
    Modified,
    Removed,
    Renamed,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceWatchEvent {
    pub kind: WatchEventKind,
    pub path: String,
    pub old_path: Option<String>,
}

// ── Git ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GitStatusKind {
    Added,
    Modified,
    Deleted,
    Renamed,
    Untracked,
    Conflicted,
    Ignored,
    TypeChanged,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub kind: GitStatusKind,
    pub staged: bool,
    pub worktree: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub entries: Vec<GitStatusEntry>,
    pub is_git_repo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffHunk {
    pub old_start: u32,
    pub old_count: u32,
    pub new_start: u32,
    pub new_count: u32,
    pub header: String,
    pub lines: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiffFile {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub hunks: Vec<GitDiffHunk>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitDiff {
    pub files: Vec<GitDiffFile>,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitWorktree {
    pub path: String,
    pub head: String,
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
}

// ── Symbols ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SymbolKind {
    File,
    Module,
    Namespace,
    Package,
    Class,
    Method,
    Property,
    Field,
    Constructor,
    Enum,
    Interface,
    Function,
    Variable,
    Constant,
    String,
    Number,
    Boolean,
    Array,
    Object,
    Key,
    Null,
    EnumMember,
    Struct,
    Event,
    Operator,
    TypeParameter,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub path: String,
    pub line: u32,
    pub column: u32,
    pub end_line: u32,
    pub end_column: u32,
    pub container: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SymbolQuery {
    pub name: String,
    pub kind: Option<SymbolKind>,
    pub path_prefix: Option<String>,
    pub max_results: u32,
}

impl Default for SymbolQuery {
    fn default() -> Self {
        Self {
            name: String::new(),
            kind: None,
            path_prefix: None,
            max_results: 100,
        }
    }
}

// ── Diagnostics / code actions ───────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextRange {
    pub start_line: u32,
    pub start_column: u32,
    pub end_line: u32,
    pub end_column: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Diagnostic {
    pub path: String,
    pub range: TextRange,
    pub severity: DiagnosticSeverity,
    pub code: Option<String>,
    pub source: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeAction {
    pub title: String,
    pub kind: Option<String>,
    pub path: String,
    pub range: TextRange,
    pub is_preferred: bool,
    /// Uygulama için serileştirilmiş edit listesi (LSP workspace edit özeti).
    pub edits: Vec<TextEdit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextEdit {
    pub path: String,
    pub range: TextRange,
    pub new_text: String,
}

// ── Git blame / log ──────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitBlameLine {
    pub commit: String,
    pub author: String,
    pub author_time: u64,
    pub summary: String,
    pub line: u32,
    pub orig_line: u32,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    pub author: String,
    pub email: String,
    pub timestamp: u64,
    pub subject: String,
}
