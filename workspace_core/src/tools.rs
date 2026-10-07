use crate::models::*;
use crate::{Result, Workspace, WorkspaceError};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum WorkspaceToolRequest {
    ReadFile {
        path: String,
        max_bytes: Option<usize>,
    },
    ListFiles {
        path: String,
        depth: Option<usize>,
        include_hidden: Option<bool>,
    },
    Search {
        query: String,
        case_sensitive: Option<bool>,
        max_results: Option<u32>,
        include_hidden: Option<bool>,
        regex: Option<bool>,
    },
    CreateFile {
        path: String,
    },
    CreateDirectory {
        path: String,
    },
    WriteFile {
        path: String,
        content: String,
        expected_version: Option<u64>,
    },
    ApplyPatch {
        path: String,
        edits: Vec<PatchEdit>,
        expected_version: Option<u64>,
    },
    Rename {
        from: String,
        to: String,
    },
    Delete {
        path: String,
    },
    // Metadata / index
    FileMetadata {
        path: String,
    },
    ListMetadata,
    // Symbols
    FindSymbols {
        name: String,
        kind: Option<SymbolKind>,
        path_prefix: Option<String>,
        max_results: Option<u32>,
    },
    FileSymbols {
        path: String,
    },
    // Git
    GitStatus,
    GitDiff {
        staged: Option<bool>,
        path: Option<String>,
    },
    GitWorktreeList,
    // Diagnostics / code actions (from LSP cache)
    GetDiagnostics {
        path: Option<String>,
    },
    ApplyTextEdits {
        path: String,
        edits: Vec<TextEdit>,
        expected_version: Option<u64>,
    },
    GitBlame {
        path: String,
        start_line: Option<u32>,
        end_line: Option<u32>,
    },
    GitLog {
        max: Option<u32>,
        path: Option<String>,
    },
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
    File(FileDocument),
    Entries {
        entries: Vec<WorkspaceEntry>,
    },
    Search {
        matches: Vec<SearchMatch>,
    },
    Created(CreateResult),
    Written(FileDocument),
    Patched(FileDocument),
    Ok,
    Git(GitStatus),
    Diff(GitDiff),
    Worktrees {
        worktrees: Vec<GitWorktree>,
    },
    Metadata(FileMetadata),
    MetadataList {
        files: Vec<FileMetadata>,
    },
    Symbols {
        symbols: Vec<Symbol>,
    },
    Diagnostics {
        diagnostics: Vec<Diagnostic>,
    },
    Blame {
        lines: Vec<GitBlameLine>,
    },
    Log {
        commits: Vec<GitCommit>,
    },
}

