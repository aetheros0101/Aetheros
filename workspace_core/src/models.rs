use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceInfo { pub root: String, pub name: String }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EntryKind { File, Directory, Symlink }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEntry { pub path: String, pub name: String, pub kind: EntryKind, pub size: u64, pub hidden: bool, pub ignored: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDocument { pub path: String, pub content: String, pub size: u64, pub readonly: bool, pub version: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions { pub query: String, pub case_sensitive: bool, pub max_results: u32, pub include_hidden: bool }

impl Default for SearchOptions {
    fn default() -> Self { Self { query: String::new(), case_sensitive: false, max_results: 1000, include_hidden: false } }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchMatch { pub path: String, pub line: u32, pub column: u32, pub preview: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateResult { pub path: String, pub kind: EntryKind }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WatchEventKind { Created, Modified, Removed, Renamed, Other }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceWatchEvent { pub kind: WatchEventKind, pub path: String, pub old_path: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum GitStatusKind { Added, Modified, Deleted, Renamed, Untracked, Conflicted, Ignored, TypeChanged, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitStatusEntry { pub path: String, pub kind: GitStatusKind, pub staged: bool, pub worktree: bool }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GitStatus { pub branch: Option<String>, pub ahead: u32, pub behind: u32, pub entries: Vec<GitStatusEntry>, pub is_git_repo: bool }
