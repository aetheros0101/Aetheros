// ============================================================
// bridge/api/workspace_dev.rs
//
// Workbench'in geliştirici özellikleri: arama, Git ve dosya izleme.
// İnce FRB sarmalayıcıları; mantık `aetheros_workspace` içindedir.
//
// Güvenlik: yollar `Workspace` tarafında doğrulanır (kök dışına çıkış,
// `.git` koruması). Burada ek olarak girdi boyutları sınırlanır.
// ============================================================

use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use aetheros_workspace::{
    GitStatusKind, SearchOptions, WatchEventKind, Workspace, WorkspaceError, WorkspaceWatcher,
};

use crate::bridge::api::error::require_runtime;
use crate::bridge::files::friendly_error;
use crate::bridge::types::{
    GitDiffFileDto, GitDiffHunkDto, GitDiffResponse, GitStatusEntryDto, GitStatusResponse,
    WorkspaceSearchHit, WorkspaceWatchHit,
};

/// Tek aramada dönecek en fazla sonuç.
const MAX_SEARCH_RESULTS: usize = 2000;
/// Arama sorgusu için üst sınır (karakter).
const MAX_QUERY_CHARS: usize = 500;
/// Tek drain çağrısında dönecek en fazla olay.
const MAX_WATCH_EVENTS: usize = 256;
/// Drain'in olay beklerken en fazla bekleyeceği süre.
const WATCH_DRAIN_WAIT: Duration = Duration::from_millis(20);

/// Süreç başına tek izleyici (workspace de süreç başına tektir).
static WATCHER: Mutex<Option<WorkspaceWatcher>> = Mutex::new(None);

fn watcher_slot() -> MutexGuard<'static, Option<WorkspaceWatcher>> {
    // Zehirlenmiş kilit izleyiciyi kullanılmaz kılmasın.
    WATCHER.lock().unwrap_or_else(|e| e.into_inner())
}

fn files_ws() -> Result<Arc<Workspace>, String> {
    let rt = require_runtime()?;
    rt.workspace
        .clone()
        .ok_or_else(|| "Çalışma alanı açılamadı.".to_string())
}

fn err(e: WorkspaceError) -> String {
    friendly_error(e)
}

// ── Arama ───────────────────────────────────────────────────

/// Workspace içinde metin ara. Boş sorgu boş liste döner.
pub fn workspace_search(
    query: String,
    case_sensitive: bool,
    max_results: usize,
    regex: bool,
) -> Result<Vec<WorkspaceSearchHit>, String> {
    if query.trim().is_empty() {
        return Ok(Vec::new());
    }
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(format!(
            "Arama ifadesi çok uzun (en fazla {MAX_QUERY_CHARS} karakter)."
        ));
    }
    let ws = files_ws()?;
    let hits = ws
        .search(SearchOptions {
            query,
            case_sensitive,
            max_results: clamp_results(max_results),
            include_hidden: false,
            regex,
        })
        .map_err(err)?;
    Ok(hits
        .into_iter()
        .map(|m| WorkspaceSearchHit {
            path: m.path,
            line: m.line,
            column: m.column,
            preview: m.preview,
        })
        .collect())
}

fn clamp_results(requested: usize) -> u32 {
    requested.clamp(1, MAX_SEARCH_RESULTS) as u32
}

// ── Git ─────────────────────────────────────────────────────

/// Depo durumu: dal, ahead/behind ve değişen dosyalar.
/// Depo değilse `is_git_repo = false` döner (hata değil).
pub fn workspace_git_status() -> Result<GitStatusResponse, String> {
    let ws = files_ws()?;
    let s = ws.git_status().map_err(err)?;
    Ok(GitStatusResponse {
        branch: s.branch,
        ahead: s.ahead,
        behind: s.behind,
        is_git_repo: s.is_git_repo,
        entries: s
            .entries
            .into_iter()
            .map(|e| GitStatusEntryDto {
                path: e.path,
                kind: git_kind_name(e.kind).to_string(),
                staged: e.staged,
                worktree: e.worktree,
            })
            .collect(),
    })
}

/// `staged = true` → sahnelenmiş değişiklikler; `path` verilirse tek dosya.
pub fn workspace_git_diff(staged: bool, path: Option<String>) -> Result<GitDiffResponse, String> {
    let ws = files_ws()?;
    let d = ws.git_diff(staged, path.as_deref()).map_err(err)?;
    Ok(GitDiffResponse {
        staged: d.staged,
        files: d
            .files
            .into_iter()
            .map(|f| GitDiffFileDto {
                path: f.path,
                old_path: f.old_path,
                status: f.status,
                hunks: f
                    .hunks
                    .into_iter()
                    .map(|h| GitDiffHunkDto {
                        header: h.header,
                        lines: h.lines,
                    })
                    .collect(),
            })
            .collect(),
    })
}

/// Dosyayı sahneye al (`git add -A -- path`).
pub fn workspace_git_stage(path: String) -> Result<(), String> {
    let ws = files_ws()?;
    ws.git_stage(&path).map_err(err)
}

