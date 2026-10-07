use super::{git, run};
use crate::error::{Result, WorkspaceError};
use crate::models::GitCommit;
use std::path::Path;

/// Recent commits. Optional path filter. `max` capped at 200.
pub fn log(root: &Path, max: u32, path: Option<&str>) -> Result<Vec<GitCommit>> {
    let max = max.clamp(1, 200);
    let mut c = git(root);
    // custom format with RS separators
    c.args([
        "log",
        &format!("-n{max}"),
        "--format=%H%x1f%h%x1f%an%x1f%ae%x1f%at%x1f%s%x1e",
    ]);
    if let Some(p) = path {
        c.args(["--", p]);
    }
    let output = run(c)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(parse_log(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_log(text: &str) -> Vec<GitCommit> {
    let mut out = Vec::new();
    for record in text.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        let parts: Vec<&str> = record.split('\x1f').collect();
        if parts.len() < 6 {
            continue;
        }
        out.push(GitCommit {
            hash: parts[0].to_string(),
            short_hash: parts[1].to_string(),
            author: parts[2].to_string(),
            email: parts[3].to_string(),
            timestamp: parts[4].parse().unwrap_or(0),
            subject: parts[5].to_string(),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_log_records() {
        let sample = "aabbcc\x1faabb\x1fBob\x1fbob@x.com\x1f1700000001\x1fFix bug\x1e\nddee\x1fdd\x1fAnn\x1fa@x.com\x1f1700000000\x1fInit\x1e\n";
        let commits = parse_log(sample);
        assert_eq!(commits.len(), 2);
        assert_eq!(commits[0].author, "Bob");
        assert_eq!(commits[0].subject, "Fix bug");
        assert_eq!(commits[1].short_hash, "dd");
    }
}
