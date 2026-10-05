use std::sync::Arc;

use async_trait::async_trait;
use aetheros_workspace::{Workspace, WorkspaceToolRequest};

use crate::agents::capabilities::AgentCapability;
use crate::types::agent_tool::{AgentTool, CallVerdict, RiskLevel};

/// Workspace operasyonlarını mevcut SecurityGovernor hattına taşıyan tool.
///
/// Her tool instance tek bir capability/risk alanına sahiptir. Böylece
/// Workspace'in kendisi güvenlik kararı vermez; yalnızca güvenli filesystem
/// primitive'lerini uygular. Allow/Deny/Approval kararı src governance'tadır.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkspaceToolKind {
    Read,
    List,
    Search,
    CreateFile,
    CreateDirectory,
    Write,
    Patch,
    Rename,
    Delete,
    GitStatus,
}

/// Agent okumalarının varsayılan / azami bayt sınırı.
const DEFAULT_READ_BYTES: usize = 32 * 1024;
const MAX_READ_BYTES: usize = 128 * 1024;

impl WorkspaceToolKind {
    /// Agent'a sunulan tüm araçlar (sıra: önce salt-okunur olanlar).
    pub const ALL: [WorkspaceToolKind; 10] = [
        WorkspaceToolKind::Read,
        WorkspaceToolKind::List,
        WorkspaceToolKind::Search,
        WorkspaceToolKind::GitStatus,
        WorkspaceToolKind::CreateFile,
        WorkspaceToolKind::CreateDirectory,
        WorkspaceToolKind::Write,
        WorkspaceToolKind::Patch,
        WorkspaceToolKind::Rename,
        WorkspaceToolKind::Delete,
    ];

    fn is_mutating(self) -> bool {
        !matches!(
            self,
            WorkspaceToolKind::Read
                | WorkspaceToolKind::List
                | WorkspaceToolKind::Search
                | WorkspaceToolKind::GitStatus
        )
    }