/// Dosyanın sahnelenmiş değişikliğini geri al.
pub fn workspace_git_unstage(path: String) -> Result<(), String> {
    let ws = files_ws()?;
    ws.git_unstage(&path).map_err(err)
}

/// Sahnelenmiş değişikliklerle commit oluştur.
pub fn workspace_git_commit(message: String) -> Result<(), String> {
    let ws = files_ws()?;
    ws.git_commit(&message).map_err(err)
}

/// Arayüzün (SCM listesi / rozetler) tanıdığı küçük harfli ad.
fn git_kind_name(kind: GitStatusKind) -> &'static str {
    match kind {
        GitStatusKind::Added => "added",
        GitStatusKind::Deleted => "deleted",
        GitStatusKind::Renamed => "renamed",
        GitStatusKind::Untracked => "untracked",
        GitStatusKind::Conflicted => "conflict",
        GitStatusKind::Ignored => "ignored",
        GitStatusKind::Modified | GitStatusKind::TypeChanged | GitStatusKind::Unknown => "modified",
    }
}

// ── Dosya izleme ────────────────────────────────────────────

/// Dosya izleyiciyi başlat. Zaten çalışıyorsa bir şey yapmaz.
pub fn workspace_watch_start() -> Result<(), String> {
    let ws = files_ws()?;
    let mut slot = watcher_slot();
    if slot.is_some() {
        return Ok(());
    }
    *slot = Some(ws.start_watcher().map_err(err)?);
    Ok(())
}

/// Biriken dosya olaylarını al (yol başına birleştirilmiş). İzleyici
/// başlamadıysa boş liste döner. Arama dizini olaylarla güncel tutulur.
pub fn workspace_watch_drain(max_events: usize) -> Result<Vec<WorkspaceWatchHit>, String> {
    let ws = files_ws()?;
    let events = {
        let slot = watcher_slot();
        match slot.as_ref() {
            Some(w) => w.drain_coalesced(WATCH_DRAIN_WAIT, max_events.clamp(1, MAX_WATCH_EVENTS)),
            None => return Ok(Vec::new()),
        }
    };
    let events: Vec<_> = events
        .into_iter()
        .filter(|e| !is_git_internal(&e.path))
        .collect();
    if events.is_empty() {
        return Ok(Vec::new());
    }
    // Dizin güncellemesi başarısız olsa da olaylar arayüze iletilir.
    if let Err(e) = ws.apply_watch_events(&events) {
        tracing::warn!(err = %e, "workspace izleyici: dizin güncellenemedi");
    }
    Ok(events
        .into_iter()
        .map(|e| WorkspaceWatchHit {
            kind: watch_kind_name(e.kind).to_string(),
            path: e.path,
            old_path: e.old_path,
        })
        .collect())
}

/// `.git` altındaki dahili dosya değişiklikleri arayüzü boğmasın.
fn is_git_internal(path: &str) -> bool {
    let p = path.trim_start_matches("./");
    p == ".git" || p.starts_with(".git/")
}

fn watch_kind_name(kind: WatchEventKind) -> &'static str {
    match kind {
        WatchEventKind::Created => "created",
        WatchEventKind::Removed => "removed",
        WatchEventKind::Renamed => "renamed",
        WatchEventKind::Modified | WatchEventKind::Other => "modified",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_limit_is_clamped() {
        assert_eq!(clamp_results(0), 1);
        assert_eq!(clamp_results(200), 200);
        assert_eq!(clamp_results(usize::MAX), MAX_SEARCH_RESULTS as u32);
    }

    #[test]
    fn git_kind_names_match_ui_vocabulary() {
        assert_eq!(git_kind_name(GitStatusKind::Conflicted), "conflict");
        assert_eq!(git_kind_name(GitStatusKind::Untracked), "untracked");
        assert_eq!(git_kind_name(GitStatusKind::TypeChanged), "modified");
        assert_eq!(git_kind_name(GitStatusKind::Added), "added");
    }

    #[test]
    fn watch_kind_names_match_ui_vocabulary() {
        assert_eq!(watch_kind_name(WatchEventKind::Created), "created");
        assert_eq!(watch_kind_name(WatchEventKind::Other), "modified");
        assert_eq!(watch_kind_name(WatchEventKind::Renamed), "renamed");
    }

    #[test]
    fn git_internal_paths_are_filtered() {
        assert!(is_git_internal(".git"));
        assert!(is_git_internal(".git/index"));
        assert!(is_git_internal("./.git/HEAD"));
        assert!(!is_git_internal(".gitignore"));
        assert!(!is_git_internal("src/.git_notes"));
    }

    #[test]
    fn empty_query_returns_empty_without_runtime() {
        // Çalışma zamanı başlatılmadan da boş sorgu hata vermez.
        assert!(
            workspace_search("  ".into(), false, 10, false)
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn drain_without_runtime_reports_not_initialized() {
        // Çalışma zamanı yoksa net bir hata döner (panik yok).
        if crate::bridge::state::get_runtime().is_none() {
            assert!(workspace_watch_drain(8).is_err());
        }
    }
}
