//! Workspace agent tools — aetheros-workspace v0.5+ köprüsü.
//!
//! Her instance tek bir `WorkspaceToolKind` taşır; capability / risk / approval
//! kararı SecurityGovernor hattındadır. Workspace yalnız güvenli primitive uygular.

use std::sync::Arc;

use async_trait::async_trait;
use aetheros_workspace::{Workspace, WorkspaceToolRequest};

use crate::agents::capabilities::AgentCapability;
use crate::types::agent_tool::{AgentTool, CallVerdict, RiskLevel};

/// Agent okumalarının varsayılan / azami bayt sınırı.
const DEFAULT_READ_BYTES: usize = 32 * 1024;
const MAX_READ_BYTES: usize = 128 * 1024;
const MAX_OUTPUT_CHARS: usize = 20_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WorkspaceToolKind {
    // ── read-only ──────────────────────────────────────────
    Read,
    List,
    Search,
    FileMetadata,
    ListMetadata,
    FindSymbols,
    FileSymbols,
    GitStatus,
    GitDiff,
    GitBlame,
    GitLog,
    GitWorktreeList,
    GetDiagnostics,
    // ── write ──────────────────────────────────────────────
    CreateFile,
    CreateDirectory,
    Write,
    Patch,
    ApplyTextEdits,
    // ── mutate (yüksek risk) ───────────────────────────────
    Rename,
    Delete,
}

impl WorkspaceToolKind {
    /// Tüm araçlar (önce salt-okunur).
    pub const ALL: [WorkspaceToolKind; 20] = [
        WorkspaceToolKind::Read,
        WorkspaceToolKind::List,
        WorkspaceToolKind::Search,
        WorkspaceToolKind::FileMetadata,
        WorkspaceToolKind::ListMetadata,
        WorkspaceToolKind::FindSymbols,
        WorkspaceToolKind::FileSymbols,
        WorkspaceToolKind::GitStatus,
        WorkspaceToolKind::GitDiff,
        WorkspaceToolKind::GitBlame,
        WorkspaceToolKind::GitLog,
        WorkspaceToolKind::GitWorktreeList,
        WorkspaceToolKind::GetDiagnostics,
        WorkspaceToolKind::CreateFile,
        WorkspaceToolKind::CreateDirectory,
        WorkspaceToolKind::Write,
        WorkspaceToolKind::Patch,
        WorkspaceToolKind::ApplyTextEdits,
        WorkspaceToolKind::Rename,
        WorkspaceToolKind::Delete,
    ];

    pub fn is_read_only(self) -> bool {
        matches!(
            self,
            Self::Read
                | Self::List
                | Self::Search
                | Self::FileMetadata
                | Self::ListMetadata
                | Self::FindSymbols
                | Self::FileSymbols
                | Self::GitStatus
                | Self::GitDiff
                | Self::GitBlame
                | Self::GitLog
                | Self::GitWorktreeList
                | Self::GetDiagnostics
        )
    }

    pub fn is_mutating(self) -> bool {
        matches!(self, Self::Rename | Self::Delete)
    }

    pub fn is_write(self) -> bool {
        matches!(
            self,
            Self::CreateFile
                | Self::CreateDirectory
                | Self::Write
                | Self::Patch
                | Self::ApplyTextEdits
        )
    }

    pub fn required_capability(self) -> AgentCapability {
        if self.is_mutating() {
            AgentCapability::WorkspaceMutate
        } else if self.is_write() {
            AgentCapability::WorkspaceWrite
        } else {
            AgentCapability::WorkspaceRead
        }
    }

    pub fn tool_name(self) -> &'static str {
        match self {
            Self::Read => "workspace_read",
            Self::List => "workspace_list",
            Self::Search => "workspace_search",
            Self::FileMetadata => "workspace_file_metadata",
            Self::ListMetadata => "workspace_list_metadata",
            Self::FindSymbols => "workspace_find_symbols",
            Self::FileSymbols => "workspace_file_symbols",
            Self::GitStatus => "workspace_git_status",
            Self::GitDiff => "workspace_git_diff",
            Self::GitBlame => "workspace_git_blame",
            Self::GitLog => "workspace_git_log",
            Self::GitWorktreeList => "workspace_git_worktree_list",
            Self::GetDiagnostics => "workspace_get_diagnostics",
            Self::CreateFile => "workspace_create_file",
            Self::CreateDirectory => "workspace_create_directory",
            Self::Write => "workspace_write",
            Self::Patch => "workspace_patch",
            Self::ApplyTextEdits => "workspace_apply_text_edits",
            Self::Rename => "workspace_rename",
            Self::Delete => "workspace_delete",
        }
    }

    /// Yol taşıyan argüman indeksleri (erken path inspect için).
    fn path_arg_indexes(self) -> &'static [usize] {
        match self {
            Self::Search
            | Self::GitStatus
            | Self::GitWorktreeList
            | Self::ListMetadata
            | Self::FindSymbols => &[],
            Self::GitDiff | Self::GitLog | Self::GetDiagnostics => &[], // path optional at 1 or 0
            Self::Rename => &[0, 1],
            Self::GitBlame => &[0],
            _ => &[0],
        }
    }
}

