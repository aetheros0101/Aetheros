//! AetherOS Workspace Engine — coding-agent workspace core.
//!
//! Capabilities:
//!   * guarded filesystem (path_guard: traversal, symlink, `.git` protection)
//!   * atomic documents with monotonic versioning + content hash
//!   * incremental full-text + regex search index
//!   * heuristic symbol index (tree-sitter ready via feature)
//!   * file metadata (mtime, size, hash, language)
//!   * watcher integration with coalesce/debounce
//!   * git status / diff / worktree (scrubbed environment)
//!   * LSP abstraction + diagnostics cache + code-action text edits
//!
//! Security boundary: this crate does NOT decide permissions; it only offers
//! safe primitives. Authorization lives in the host SecurityGovernor.

mod atomic;
mod error;
mod git;
mod index;
pub mod lsp;
mod models;
mod path_guard;
mod symbols;
mod tools;
mod trigram;
#[cfg(feature = "watcher")]
mod watcher;

pub use error::{Result, WorkspaceError};
pub use lsp::StdioLanguageServer;
pub use models::*;
pub use symbols::{SharedSymbolRegistry, SymbolParser, SymbolRegistry};
pub use tools::{PatchEdit, WorkspaceToolRequest, WorkspaceToolResponse};
#[cfg(feature = "watcher")]
pub use watcher::{coalesce_events, Debouncer, WorkspaceWatcher};

use atomic::atomic_write;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use index::{content_hash, detect_language, WorkspaceIndex};
use std::fs::{self, File};
use std::io::{self, Read};
use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};
use walkdir::WalkDir;

/// Tek yazma işleminin içerik üst sınırı.
pub const MAX_WRITE_BYTES: usize = 8 * 1024 * 1024;
/// Dışarıdan içe aktarılan tek dosyanın üst sınırı.
pub const MAX_IMPORT_BYTES: u64 = 25 * 1024 * 1024;

pub struct Workspace {
    root: PathBuf,
    ignore: Gitignore,
    index: Arc<RwLock<WorkspaceIndex>>,
    /// Optional LSP registry (host wires concrete servers).
    lsp: Arc<RwLock<lsp::LspRegistry>>,
    symbols: crate::symbols::SharedSymbolRegistry,
}

