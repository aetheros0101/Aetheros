use crate::{GitStatus, Result, SearchMatch, SearchOptions, Workspace, WorkspaceEntry, WorkspaceError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WorkspaceToolRequest {
    ReadFile { path: String, max_bytes: Option<usize> },
    ListFiles { path: String, depth: Option<usize>, include_hidden: Option<bool> },
    Search { query: String, case_sensitive: Option<bool>, max_results: Option<u32>, include_hidden: Option<bool> },
    CreateFile { path: String },
    CreateDirectory { path: String },
    WriteFile { path: String, content: String, expected_version: Option<u64> },
    ApplyPatch { path: String, edits: Vec<PatchEdit>, expected_version: Option<u64> },
    Rename { from: String, to: String },
    Delete { path: String },
    GitStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatchEdit {
    pub find: String,
    pub replace: String,
    pub expected_matches: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum WorkspaceToolResponse {
    File(crate::FileDocument),
    // NOT: `tag = "kind"` ile etiketlenmiş enum'da dizi taşıyan newtype varyant
    // serileştirilemez (serde_json hata verir) → struct varyant şart.
    Entries { entries: Vec<WorkspaceEntry> },
    Search { matches: Vec<SearchMatch> },
    Created(crate::CreateResult),
    Written(crate::FileDocument),
    Patched(crate::FileDocument),
    Ok,
    Git(GitStatus),
}

impl Workspace {
    /// Execute one bounded, typed workspace operation. Callers never receive a
    /// filesystem path outside the workspace root because every path operation
    /// is resolved through the core path guard.
    pub fn execute_tool(&self, request: WorkspaceToolRequest) -> Result<WorkspaceToolResponse> {
        match request {
            WorkspaceToolRequest::ReadFile { path, max_bytes } => {
                Ok(WorkspaceToolResponse::File(self.read_file(&path, max_bytes.unwrap_or(2 * 1024 * 1024))?))
            }
            WorkspaceToolRequest::ListFiles { path, depth, include_hidden } => {
                Ok(WorkspaceToolResponse::Entries {
                    entries: self.tree(&path, depth.unwrap_or(3), include_hidden.unwrap_or(false))?,
                })
            }
            WorkspaceToolRequest::Search { query, case_sensitive, max_results, include_hidden } => {
                Ok(WorkspaceToolResponse::Search {
                    matches: self.search(SearchOptions {
                        query,
                        case_sensitive: case_sensitive.unwrap_or(false),
                        max_results: max_results.unwrap_or(1000).min(10_000),
                        include_hidden: include_hidden.unwrap_or(false),
                    })?,
                })
            }
            WorkspaceToolRequest::CreateFile { path } => Ok(WorkspaceToolResponse::Created(self.create_file(&path)?)),
            WorkspaceToolRequest::CreateDirectory { path } => Ok(WorkspaceToolResponse::Created(self.create_dir(&path)?)),
            WorkspaceToolRequest::WriteFile { path, content, expected_version } => {
                self.ensure_version(&path, expected_version)?;
                Ok(WorkspaceToolResponse::Written(self.write_file(&path, &content)?))
            }
            WorkspaceToolRequest::ApplyPatch { path, edits, expected_version } => {
                let document = self.read_file(&path, 8 * 1024 * 1024)?;
                if let Some(expected) = expected_version {
                    if expected != document.version {
                        return Err(WorkspaceError::VersionConflict { path, expected, actual: document.version });
                    }
                }
                let patched = apply_edits(&document.content, &edits)?;
                Ok(WorkspaceToolResponse::Patched(self.write_file(&document.path, &patched)?))
            }
            WorkspaceToolRequest::Rename { from, to } => {
                self.rename(&from, &to)?;
                Ok(WorkspaceToolResponse::Ok)
            }
            WorkspaceToolRequest::Delete { path } => {
                self.delete(&path)?;
                Ok(WorkspaceToolResponse::Ok)
            }
            WorkspaceToolRequest::GitStatus => Ok(WorkspaceToolResponse::Git(self.git_status()?)),
        }
    }

    fn ensure_version(&self, path: &str, expected: Option<u64>) -> Result<()> {
        if let Some(expected) = expected {
            let resolved = self.resolve(path)?;
            let metadata = std::fs::metadata(&resolved).map_err(|e| {
                if e.kind() == std::io::ErrorKind::NotFound {
                    WorkspaceError::NotFound(path.to_owned())
                } else {
                    WorkspaceError::Io(e)
                }
            })?;
            if !metadata.is_file() {
                return Err(WorkspaceError::IsDirectory);
            }
            let actual = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0);
            if expected != actual {
                return Err(WorkspaceError::VersionConflict { path: path.to_owned(), expected, actual });
            }
        }
        Ok(())
    }
}

fn apply_edits(original: &str, edits: &[PatchEdit]) -> Result<String> {
    let mut result = original.to_owned();
    for edit in edits {
        if edit.find.is_empty() {
            return Err(WorkspaceError::InvalidPatch("patch search text cannot be empty".into()));
        }
        let count = result.match_indices(&edit.find).count() as u32;
        let expected = edit.expected_matches.unwrap_or(1);
        if count != expected {
            return Err(WorkspaceError::InvalidPatch(format!(
                "expected {expected} match(es) for patch, found {count}"
            )));
        }
        result = result.replace(&edit.find, &edit.replace);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn patch_requires_unambiguous_match() {
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        w.create_file("main.rs").unwrap();
        w.write_file("main.rs", "let x = 1;\nlet x = 1;\n").unwrap();
        let err = w.execute_tool(WorkspaceToolRequest::ApplyPatch {
            path: "main.rs".into(),
            edits: vec![PatchEdit { find: "let x".into(), replace: "let y".into(), expected_matches: None }],
            expected_version: None,
        }).unwrap_err();
        assert!(matches!(err, WorkspaceError::InvalidPatch(_)));
    }

    #[test]
    fn patch_and_version_guard_work() {
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        w.create_file("main.rs").unwrap();
        let first = w.write_file("main.rs", "hello world\n").unwrap();
        let response = w.execute_tool(WorkspaceToolRequest::ApplyPatch {
            path: "main.rs".into(),
            edits: vec![PatchEdit { find: "world".into(), replace: "AetherOS".into(), expected_matches: None }],
            expected_version: Some(first.version),
        }).unwrap();
        match response { WorkspaceToolResponse::Patched(d) => assert!(d.content.contains("AetherOS")), _ => panic!() }

        let err = w.execute_tool(WorkspaceToolRequest::WriteFile {
            path: "main.rs".into(), content: "x".into(), expected_version: Some(first.version)
        }).unwrap_err();
        assert!(matches!(err, WorkspaceError::VersionConflict { .. }));
    }

    #[test]
    fn every_response_variant_serializes_to_json() {
        // Etiketli enum + dizi taşıyan newtype varyant tuzağına karşı nöbetçi.
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        w.create_file("a.txt").unwrap();
        w.write_file("a.txt", "merhaba dünya\n").unwrap();

        let ls = w.execute_tool(WorkspaceToolRequest::ListFiles {
            path: ".".into(), depth: None, include_hidden: None,
        }).unwrap();
        let json = serde_json::to_string(&ls).unwrap();
        assert!(json.contains("\"kind\":\"entries\"") && json.contains("a.txt"), "{json}");

        let se = w.execute_tool(WorkspaceToolRequest::Search {
            query: "dünya".into(), case_sensitive: None, max_results: None, include_hidden: None,
        }).unwrap();
        let json = serde_json::to_string(&se).unwrap();
        assert!(json.contains("\"kind\":\"search\"") && json.contains("a.txt"), "{json}");

        for r in [
            w.execute_tool(WorkspaceToolRequest::ReadFile { path: "a.txt".into(), max_bytes: None }).unwrap(),
            w.execute_tool(WorkspaceToolRequest::CreateDirectory { path: "d".into() }).unwrap(),
            w.execute_tool(WorkspaceToolRequest::Rename { from: "d".into(), to: "e".into() }).unwrap(),
            w.execute_tool(WorkspaceToolRequest::Delete { path: "e".into() }).unwrap(),
        ] {
            serde_json::to_string(&r).unwrap();
        }
    }
}