pub struct WorkspaceAgentTool {
    workspace: Arc<Workspace>,
    kind: WorkspaceToolKind,
}

impl WorkspaceAgentTool {
    pub fn new(workspace: Arc<Workspace>, kind: WorkspaceToolKind) -> Self {
        Self { workspace, kind }
    }

    /// Tüm kind'lar için tool seti üret.
    pub fn all_tools(workspace: Arc<Workspace>) -> Vec<Self> {
        WorkspaceToolKind::ALL
            .iter()
            .map(|k| Self::new(workspace.clone(), *k))
            .collect()
    }

    fn arg(&self, args: &[String], index: usize, name: &str) -> Result<String, String> {
        args.get(index)
            .cloned()
            .ok_or_else(|| format!("{}: eksik argüman `{name}`", self.kind.tool_name()))
    }

    fn opt_arg(&self, args: &[String], index: usize) -> Option<String> {
        args.get(index).cloned().filter(|s| !s.is_empty())
    }

    fn request(&self, args: &[String]) -> Result<WorkspaceToolRequest, String> {
        match self.kind {
            WorkspaceToolKind::Read => Ok(WorkspaceToolRequest::ReadFile {
                path: self.arg(args, 0, "path")?,
                max_bytes: Some(
                    args.get(1)
                        .and_then(|v| v.trim().parse::<usize>().ok())
                        .unwrap_or(DEFAULT_READ_BYTES)
                        .min(MAX_READ_BYTES),
                ),
            }),
            WorkspaceToolKind::List => Ok(WorkspaceToolRequest::ListFiles {
                path: self.arg(args, 0, "path")?,
                depth: args.get(1).and_then(|v| v.parse().ok()),
                include_hidden: args.get(2).and_then(|v| parse_bool(v)),
            }),
            WorkspaceToolKind::Search => Ok(WorkspaceToolRequest::Search {
                query: self.arg(args, 0, "query")?,
                case_sensitive: args.get(1).and_then(|v| parse_bool(v)),
                max_results: Some(
                    args.get(2)
                        .and_then(|v| v.trim().parse::<u32>().ok())
                        .unwrap_or(100)
                        .min(500),
                ),
                include_hidden: args.get(3).and_then(|v| parse_bool(v)),
                regex: args.get(4).and_then(|v| parse_bool(v)),
            }),
            WorkspaceToolKind::FileMetadata => Ok(WorkspaceToolRequest::FileMetadata {
                path: self.arg(args, 0, "path")?,
            }),
            WorkspaceToolKind::ListMetadata => Ok(WorkspaceToolRequest::ListMetadata),
            WorkspaceToolKind::FindSymbols => Ok(WorkspaceToolRequest::FindSymbols {
                name: self.arg(args, 0, "name")?,
                kind: None,
                path_prefix: self.opt_arg(args, 1),
                max_results: args.get(2).and_then(|v| v.parse().ok()),
            }),
            WorkspaceToolKind::FileSymbols => Ok(WorkspaceToolRequest::FileSymbols {
                path: self.arg(args, 0, "path")?,
            }),
            WorkspaceToolKind::GitStatus => Ok(WorkspaceToolRequest::GitStatus),
            WorkspaceToolKind::GitDiff => Ok(WorkspaceToolRequest::GitDiff {
                staged: args.first().and_then(|v| parse_bool(v)),
                path: self.opt_arg(args, 1),
            }),
            WorkspaceToolKind::GitBlame => Ok(WorkspaceToolRequest::GitBlame {
                path: self.arg(args, 0, "path")?,
                start_line: args.get(1).and_then(|v| v.parse().ok()),
                end_line: args.get(2).and_then(|v| v.parse().ok()),
            }),
            WorkspaceToolKind::GitLog => Ok(WorkspaceToolRequest::GitLog {
                max: args.first().and_then(|v| v.parse().ok()),
                path: self.opt_arg(args, 1),
            }),
            WorkspaceToolKind::GitWorktreeList => Ok(WorkspaceToolRequest::GitWorktreeList),
            WorkspaceToolKind::GetDiagnostics => Ok(WorkspaceToolRequest::GetDiagnostics {
                path: self.opt_arg(args, 0),
            }),
            WorkspaceToolKind::CreateFile => Ok(WorkspaceToolRequest::CreateFile {
                path: self.arg(args, 0, "path")?,
            }),
            WorkspaceToolKind::CreateDirectory => Ok(WorkspaceToolRequest::CreateDirectory {
                path: self.arg(args, 0, "path")?,
            }),
            WorkspaceToolKind::Write => Ok(WorkspaceToolRequest::WriteFile {
                path: self.arg(args, 0, "path")?,
                content: self.arg(args, 1, "content")?,
                expected_version: args.get(2).and_then(|v| v.parse().ok()),
            }),
            WorkspaceToolKind::Patch => {
                let edits = serde_json::from_str(&self.arg(args, 1, "edits_json")?)
                    .map_err(|e| format!("patch edits_json geçersiz: {e}"))?;
                Ok(WorkspaceToolRequest::ApplyPatch {
                    path: self.arg(args, 0, "path")?,
                    edits,
                    expected_version: args.get(2).and_then(|v| v.parse().ok()),
                })
            }
            WorkspaceToolKind::ApplyTextEdits => {
                let edits = serde_json::from_str(&self.arg(args, 1, "edits_json")?)
                    .map_err(|e| format!("text edits_json geçersiz: {e}"))?;
                Ok(WorkspaceToolRequest::ApplyTextEdits {
                    path: self.arg(args, 0, "path")?,
                    edits,
                    expected_version: args.get(2).and_then(|v| v.parse().ok()),
                })
            }
            WorkspaceToolKind::Rename => Ok(WorkspaceToolRequest::Rename {
                from: self.arg(args, 0, "from")?,
                to: self.arg(args, 1, "to")?,
            }),
            WorkspaceToolKind::Delete => Ok(WorkspaceToolRequest::Delete {
                path: self.arg(args, 0, "path")?,
            }),
        }
    }
}

