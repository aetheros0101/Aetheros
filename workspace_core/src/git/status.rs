use super::{git, run};
use crate::error::{Result, WorkspaceError};
use crate::models::{GitStatus, GitStatusEntry, GitStatusKind};
use std::path::Path;

pub fn status(root: &Path) -> Result<GitStatus> {
    let mut probe = git(root);
    probe.args(["rev-parse", "--is-inside-work-tree"]);
    let probe = run(probe)?;
    if !probe.status.success() {
        return Err(WorkspaceError::NotGitRepository);
    }

    let mut b = git(root);
    b.args(["branch", "--show-current"]);
    let branch = run(b)
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    let mut st = git(root);
    st.args(["status", "--porcelain=v1", "-z"]);
    let output = run(st)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }

    let mut entries = Vec::new();
    let mut parts = output
        .stdout
        .split(|b| *b == 0)
        .filter(|p| !p.is_empty())
        .peekable();
    while let Some(raw) = parts.next() {
        if raw.len() < 3 {
            continue;
        }
        let x = raw[0] as char;
        let y = raw[1] as char;
        let mut path = String::from_utf8_lossy(&raw[3..]).replace('\\', "/");
        if x == 'R' || y == 'R' {
            if let Some(new_path) = parts.next() {
                path = String::from_utf8_lossy(new_path).replace('\\', "/");
            }
        }
        let kind = if x == '?' && y == '?' {
            GitStatusKind::Untracked
        } else if x == 'U' || y == 'U' {
            GitStatusKind::Conflicted
        } else if x == 'R' || y == 'R' {
            GitStatusKind::Renamed
        } else if x == 'A' || y == 'A' {
            GitStatusKind::Added
        } else if x == 'D' || y == 'D' {
            GitStatusKind::Deleted
        } else if x == 'T' || y == 'T' {
            GitStatusKind::TypeChanged
        } else if x == 'M' || y == 'M' {
            GitStatusKind::Modified
        } else {
            GitStatusKind::Unknown
        };
        entries.push(GitStatusEntry {
            path,
            kind,
            staged: x != ' ' && x != '?',
            worktree: y != ' ' && y != '?',
        });
    }

    let (ahead, behind) = upstream_counts(root);
    Ok(GitStatus {
        branch,
        ahead,
        behind,
        entries,
        is_git_repo: true,
    })
}

fn upstream_counts(root: &Path) -> (u32, u32) {
    let mut c = git(root);
    c.args(["rev-list", "--left-right", "--count", "HEAD...@{upstream}"]);
    let Ok(output) = run(c) else {
        return (0, 0);
    };
    if !output.status.success() {
        return (0, 0);
    }
    let text = String::from_utf8_lossy(&output.stdout).into_owned();
    let mut it = text.split_whitespace();
    let ahead = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    let behind = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
    (ahead, behind)
}