    /// Hangi argüman konumları workspace-göreli YOL taşır.
    fn path_arg_indexes(self) -> &'static [usize] {
        match self {
            WorkspaceToolKind::Search | WorkspaceToolKind::GitStatus => &[],
            WorkspaceToolKind::Rename => &[0, 1],
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

    fn arg(&self, args: &[String], index: usize, name: &str) -> Result<String, String> {
        args.get(index)
            .cloned()
            .ok_or_else(|| format!("workspace {}: eksik argüman {name}", self.name()))
    }

    fn request(&self, args: &[String]) -> Result<WorkspaceToolRequest, String> {
        match self.kind {
            WorkspaceToolKind::Read => Ok(WorkspaceToolRequest::ReadFile {
                path: self.arg(args, 0, "path")?,
                // Çıktı planlayıcı istemine girer: varsayılan küçük, tavan sınırlı.
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
                include_hidden: args.get(2).and_then(|v| v.parse().ok()),
            }),
            WorkspaceToolKind::Search => Ok(WorkspaceToolRequest::Search {
                query: self.arg(args, 0, "query")?,
                case_sensitive: args.get(1).and_then(|v| v.parse().ok()),
                max_results: Some(
                    args.get(2)
                        .and_then(|v| v.trim().parse::<u32>().ok())
                        .unwrap_or(100)
                        .min(200),
                ),
                include_hidden: args.get(3).and_then(|v| v.parse().ok()),
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
                    .map_err(|e| format!("workspace patch edits_json geçersiz: {e}"))?;
                Ok(WorkspaceToolRequest::ApplyPatch {
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
            WorkspaceToolKind::GitStatus => Ok(WorkspaceToolRequest::GitStatus),
        }
    }

    fn capability(&self) -> AgentCapability {
        match self.kind {
            WorkspaceToolKind::Read
            | WorkspaceToolKind::List
            | WorkspaceToolKind::Search
            | WorkspaceToolKind::GitStatus => AgentCapability::WorkspaceRead,
            WorkspaceToolKind::CreateFile
            | WorkspaceToolKind::CreateDirectory
            | WorkspaceToolKind::Write
            | WorkspaceToolKind::Patch => AgentCapability::WorkspaceWrite,
            WorkspaceToolKind::Rename | WorkspaceToolKind::Delete => AgentCapability::WorkspaceMutate,
        }
    }

    fn risk(&self) -> RiskLevel {
        match self.kind {
            WorkspaceToolKind::Read
            | WorkspaceToolKind::List
            | WorkspaceToolKind::Search
            | WorkspaceToolKind::GitStatus => RiskLevel::Low,
            WorkspaceToolKind::CreateFile
            | WorkspaceToolKind::CreateDirectory
            | WorkspaceToolKind::Write
            | WorkspaceToolKind::Patch => RiskLevel::Medium,
            WorkspaceToolKind::Rename | WorkspaceToolKind::Delete => RiskLevel::High,
        }
    }
}

#[async_trait]
impl AgentTool for WorkspaceAgentTool {
    fn name(&self) -> &'static str {
        match self.kind {
            WorkspaceToolKind::Read => "workspace_read",
            WorkspaceToolKind::List => "workspace_list",
            WorkspaceToolKind::Search => "workspace_search",
            WorkspaceToolKind::CreateFile => "workspace_create_file",
            WorkspaceToolKind::CreateDirectory => "workspace_create_directory",
            WorkspaceToolKind::Write => "workspace_write",
            WorkspaceToolKind::Patch => "workspace_patch",
            WorkspaceToolKind::Rename => "workspace_rename",
            WorkspaceToolKind::Delete => "workspace_delete",
            WorkspaceToolKind::GitStatus => "workspace_git_status",
        }
    }

    fn description(&self) -> &'static str {
        match self.kind {
            WorkspaceToolKind::Read => {
                "Workspace'ten metin dosyası oku. Argümanlar: [yol, (isteğe bağlı) azami_bayt]. \
                 Yol workspace köküne görelidir (örn. notlar/a.txt); mutlak yol ve '..' yasak."
            }
            WorkspaceToolKind::List => {
                "Workspace dosya ağacını listele. Argümanlar: [yol, (ops.) derinlik, (ops.) gizlileri_dahil_et true/false]. \
                 Kökü listelemek için yol olarak '.' ver."
            }
            WorkspaceToolKind::Search => {
                "Workspace içinde metin ara. Argümanlar: [aranan_metin, (ops.) büyük_küçük_duyarlı true/false, (ops.) azami_sonuç]."
            }
            WorkspaceToolKind::CreateFile => {
                "Boş yeni dosya oluştur (varsa hata). Argümanlar: [yol]. İçerik yazmak için workspace_write kullan."
            }
            WorkspaceToolKind::CreateDirectory => {
                "Yeni klasör oluştur. Argümanlar: [yol]."
            }
            WorkspaceToolKind::Write => {
                "Dosyanın TÜM içeriğini atomik olarak yaz (yoksa oluşturur). \
                 Argümanlar: [yol, içerik, (ops.) beklenen_sürüm]. Küçük değişiklik için workspace_patch tercih et."
            }
            WorkspaceToolKind::Patch => {
                "Dosyaya birebir eşleşmeli düzenleme uygula. Argümanlar: [yol, düzenlemeler_json, (ops.) beklenen_sürüm]. \
                 düzenlemeler_json = JSON dizi: [{\"find\":\"eski\",\"replace\":\"yeni\",\"expected_matches\":1}]."
            }
            WorkspaceToolKind::Rename => {
                "Dosya/klasörü yeniden adlandır veya taşı. Argümanlar: [eski_yol, yeni_yol]. Onay gerektirir."
            }
            WorkspaceToolKind::Delete => {
                "Dosya/klasörü SİL (geri alınamaz). Argümanlar: [yol]. Onay gerektirir."
            }
            WorkspaceToolKind::GitStatus => {
                "Workspace'in git durumunu oku (dal, değişen dosyalar). Argüman yok."
            }
        }
    }

    fn required_capability(&self) -> Option<AgentCapability> {
        Some(self.capability())
    }

    fn risk_level(&self) -> RiskLevel {
        self.risk()
    }

    fn assess_call(&self, args: &[String]) -> Option<CallVerdict> {
        let mut touches_git = false;
        for &i in self.kind.path_arg_indexes() {
            let Some(raw) = args.get(i) else { continue };
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
        if touches_git {
            // .git yazmaya kapalı; okuma kimlik bilgisi (config/credentials) sızdırabilir → onay.
            return Some(if self.kind.is_mutating() {
                CallVerdict::Deny {
                    reason: ".git dizini agent tarafından değiştirilemez".into(),
                }
            } else {
                CallVerdict::Ask {
                    reason: ".git içeriğini okuma (kimlik bilgisi içerebilir)".into(),
                }
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

/// Araç çıktısı planlayıcı istemine girer; çok büyük listeleri kırp.
const MAX_OUTPUT_CHARS: usize = 20_000;

fn truncate_output(s: String) -> String {
    if s.chars().count() <= MAX_OUTPUT_CHARS {
        return s;
    }
    let mut out: String = s.chars().take(MAX_OUTPUT_CHARS).collect();
    out.push_str("…[çıktı kırpıldı; daha dar bir yol/sorgu kullan]");
    out
}

enum PathArg {
    Ok,
    Git,
    Bad(&'static str),
}

/// Bir yol argümanını ÇALIŞTIRMADAN önce kaba biçimde denetler. Asıl sınır
/// Workspace'in path guard'ıdır (sembolik bağlar dahil); bu, agent'a erken
/// ve anlaşılır ret verir ve onay ekranına hiç düşmemesini sağlar.
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
    if git { PathArg::Git } else { PathArg::Ok }
}