#[async_trait]
impl AgentTool for WorkspaceAgentTool {
    fn name(&self) -> &'static str {
        self.kind.tool_name()
    }

    fn description(&self) -> &'static str {
        match self.kind {
            WorkspaceToolKind::Read => {
                "Workspace'ten metin dosyası oku. Argümanlar: [yol, (isteğe bağlı) azami_bayt]."
            }
            WorkspaceToolKind::List => {
                "Workspace dosya ağacını listele. Argümanlar: [yol, (ops.) derinlik, (ops.) gizlileri_dahil]."
            }
            WorkspaceToolKind::Search => {
                "Workspace içinde metin/regex ara. Argümanlar: [sorgu, (ops.) case_sensitive, (ops.) max, (ops.) hidden, (ops.) regex]."
            }
            WorkspaceToolKind::FileMetadata => "Dosya metadata (hash, version, dil). Argümanlar: [yol].",
            WorkspaceToolKind::ListMetadata => "İndeksteki tüm dosya metadata listesi. Argümanlar: yok.",
            WorkspaceToolKind::FindSymbols => {
                "Sembol ara. Argümanlar: [name, (ops.) path_prefix, (ops.) max_results]."
            }
            WorkspaceToolKind::FileSymbols => "Dosyadaki semboller. Argümanlar: [yol].",
            WorkspaceToolKind::GitStatus => "Git durumu (branch, porcelain entries). Argümanlar: yok.",
            WorkspaceToolKind::GitDiff => {
                "Git diff. Argümanlar: [(ops.) staged true/false, (ops.) path]."
            }
            WorkspaceToolKind::GitBlame => {
                "Git blame. Argümanlar: [path, (ops.) start_line, (ops.) end_line]."
            }
            WorkspaceToolKind::GitLog => {
                "Git log. Argümanlar: [(ops.) max, (ops.) path]."
            }
            WorkspaceToolKind::GitWorktreeList => "Git worktree listesi. Argümanlar: yok.",
            WorkspaceToolKind::GetDiagnostics => {
                "LSP diagnostics önbelleği. Argümanlar: [(ops.) path]."
            }
            WorkspaceToolKind::CreateFile => {
                "Boş yeni dosya oluştur. Argümanlar: [yol]."
            }
            WorkspaceToolKind::CreateDirectory => {
                "Dizin oluştur. Argümanlar: [yol]."
            }
            WorkspaceToolKind::Write => {
                "Dosya yaz. Argümanlar: [yol, content, (ops.) expected_version]."
            }
            WorkspaceToolKind::Patch => {
                "find/replace patch. Argümanlar: [yol, edits_json, (ops.) expected_version]."
            }
            WorkspaceToolKind::ApplyTextEdits => {
                "LSP text edit listesi uygula. Argümanlar: [yol, edits_json, (ops.) expected_version]."
            }
            WorkspaceToolKind::Rename => {
                "Yeniden adlandır. Argümanlar: [from, to]."
            }
            WorkspaceToolKind::Delete => {
                "Dosya/dizin sil. Argümanlar: [yol]."
            }
        }
    }

    fn required_capability(&self) -> Option<AgentCapability> {
        Some(self.kind.required_capability())
    }

    fn risk_level(&self) -> RiskLevel {
        match self.kind {
            k if k.is_mutating() => RiskLevel::High,
            k if k.is_write() => RiskLevel::Medium,
            _ => RiskLevel::Low,
        }
    }

    fn assess_call(&self, arguments: &[String]) -> Option<CallVerdict> {
        let mut touches_git = false;
        for &idx in self.kind.path_arg_indexes() {
            let Some(raw) = arguments.get(idx) else { continue };
            match inspect_path_arg(raw) {
                PathArg::Bad(reason) => {
                    return Some(CallVerdict::Deny {
                        reason: format!("workspace yolu reddedildi: {reason}"),
                    });
                }
                PathArg::Git => touches_git = true,
                PathArg::Ok => {}
            }
        }

        // Optional path on diff/log/diagnostics
        if matches!(
            self.kind,
            WorkspaceToolKind::GitDiff
                | WorkspaceToolKind::GitLog
                | WorkspaceToolKind::GetDiagnostics
        ) {
            for idx in 0..=1 {
                if let Some(raw) = arguments.get(idx) {
                    if raw.is_empty() || raw == "true" || raw == "false" {
                        continue;
                    }
                    match inspect_path_arg(raw) {
                        PathArg::Bad(reason) => {
                            return Some(CallVerdict::Deny {
                                reason: format!("workspace yolu reddedildi: {reason}"),
                            });
                        }
                        PathArg::Git => touches_git = true,
                        PathArg::Ok => {}
                    }
                }
            }
        }

        if touches_git {
            return Some(if self.kind.is_mutating() || self.kind.is_write() {
                CallVerdict::Deny {
                    reason: ".git dizini agent tarafından değiştirilemez".into(),
                }
            } else {
                CallVerdict::Ask {
                    reason: ".git içeriğini okuma (kimlik bilgisi içerebilir)".into(),
                }
            });
        }

        // Yol temizse bile Rename/Delete her zaman insan onayı ister.
        // (Deny kararları yukarıda döndüğü için onay hattına hiç ulaşmaz.)
        if self.kind.is_mutating() {
            return Some(CallVerdict::Ask {
                reason: format!(
                    "{} yüksek riskli mutasyon; onay gerekli",
                    self.kind.tool_name()
                ),
            });
        }
        None
    }

    async fn invoke(&self, arguments: Vec<String>) -> Result<String, String> {
        let request = self.request(&arguments)?;
        let response = self
            .workspace
            .execute_tool(request)
            .map_err(|e| e.to_string())?;
        let json = serde_json::to_string(&response)
            .map_err(|e| format!("workspace response encode failed: {e}"))?;
        Ok(truncate_output(json))
    }
}

