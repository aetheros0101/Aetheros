use super::{git, run};
use crate::error::{Result, WorkspaceError};
use crate::models::{GitDiff, GitDiffFile, GitDiffHunk};
use std::path::Path;

/// `staged = true` → `git diff --cached`, aksi halde worktree diff.
/// `path_filter` None ise tüm repo; Some ise tek path (pathspec).
pub fn diff(root: &Path, staged: bool, path_filter: Option<&str>) -> Result<GitDiff> {
    let mut c = git(root);
    c.args(["diff", "--no-color", "--find-renames"]);
    if staged {
        c.arg("--cached");
    }
    if let Some(p) = path_filter {
        c.args(["--", p]);
    }
    let output = run(c)?;
    if !output.status.success() {
        return Err(WorkspaceError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(GitDiff {
        files: parse_unified_diff(&text),
        staged,
    })
}

fn parse_unified_diff(text: &str) -> Vec<GitDiffFile> {
    let mut files = Vec::new();
    let mut current: Option<GitDiffFile> = None;
    let mut hunk: Option<GitDiffHunk> = None;

    for line in text.lines() {
        if line.starts_with("diff --git ") {
            if let Some(mut f) = current.take() {
                if let Some(h) = hunk.take() {
                    f.hunks.push(h);
                }
                files.push(f);
            }
            // diff --git a/path b/path
            let parts: Vec<&str> = line.split_whitespace().collect();
            let path = parts
                .get(3)
                .map(|p| p.trim_start_matches("b/").to_string())
                .unwrap_or_default();
            current = Some(GitDiffFile {
                path,
                old_path: parts.get(2).map(|p| p.trim_start_matches("a/").to_string()),
                status: "modified".into(),
                hunks: Vec::new(),
            });
        } else if line.starts_with("new file mode") {
            if let Some(f) = current.as_mut() {
                f.status = "added".into();
            }
        } else if line.starts_with("deleted file mode") {
            if let Some(f) = current.as_mut() {
                f.status = "deleted".into();
            }
        } else if line.starts_with("rename from ") {
            if let Some(f) = current.as_mut() {
                f.status = "renamed".into();
                f.old_path = Some(line.trim_start_matches("rename from ").to_string());
            }
        } else if line.starts_with("@@ ") {
            if let Some(f) = current.as_mut() {
                if let Some(h) = hunk.take() {
                    f.hunks.push(h);
                }
            }
            hunk = Some(parse_hunk_header(line));
        } else if line.starts_with('+')
            || line.starts_with('-')
            || line.starts_with(' ')
            || line == "\\ No newline at end of file"
        {
            if let Some(h) = hunk.as_mut() {
                h.lines.push(line.to_string());
            }
        }
    }

    if let Some(mut f) = current.take() {
        if let Some(h) = hunk.take() {
            f.hunks.push(h);
        }
        files.push(f);
    }
    files
}

fn parse_hunk_header(line: &str) -> GitDiffHunk {
    // @@ -old_start,old_count +new_start,new_count @@ optional context
    let mut old_start = 0u32;
    let mut old_count = 0u32;
    let mut new_start = 0u32;
    let mut new_count = 0u32;

    if let Some(rest) = line.strip_prefix("@@ ") {
        let parts: Vec<&str> = rest.split(" @@").collect();
        let ranges = parts.first().copied().unwrap_or("");
        for token in ranges.split_whitespace() {
            if let Some(s) = token.strip_prefix('-') {
                let mut it = s.split(',');
                old_start = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                old_count = it.next().and_then(|v| v.parse().ok()).unwrap_or(1);
            } else if let Some(s) = token.strip_prefix('+') {
                let mut it = s.split(',');
                new_start = it.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                new_count = it.next().and_then(|v| v.parse().ok()).unwrap_or(1);
            }
        }
    }

    GitDiffHunk {
        old_start,
        old_count,
        new_start,
        new_count,
        header: line.to_string(),
        lines: Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_diff() {
        let sample = "\
diff --git a/foo.rs b/foo.rs
index 111..222 100644
--- a/foo.rs
+++ b/foo.rs
@@ -1,3 +1,4 @@
 line1
-line2
+line2changed
 line3
+line4
";
        let files = parse_unified_diff(sample);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].path, "foo.rs");
        assert_eq!(files[0].hunks.len(), 1);
        assert_eq!(files[0].hunks[0].old_start, 1);
        assert_eq!(files[0].hunks[0].new_start, 1);
        assert!(files[0].hunks[0].lines.iter().any(|l| l.starts_with('+')));
    }
}