impl Workspace {
    /// Execute one bounded, typed workspace operation.
    pub fn execute_tool(&self, request: WorkspaceToolRequest) -> Result<WorkspaceToolResponse> {
        match request {
            WorkspaceToolRequest::ReadFile { path, max_bytes } => Ok(WorkspaceToolResponse::File(
                self.read_file(&path, max_bytes.unwrap_or(2 * 1024 * 1024))?,
            )),
            WorkspaceToolRequest::ListFiles {
                path,
                depth,
                include_hidden,
            } => Ok(WorkspaceToolResponse::Entries {
                entries: self.tree(&path, depth.unwrap_or(3), include_hidden.unwrap_or(false))?,
            }),
            WorkspaceToolRequest::Search {
                query,
                case_sensitive,
                max_results,
                include_hidden,
                regex,
            } => Ok(WorkspaceToolResponse::Search {
                matches: self.search(SearchOptions {
                    query,
                    case_sensitive: case_sensitive.unwrap_or(false),
                    max_results: max_results.unwrap_or(1000).min(10_000),
                    include_hidden: include_hidden.unwrap_or(false),
                    regex: regex.unwrap_or(false),
                })?,
            }),
            WorkspaceToolRequest::CreateFile { path } => {
                Ok(WorkspaceToolResponse::Created(self.create_file(&path)?))
            }
            WorkspaceToolRequest::CreateDirectory { path } => {
                Ok(WorkspaceToolResponse::Created(self.create_dir(&path)?))
            }
            WorkspaceToolRequest::WriteFile {
                path,
                content,
                expected_version,
            } => {
                self.ensure_version(&path, expected_version)?;
                Ok(WorkspaceToolResponse::Written(
                    self.write_file(&path, &content)?,
                ))
            }
            WorkspaceToolRequest::ApplyPatch {
                path,
                edits,
                expected_version,
            } => {
                let document = self.read_file(&path, 8 * 1024 * 1024)?;
                if let Some(expected) = expected_version {
                    if expected != document.version {
                        return Err(WorkspaceError::VersionConflict {
                            path,
                            expected,
                            actual: document.version,
                        });
                    }
                }
                let patched = apply_edits(&document.content, &edits)?;
                Ok(WorkspaceToolResponse::Patched(
                    self.write_file(&document.path, &patched)?,
                ))
            }
            WorkspaceToolRequest::Rename { from, to } => {
                self.rename(&from, &to)?;
                Ok(WorkspaceToolResponse::Ok)
            }
            WorkspaceToolRequest::Delete { path } => {
                self.delete(&path)?;
                Ok(WorkspaceToolResponse::Ok)
            }
            WorkspaceToolRequest::FileMetadata { path } => {
                let meta = self
                    .file_metadata(&path)?
                    .ok_or_else(|| WorkspaceError::NotFound(path))?;
                Ok(WorkspaceToolResponse::Metadata(meta))
            }
            WorkspaceToolRequest::ListMetadata => Ok(WorkspaceToolResponse::MetadataList {
                files: self.list_metadata()?,
            }),
            WorkspaceToolRequest::FindSymbols {
                name,
                kind,
                path_prefix,
                max_results,
            } => Ok(WorkspaceToolResponse::Symbols {
                symbols: self.find_symbols(SymbolQuery {
                    name,
                    kind,
                    path_prefix,
                    max_results: max_results.unwrap_or(100),
                })?,
            }),
            WorkspaceToolRequest::FileSymbols { path } => Ok(WorkspaceToolResponse::Symbols {
                symbols: self.file_symbols(&path)?,
            }),
            WorkspaceToolRequest::GitStatus => {
                Ok(WorkspaceToolResponse::Git(self.git_status()?))
            }
            WorkspaceToolRequest::GitDiff { staged, path } => Ok(WorkspaceToolResponse::Diff(
                self.git_diff(staged.unwrap_or(false), path.as_deref())?,
            )),
            WorkspaceToolRequest::GitWorktreeList => Ok(WorkspaceToolResponse::Worktrees {
                worktrees: self.git_worktree_list()?,
            }),
            WorkspaceToolRequest::GetDiagnostics { path } => {
                Ok(WorkspaceToolResponse::Diagnostics {
                    diagnostics: self.get_diagnostics(path.as_deref())?,
                })
            }
            WorkspaceToolRequest::ApplyTextEdits {
                path,
                edits,
                expected_version,
            } => {
                let document = self.read_file(&path, 8 * 1024 * 1024)?;
                if let Some(expected) = expected_version {
                    if expected != document.version {
                        return Err(WorkspaceError::VersionConflict {
                            path,
                            expected,
                            actual: document.version,
                        });
                    }
                }
                let new_content = crate::lsp::apply_text_edits(&document.content, &edits)?;
                Ok(WorkspaceToolResponse::Patched(
                    self.write_file(&document.path, &new_content)?,
                ))
            }
            WorkspaceToolRequest::GitBlame {
                path,
                start_line,
                end_line,
            } => Ok(WorkspaceToolResponse::Blame {
                lines: self.git_blame(&path, start_line, end_line)?,
            }),
            WorkspaceToolRequest::GitLog { max, path } => Ok(WorkspaceToolResponse::Log {
                commits: self.git_log(max.unwrap_or(50), path.as_deref())?,
            }),
        }
    }

    fn ensure_version(&self, path: &str, expected: Option<u64>) -> Result<()> {
        if let Some(expected) = expected {
            let actual = self.document_version(path)?;
            if expected != actual {
                return Err(WorkspaceError::VersionConflict {
                    path: path.to_owned(),
                    expected,
                    actual,
                });
            }
        }
        Ok(())
    }
}

fn apply_edits(content: &str, edits: &[PatchEdit]) -> Result<String> {
    let mut result = content.to_owned();
    for edit in edits {
        if edit.find.is_empty() {
            return Err(WorkspaceError::Patch("empty find string".into()));
        }
        let count = result.matches(&edit.find).count();
        if let Some(expected) = edit.expected_matches {
            if count as u32 != expected {
                return Err(WorkspaceError::Patch(format!(
                    "expected {} matches for {:?}, found {}",
                    expected, edit.find, count
                )));
            }
        }
        if count == 0 {
            return Err(WorkspaceError::Patch(format!(
                "pattern not found: {:?}",
                edit.find
            )));
        }
        result = result.replace(&edit.find, &edit.replace);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_edits_respects_expected_matches() {
        let content = "aaa bbb aaa";
        let edits = vec![PatchEdit {
            find: "aaa".into(),
            replace: "ccc".into(),
            expected_matches: Some(2),
        }];
        let out = apply_edits(content, &edits).unwrap();
        assert_eq!(out, "ccc bbb ccc");
    }
}