fn parse_bool(s: &str) -> Option<bool> {
    match s.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn truncate_output(s: String) -> String {
    if s.chars().count() <= MAX_OUTPUT_CHARS {
        return s;
    }
    let mut out: String = s.chars().take(MAX_OUTPUT_CHARS).collect();
    out.push_str("…[çıktı kırpıldı; daha dar yol/sorgu kullan]");
    out
}

enum PathArg {
    Ok,
    Git,
    Bad(&'static str),
}

fn inspect_path_arg(raw: &str) -> PathArg {
    if raw.contains('\0') {
        return PathArg::Bad("NUL karakteri");
    }
    let b = raw.as_bytes();
    if raw.starts_with('/') || raw.starts_with('\\') {
        return PathArg::Bad("mutlak yol");
    }
    if b.len() >= 2 && b[1] == b':' && b[0].is_ascii_alphabetic() {
        return PathArg::Bad("sürücü harfli mutlak yol");
    }
    let mut git = false;
    for comp in raw.split(['/', '\\']) {
        if comp == ".." {
            return PathArg::Bad("'..' ile üst dizine çıkış");
        }
        if comp.eq_ignore_ascii_case(".git") {
            git = true;
        }
    }
    if git {
        PathArg::Git
    } else {
        PathArg::Ok
    }
}