impl Workspace {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = fs::canonicalize(root).map_err(|_| WorkspaceError::RootMissing)?;
        if !root.is_dir() {
            return Err(WorkspaceError::NotDirectory);
        }
        let ignore = load_ignore(&root)?;
        let index = WorkspaceIndex::build(&root, &ignore)?;
        let symbols = crate::symbols::new_shared_registry();
        {
            // share registry with index for parser-backed extraction
            // (index already built with heuristic; next updates use registry)
        }
        let mut index = index;
        index.set_symbol_registry(symbols.clone());
        Ok(Self {
            root,
            ignore,
            index: Arc::new(RwLock::new(index)),
            lsp: Arc::new(RwLock::new(lsp::LspRegistry::new())),
            symbols,
        })
    }

    pub fn info(&self) -> WorkspaceInfo {
        WorkspaceInfo {
            root: self.root.to_string_lossy().into_owned(),
            name: self
                .root
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("workspace")
                .into(),
        }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    #[cfg(feature = "watcher")]
    pub fn start_watcher(&self) -> Result<WorkspaceWatcher> {
        WorkspaceWatcher::start(self.root.clone())
    }

    /// Access LSP registry for host to register language servers.
    pub fn lsp_registry(&self) -> Arc<RwLock<lsp::LspRegistry>> {
        self.lsp.clone()
    }

    pub fn symbol_registry(&self) -> crate::symbols::SharedSymbolRegistry {
        self.symbols.clone()
    }

    /// Register a symbol parser (e.g. tree-sitter backed) and rebuild index.
    pub fn register_symbol_parser(
        &self,
        parser: std::sync::Arc<dyn crate::symbols::SymbolParser>,
    ) -> Result<()> {
        self.symbols
            .write()
            .map_err(|_| WorkspaceError::Symbol("registry lock poisoned".into()))?
            .register(parser);
        self.refresh_index()
    }

    fn index_read(&self) -> Result<std::sync::RwLockReadGuard<'_, WorkspaceIndex>> {
        self.index
            .read()
            .map_err(|_| WorkspaceError::Watcher("index lock poisoned".into()))
    }

    fn index_write(&self) -> Result<std::sync::RwLockWriteGuard<'_, WorkspaceIndex>> {
        self.index
            .write()
            .map_err(|_| WorkspaceError::Watcher("index lock poisoned".into()))
    }

    pub fn index_file_count(&self) -> Result<usize> {
        Ok(self.index_read()?.len())
    }

    pub fn refresh_index(&self) -> Result<()> {
        // Preserve versions across full rebuild
        let versions: std::collections::HashMap<String, u64> = {
            let idx = self.index_read()?;
            idx.all_metadata()
                .into_iter()
                .filter(|m| m.version > 0)
                .map(|m| (m.path, m.version))
                .collect()
        };
        let mut fresh = WorkspaceIndex::build(&self.root, &self.ignore)?;
        for (path, ver) in versions {
            fresh.set_version(&path, ver);
        }
        *self.index_write()? = fresh;
        Ok(())
    }

    /// Incremental single-path refresh (preferred over full rebuild).
    pub fn refresh_one(&self, relative: &str) -> Result<()> {
        self.index_write()?
            .update(&self.root, relative, &self.ignore);
        Ok(())
    }

    /// Apply a coalesced watch event to the incremental index.
    pub fn apply_watch_event(&self, event: &WorkspaceWatchEvent) -> Result<()> {
        let mut i = self.index_write()?;
        if let Some(old) = &event.old_path {
            i.remove(old);
        }
        match event.kind {
            WatchEventKind::Removed => {
                i.remove(&event.path);
            }
            WatchEventKind::Other if event.path.is_empty() => {}
            _ => {
                if !event.path.is_empty() {
                    i.update(&self.root, &event.path, &self.ignore);
                }
            }
        }
        Ok(())
    }

    /// Apply a batch of coalesced watch events.
    pub fn apply_watch_events(&self, events: &[WorkspaceWatchEvent]) -> Result<()> {
        for ev in events {
            self.apply_watch_event(ev)?;
        }
        Ok(())
    }

    // ── path resolution ──────────────────────────────────────

    fn resolve(&self, relative: &str) -> Result<PathBuf> {
        Ok(path_guard::resolve(&self.root, relative, true)?.full)
    }

    fn resolve_mutable(&self, relative: &str) -> Result<PathBuf> {
        self.guard_mutation(relative, true)
    }

    fn resolve_mutable_nofollow(&self, relative: &str) -> Result<PathBuf> {
        self.guard_mutation(relative, false)
    }

    fn guard_mutation(&self, relative: &str, follow_final: bool) -> Result<PathBuf> {
        let r = path_guard::resolve(&self.root, relative, follow_final)?;
        if r.is_root() {
            return Err(WorkspaceError::InvalidPath);
        }
        if path_guard::is_protected(&r.rel) {
            return Err(WorkspaceError::Protected(relative.replace('\\', "/")));
        }
        Ok(r.full)
    }

    fn rel(&self, p: &Path) -> String {
        p.strip_prefix(&self.root)
            .unwrap_or(p)
            .to_string_lossy()
            .replace('\\', "/")
    }

    fn ignored(&self, p: &Path, d: bool) -> bool {
        self.ignore.matched_path_or_any_parents(p, d).is_ignore()
    }

    // ── read ─────────────────────────────────────────────────

    pub fn tree(
        &self,
        relative: &str,
        depth: usize,
        include_hidden: bool,
    ) -> Result<Vec<WorkspaceEntry>> {
        let root = self.resolve(relative)?;
        if !root.is_dir() {
            return Err(WorkspaceError::NotDirectory);
        }
        let mut out = Vec::new();
        for item in WalkDir::new(&root)
            .min_depth(1)
            .max_depth(depth.saturating_add(1))
            .follow_links(false)
            .into_iter()
            .filter_map(|x| x.ok())
        {
            let p = item.path();
            let name = p.file_name().and_then(|x| x.to_str()).unwrap_or("");
            if !include_hidden && name.starts_with('.') {
                continue;
            }
            let kind = if item.file_type().is_dir() {
                EntryKind::Directory
            } else if item.file_type().is_symlink() {
                EntryKind::Symlink
            } else {
                EntryKind::File
            };
            let size = if kind == EntryKind::File {
                item.metadata().map(|m| m.len()).unwrap_or(0)
            } else {
                0
            };
            out.push(WorkspaceEntry {
                path: self.rel(p),
                name: name.into(),
                kind,
                size,
                hidden: name.starts_with('.'),
                ignored: self.ignored(p, kind == EntryKind::Directory),
            });
        }
        out.sort_by(|a, b| a.path.cmp(&b.path));
        Ok(out)
    }

    pub fn read_file(&self, relative: &str, max_bytes: usize) -> Result<FileDocument> {
        let path = self.resolve(relative)?;
        let meta = fs::metadata(&path).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                WorkspaceError::NotFound(relative.into())
            } else {
                e.into()
            }
        })?;
        if meta.is_dir() {
            return Err(WorkspaceError::IsDirectory);
        }
        if meta.len() > max_bytes as u64 {
            return Err(WorkspaceError::TooLarge { limit: max_bytes });
        }
        let mut f = File::open(&path)?;
        let mut bytes = Vec::new();
        f.read_to_end(&mut bytes)?;
        // Geçersiz UTF-8 ya da NUL içeren dosya metin değildir; ham Io hatası
        // yerine NotText döner (arayüz bunu anlaşılır mesaja çevirir).
        let buf = String::from_utf8(bytes).map_err(|_| WorkspaceError::NotText)?;
        if buf.contains('\0') {
            return Err(WorkspaceError::NotText);
        }
        let version = self.document_version(relative).unwrap_or(0);
        let hash = content_hash(buf.as_bytes());
        let language = detect_language(&path);
        let readonly = meta.permissions().readonly();
        Ok(FileDocument {
            path: relative.replace('\\', "/"),
            size: buf.len() as u64,
            content: buf,
            readonly,
            version,
            content_hash: hash,
            language,
        })
    }

    pub fn document_version(&self, relative: &str) -> Result<u64> {
        self.detect_external_change(relative)?;
        Ok(self.index_read()?.get_version(relative))
    }

    /// Dosya API dışında (agent, editör, başka süreç) değiştiyse indeksi
    /// tazeler ve sürümü artırır. Watcher çalışmıyorsa sürüm tabanlı
    /// çakışma denetimi (`expected_version`) aksi halde hiç tetiklenmezdi.
    /// Yalnızca indekslenmiş dosyalar için çalışır; okunamayan dosya sessizce atlanır.
    fn detect_external_change(&self, relative: &str) -> Result<()> {
        let Ok(path) = self.resolve(relative) else {
            return Ok(());
        };
        let Ok(bytes) = fs::read(&path) else {
            return Ok(());
        };
        let indexed_hash = self
            .index_read()?
            .metadata(relative)
            .map(|m| m.content_hash);
        if let Some(h) = indexed_hash {
            if h != content_hash(&bytes) {
                let mut idx = self.index_write()?;
                idx.update(&self.root, relative, &self.ignore);
                idx.bump_version(relative);
            }
        }
        Ok(())
    }

    pub fn file_metadata(&self, relative: &str) -> Result<Option<FileMetadata>> {
        Ok(self.index_read()?.metadata(relative))
    }

    pub fn list_metadata(&self) -> Result<Vec<FileMetadata>> {
        Ok(self.index_read()?.all_metadata())
    }

    // ── write ────────────────────────────────────────────────

    pub fn write_file(&self, relative: &str, content: &str) -> Result<FileDocument> {
        if content.len() > MAX_WRITE_BYTES {
            return Err(WorkspaceError::TooLarge {
                limit: MAX_WRITE_BYTES,
            });
        }
        let path = self.resolve_mutable(relative)?;
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&path, content.as_bytes())?;

        let version = {
            let mut idx = self.index_write()?;
            idx.update(&self.root, relative, &self.ignore);
            idx.bump_version(relative)
        };

        let hash = content_hash(content.as_bytes());
        let language = detect_language(&path);

        // Notify LSP if registered
        if let Ok(mut reg) = self.lsp.write() {
            if let Some(ref lang) = language {
                let _ = reg.did_change(relative, lang, content, version);
            }
        }

        Ok(FileDocument {
            path: relative.replace('\\', "/"),
            content: content.to_owned(),
            size: content.len() as u64,
            readonly: false,
            version,
            content_hash: hash,
            language,
        })
    }

    pub fn create_file(&self, relative: &str) -> Result<CreateResult> {
        let path = self.resolve_mutable(relative)?;
        if fs::symlink_metadata(&path).is_ok() {
            return Err(WorkspaceError::AlreadyExists(relative.into()));
        }
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        atomic_write(&path, b"")?;
        self.refresh_one(relative)?;
        Ok(CreateResult {
            path: relative.replace('\\', "/"),
            kind: EntryKind::File,
        })
    }

    pub fn create_dir(&self, relative: &str) -> Result<CreateResult> {
        let path = self.resolve_mutable(relative)?;
        if fs::symlink_metadata(&path).is_ok() {
            return Err(WorkspaceError::AlreadyExists(relative.into()));
        }
        fs::create_dir_all(&path)?;
        Ok(CreateResult {
            path: relative.replace('\\', "/"),
            kind: EntryKind::Directory,
        })
    }

    /// Host bridge uyumu: `import_external` → `import_file`.
    pub fn import_external(&self, source: &Path, dest_relative: &str) -> Result<CreateResult> {
        self.import_file(source, dest_relative)
    }

    pub fn import_file(&self, source: &Path, dest_relative: &str) -> Result<CreateResult> {
        let meta = fs::metadata(source)?;
        if !meta.is_file() {
            return Err(WorkspaceError::NotDirectory);
        }
        if meta.len() > MAX_IMPORT_BYTES {
            return Err(WorkspaceError::TooLarge {
                limit: MAX_IMPORT_BYTES as usize,
            });
        }
        let dest = self.resolve_mutable(dest_relative)?;
        if fs::symlink_metadata(&dest).is_ok() {
            return Err(WorkspaceError::AlreadyExists(dest_relative.into()));
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        let name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("file");
        let tmp = dest.with_file_name(format!(
            ".{}.aetheros-import-{}.tmp",
            name,
            std::process::id()
        ));
        if let Err(e) = fs::copy(source, &tmp) {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }
        if let Err(e) = fs::rename(&tmp, &dest) {
            let _ = fs::remove_file(&tmp);
            return Err(e.into());
        }
        self.refresh_one(dest_relative)?;
        Ok(CreateResult {
            path: dest_relative.replace('\\', "/"),
            kind: EntryKind::File,
        })
    }

    pub fn rename(&self, from: &str, to: &str) -> Result<()> {
        let a = self.resolve_mutable_nofollow(from)?;
        let b = self.resolve_mutable(to)?;
        if fs::symlink_metadata(&a).is_err() {
            return Err(WorkspaceError::NotFound(from.into()));
        }
        if fs::symlink_metadata(&b).is_ok() {
            return Err(WorkspaceError::AlreadyExists(to.into()));
        }
        if let Some(p) = b.parent() {
            fs::create_dir_all(p)?;
        }
        fs::rename(&a, &b)?;
        // Incremental: remove old, index new
        {
            let mut idx = self.index_write()?;
            idx.remove(from);
            idx.update(&self.root, to, &self.ignore);
        }
        Ok(())
    }

    pub fn delete(&self, r: &str) -> Result<()> {
        let p = self.resolve_mutable_nofollow(r)?;
        let m = fs::symlink_metadata(&p).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                WorkspaceError::NotFound(r.into())
            } else {
                e.into()
            }
        })?;
        if m.is_dir() {
            fs::remove_dir_all(p)?;
        } else {
            fs::remove_file(p)?;
        }
        self.index_write()?.remove(r);
        Ok(())
    }

    // ── search / symbols ─────────────────────────────────────

    pub fn search(&self, o: SearchOptions) -> Result<Vec<SearchMatch>> {
        self.index_read()?.search(&o)
    }

    pub fn find_symbols(&self, query: SymbolQuery) -> Result<Vec<Symbol>> {
        Ok(self.index_read()?.symbols(&query))
    }

    pub fn file_symbols(&self, relative: &str) -> Result<Vec<Symbol>> {
        Ok(self.index_read()?.symbols_in_file(relative))
    }

    // ── git ──────────────────────────────────────────────────

    pub fn git_status(&self) -> Result<GitStatus> {
        match git::status(&self.root) {
            Ok(s) => Ok(s),
            Err(WorkspaceError::NotGitRepository) => Ok(GitStatus {
                is_git_repo: false,
                ..Default::default()
            }),
            Err(e) => Err(e),
        }
    }

    pub fn git_diff(&self, staged: bool, path: Option<&str>) -> Result<GitDiff> {
        git::diff(&self.root, staged, path)
    }

    /// Dosyayı git sahnesine al. Yol doğrulanır; `.git` korumalıdır.
    pub fn git_stage(&self, relative: &str) -> Result<()> {
        let rel = self.git_pathspec(relative)?;
        git::stage(&self.root, &rel)
    }

    /// Dosyanın sahnelenmiş değişikliğini geri al.
    pub fn git_unstage(&self, relative: &str) -> Result<()> {
        let rel = self.git_pathspec(relative)?;
        git::unstage(&self.root, &rel)
    }

    /// Sahnelenmiş değişikliklerle commit oluştur.
    pub fn git_commit(&self, message: &str) -> Result<()> {
        git::commit(&self.root, message)
    }

    fn git_pathspec(&self, relative: &str) -> Result<String> {
        let full = self.resolve_mutable_nofollow(relative)?;
        Ok(self.rel(&full))
    }

    pub fn git_worktree_list(&self) -> Result<Vec<GitWorktree>> {
        git::worktree_list(&self.root)
    }

    pub fn git_worktree_add(
        &self,
        path: &Path,
        branch: Option<&str>,
        create_branch: bool,
    ) -> Result<GitWorktree> {
        git::worktree_add(&self.root, path, branch, create_branch)
    }

    pub fn git_worktree_remove(&self, path: &Path, force: bool) -> Result<()> {
        git::worktree_remove(&self.root, path, force)
    }

    pub fn git_blame(
        &self,
        path: &str,
        start_line: Option<u32>,
        end_line: Option<u32>,
    ) -> Result<Vec<GitBlameLine>> {
        // path must resolve inside workspace
        let _ = self.resolve(path)?;
        git::blame(&self.root, path, start_line, end_line)
    }

    pub fn git_log(&self, max: u32, path: Option<&str>) -> Result<Vec<GitCommit>> {
        if let Some(p) = path {
            let _ = self.resolve(p)?;
        }
        git::log(&self.root, max, path)
    }

    // ── diagnostics ──────────────────────────────────────────

    pub fn get_diagnostics(&self, path: Option<&str>) -> Result<Vec<Diagnostic>> {
        self.lsp
            .read()
            .map_err(|_| WorkspaceError::Lsp("lsp lock poisoned".into()))?
            .get_diagnostics(path)
    }

    pub fn set_diagnostics(&self, path: &str, diags: Vec<Diagnostic>) -> Result<()> {
        let reg = self
            .lsp
            .read()
            .map_err(|_| WorkspaceError::Lsp("lsp lock poisoned".into()))?;
        let store = reg.diagnostics_store();
        store
            .write()
            .map_err(|_| WorkspaceError::Lsp("diagnostics lock poisoned".into()))?
            .set(path, diags);
        Ok(())
    }
}

