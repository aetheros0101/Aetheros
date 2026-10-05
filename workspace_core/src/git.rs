use crate::{error::{Result, WorkspaceError}, models::{GitStatus, GitStatusEntry, GitStatusKind}};
use std::path::Path;
use std::process::Command;

/// `git`'i TEMİZ bir ortamla ve bilinen tehlikeli yapılandırmalar kapalı olarak
/// çalıştırır. Depo yerel `.git/config`'i (ör. `core.fsmonitor`) bir komut
/// çalıştırmaya zorlayamasın diye komut satırı `-c` ayarları depo ayarını ezer.
/// Ortam sıfırlanır: gizli anahtarlar (`*_API_KEY`) alt sürece geçmez, kullanıcı
/// düzeyi git yapılandırması da (HOME yok) okunmaz.
fn git(root: &Path) -> Command {
    let mut c = Command::new("git");
    c.current_dir(root).env_clear();
    for key in ["PATH", "LANG", "TMPDIR"] {
        if let Ok(v) = std::env::var(key) {
            c.env(key, v);
        }
    }
    c.env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_OPTIONAL_LOCKS", "0")
        .env("GIT_TERMINAL_PROMPT", "0")
        .args(["-c", "core.fsmonitor=false", "-c", "core.hooksPath=/dev/null", "-c", "core.pager=cat"]);
    c
}

fn run(mut c: Command) -> Result<std::process::Output> {
    c.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            WorkspaceError::Git("git programı bulunamadı".into())
        } else {
            WorkspaceError::Io(e)
        }
    })
}

pub fn status(root: &Path) -> Result<GitStatus> {
    let mut probe = git(root);
    probe.args(["rev-parse", "--is-inside-work-tree"]);
    let probe = run(probe)?;
    if !probe.status.success() {
        return Err(WorkspaceError::NotGitRepository);
    }

    let mut b = git(root);
    b.args(["branch", "--show-current"]);
    let branch = run(b).ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut st = git(root);
    st.args(["status", "--porcelain=v1", "-z"]);
    let output = run(st)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(String::from_utf8_lossy(&output.stderr).trim().to_string()));
    }

    let mut entries = Vec::new();
    let mut parts = output.stdout.split(|b| *b == 0).filter(|p| !p.is_empty()).peekable();
    while let Some(raw) = parts.next() {
        if raw.len() < 3 { continue; }
        let x = raw[0] as char;
        let y = raw[1] as char;
        let mut path = String::from_utf8_lossy(&raw[3..]).replace('\\', "/");
        if x == 'R' || y == 'R' {
            if let Some(new_path) = parts.next() { path = String::from_utf8_lossy(new_path).replace('\\', "/"); }
        }
        let kind = if x == '?' && y == '?' { GitStatusKind::Untracked }
            else if x == 'U' || y == 'U' { GitStatusKind::Conflicted }
            else if x == 'R' || y == 'R' { GitStatusKind::Renamed }
            else if x == 'A' || y == 'A' { GitStatusKind::Added }
            else if x == 'D' || y == 'D' { GitStatusKind::Deleted }
            else if x == 'T' || y == 'T' { GitStatusKind::TypeChanged }
            else if x == 'M' || y == 'M' { GitStatusKind::Modified }
            else { GitStatusKind::Unknown };
        entries.push(GitStatusEntry { path, kind, staged: x != ' ' && x != '?', worktree: y != ' ' && y != '?' });
    }

    let (ahead, behind) = upstream_counts(root);
    Ok(GitStatus { branch, ahead, behind, entries, is_git_repo: true })
}

fn upstream_counts(root: &Path) -> (u32, u32) {
    let mut c = git(root);
    c.args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]);
    let Ok(output) = run(c) else { return (0, 0); };
    if !output.status.success() { return (0, 0); }
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut it = text.split_whitespace();
    let ahead = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let behind = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    (ahead, behind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn git_command_has_a_scrubbed_environment_and_safe_overrides() {
        std::env::set_var("AETHEROS_TEST_SECRET_API_KEY", "sirr");
        let c = git(Path::new("."));
        let envs: Vec<_> = c.get_envs().collect();
        assert!(!envs.iter().any(|(k, _)| k.to_string_lossy().contains("SECRET")));
        assert!(!envs.iter().any(|(k, _)| *k == "HOME"));
        let args: Vec<String> = c.get_args().map(|a| a.to_string_lossy().into_owned()).collect();
        assert!(args.contains(&"core.fsmonitor=false".to_string()));
        assert!(args.contains(&"core.hooksPath=/dev/null".to_string()));
    }
}
