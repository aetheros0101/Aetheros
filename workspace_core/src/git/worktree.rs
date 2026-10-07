use super::{git, run};
use crate::error::{Result, WorkspaceError};
use crate::models::GitWorktree;
use std::path::Path;

/// `git worktree list --porcelain`
pub fn worktree_list(root: &Path) -> Result<Vec<GitWorktree>> {
    let mut c = git(root);
    c.args(["worktree", "list", "--porcelain"]);
    let output = run(c)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(parse_porcelain(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_porcelain(text: &str) -> Vec<GitWorktree> {
    let mut out = Vec::new();
    let mut current: Option<GitWorktree> = None;

    for line in text.lines() {
        if line.starts_with("worktree ") {
            if let Some(wt) = current.take() {
                out.push(wt);
            }
            current = Some(GitWorktree {
                path: line.trim_start_matches("worktree ").to_string(),
                head: String::new(),
                branch: None,
                bare: false,
                detached: false,
            });
        } else if let Some(wt) = current.as_mut() {
            if line.starts_with("HEAD ") {
                wt.head = line.trim_start_matches("HEAD ").to_string();
            } else if line.starts_with("branch ") {
                let b = line.trim_start_matches("branch ");
                wt.branch = Some(
                    b.strip_prefix("refs/heads/")
                        .unwrap_or(b)
                        .to_string(),
                );
            } else if line == "detached" {
                wt.detached = true;
            } else if line == "bare" {
                wt.bare = true;
            } else if line.is_empty() {
                if let Some(wt) = current.take() {
                    out.push(wt);
                }
            }
        }
    }
    if let Some(wt) = current.take() {
        out.push(wt);
    }
    out
}

/// Yeni worktree ekle. `path` workspace dışına çıkabilir — çağıran
/// SecurityGovernor kontrol etmeli. Bu katman yalnız git komutunu çalıştırır.
pub fn worktree_add(
    root: &Path,
    path: &Path,
    branch: Option<&str>,
    create_branch: bool,
) -> Result<GitWorktree> {
    let mut c = git(root);
    c.args(["worktree", "add"]);
    if let Some(b) = branch {
        if create_branch {
            c.args(["-b", b]);
        } else {
            c.arg(b);
        }
    }
    c.arg(path);
    let output = run(c)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    // Listeleyip eşleşeni döndür
    let list = worktree_list(root)?;
    let target = path.to_string_lossy();
    list.into_iter()
        .find(|w| w.path == target || w.path.ends_with(target.as_ref()))
        .ok_or_else(|| WorkspaceError::Git("worktree eklendi ama listede bulunamadı".into()))
}

pub fn worktree_remove(root: &Path, path: &Path, force: bool) -> Result<()> {
    let mut c = git(root);
    c.args(["worktree", "remove"]);
    if force {
        c.arg("--force");
    }
    c.arg(path);
    let output = run(c)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_worktree_list() {
        let sample = "\
worktree /repo
HEAD abcdef
branch refs/heads/main

worktree /repo-feature
HEAD 123456
branch refs/heads/feature

worktree /repo-detached
HEAD fedcba
detached
";
        let list = parse_porcelain(sample);
        assert_eq!(list.len(), 3);
        assert_eq!(list[0].branch.as_deref(), Some("main"));
        assert!(list[2].detached);
    }
}
