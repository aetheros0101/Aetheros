use super::{git, run};
use crate::error::{Result, WorkspaceError};
use crate::models::GitBlameLine;
use std::path::Path;

/// `git blame --porcelain` for a file. Optional line range (1-based inclusive).
pub fn blame(
    root: &Path,
    path: &str,
    start_line: Option<u32>,
    end_line: Option<u32>,
) -> Result<Vec<GitBlameLine>> {
    let mut c = git(root);
    c.args(["blame", "--porcelain", "-w"]);
    if let (Some(s), Some(e)) = (start_line, end_line) {
        c.args(["-L", &format!("{s},{e}")]);
    } else if let Some(s) = start_line {
        c.args(["-L", &format!("{s},+1")]);
    }
    c.args(["--", path]);
    let output = run(c)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(parse_porcelain(&String::from_utf8_lossy(&output.stdout)))
}

fn parse_porcelain(text: &str) -> Vec<GitBlameLine> {
    let mut out = Vec::new();
    let mut commit = String::new();
    let mut author = String::new();
    let mut author_time: u64 = 0;
    let mut summary = String::new();
    let mut orig_line: u32 = 0;
    let mut final_line: u32 = 0;

    for line in text.lines() {
        if line.starts_with('\t') {
            out.push(GitBlameLine {
                commit: commit.clone(),
                author: author.clone(),
                author_time,
                summary: summary.clone(),
                line: final_line,
                orig_line,
                content: line[1..].to_string(),
            });
        } else if let Some(v) = line.strip_prefix("author ") {
            author = v.to_string();
        } else if let Some(v) = line.strip_prefix("author-time ") {
            author_time = v.parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("summary ") {
            summary = v.to_string();
        } else if let Some((hash, rest)) = line.split_once(' ') {
            // header: <hash> <orig> <final> [<num>]
            if hash.len() >= 7 && hash.chars().all(|c| c.is_ascii_hexdigit()) {
                commit = hash.to_string();
                let mut parts = rest.split_whitespace();
                orig_line = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                final_line = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_blame_porcelain() {
        let sample = "\
abc123def 1 1 1
author Alice
author-time 1700000000
summary First commit
\tfn main() {}
abc123def 2 2
author Alice
author-time 1700000000
summary First commit
\t
";
        let lines = parse_porcelain(sample);
        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].author, "Alice");
        assert_eq!(lines[0].content, "fn main() {}");
        assert_eq!(lines[0].line, 1);
    }
}