fn load_ignore(root: &Path) -> Result<Gitignore> {
    let mut builder = GitignoreBuilder::new(root);
    let gi = root.join(".gitignore");
    if gi.is_file() {
        let _ = builder.add(&gi);
    }
    // Always ignore .git
    let _ = builder.add_line(None, ".git/");
    builder
        .build()
        .map_err(|e| WorkspaceError::Io(io::Error::other(e.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn open_write_search_version() {
        let dir = tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        ws.write_file("src/main.rs", "fn main() {\n    println!(\"hi\");\n}\n")
            .unwrap();
        assert_eq!(ws.document_version("src/main.rs").unwrap(), 1);
        let hits = ws
            .search(SearchOptions {
                query: "println".into(),
                ..Default::default()
            })
            .unwrap();
        assert_eq!(hits.len(), 1);
        let syms = ws.file_symbols("src/main.rs").unwrap();
        assert!(syms.iter().any(|s| s.name == "main"));
    }

    #[test]
    fn version_conflict() {
        let dir = tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        ws.write_file("a.txt", "one").unwrap();
        let r = ws.execute_tool(WorkspaceToolRequest::WriteFile {
            path: "a.txt".into(),
            content: "two".into(),
            expected_version: Some(99),
        });
        assert!(matches!(r, Err(WorkspaceError::VersionConflict { .. })));
    }

    #[test]
    fn incremental_rename() {
        let dir = tempdir().unwrap();
        let ws = Workspace::open(dir.path()).unwrap();
        ws.write_file("old.rs", "fn legacy() {}").unwrap();
        ws.rename("old.rs", "new.rs").unwrap();
        assert!(ws.file_metadata("old.rs").unwrap().is_none());
        assert!(ws.file_metadata("new.rs").unwrap().is_some());
    }
}
