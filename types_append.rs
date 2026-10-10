
// ── Workbench: arama / Git / izleme ─────────────────────────

/// Arama sonucu: bir satırda eşleşme.
#[derive(Debug)]
pub struct WorkspaceSearchHit {
    pub path: String,
    /// 1 tabanlı satır.
    pub line: u32,
    pub column: u32,
    pub preview: String,
}

/// Dosya izleme olayı. `kind`: created | modified | removed | renamed.
#[derive(Debug)]
pub struct WorkspaceWatchHit {
    pub kind: String,
    pub path: String,
    pub old_path: Option<String>,
}

/// Git durum satırı. `kind`: added | modified | deleted | renamed |
/// untracked | conflict | ignored.
#[derive(Debug)]
pub struct GitStatusEntryDto {
    pub path: String,
    pub kind: String,
    pub staged: bool,
    pub worktree: bool,
}

#[derive(Debug)]
pub struct GitStatusResponse {
    pub branch: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub is_git_repo: bool,
    pub entries: Vec<GitStatusEntryDto>,
}

#[derive(Debug)]
pub struct GitDiffHunkDto {
    pub header: String,
    pub lines: Vec<String>,
}

#[derive(Debug)]
pub struct GitDiffFileDto {
    pub path: String,
    pub old_path: Option<String>,
    pub status: String,
    pub hunks: Vec<GitDiffHunkDto>,
}

#[derive(Debug)]
pub struct GitDiffResponse {
    pub staged: bool,
    pub files: Vec<GitDiffFileDto>,
}
