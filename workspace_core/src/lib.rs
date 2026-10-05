//! AetherOS Workspace: korumalı dosya sistemi, atomik belgeler, arama indeksi ve Git durumu.
//!
//! Güvenlik sınırı (bu crate karar VERMEZ, yalnız güvenli ilkeller sunar):
//!   * her yol `path_guard` üzerinden geçer (traversal, symlink kaçışı, ara
//!     klasör atlatması)
//!   * kökün kendisi değiştirilemez; `.git` altı değiştirilemez (hook/config
//!     yazarak komut çalıştırma zinciri kapatılır)
//!   * yazma boyutu sınırlı; indeks dosya/toplam boyutla sınırlı
//! İzin/onay kararı uygulamanın SecurityGovernor hattındadır.

mod atomic;
mod error;
mod git;
mod index;
mod models;
mod path_guard;
mod tools;
#[cfg(feature = "watcher")]
mod watcher;

pub use error::{Result, WorkspaceError};
pub use models::*;
pub use tools::{PatchEdit, WorkspaceToolRequest, WorkspaceToolResponse};
#[cfg(feature = "watcher")]
pub use watcher::WorkspaceWatcher;

use atomic::atomic_write;
use ignore::gitignore::{Gitignore, GitignoreBuilder};
use index::WorkspaceIndex;
use std::{
    fs::{self, File},
    io::{self, Read},
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
};
use walkdir::WalkDir;

/// Tek yazma işleminin içerik üst sınırı.
pub const MAX_WRITE_BYTES: usize = 8 * 1024 * 1024;

pub struct Workspace {
    root: PathBuf,
    ignore: Gitignore,
    index: Arc<RwLock<WorkspaceIndex>>,
}

impl Workspace {
    pub fn open(root: impl AsRef<Path>) -> Result<Self> {
        let root = fs::canonicalize(root).map_err(|_| WorkspaceError::RootMissing)?;
        if !root.is_dir() {
            return Err(WorkspaceError::NotDirectory);
        }
        let ignore = load_ignore(&root)?;
        let index = WorkspaceIndex::build(&root, &ignore)?;
        Ok(Self { root, ignore, index: Arc::new(RwLock::new(index)) })
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

    #[cfg(feature = "watcher")]
    pub fn start_watcher(&self) -> Result<WorkspaceWatcher> {
        WorkspaceWatcher::start(self.root.clone())
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
        let fresh = WorkspaceIndex::build(&self.root, &self.ignore)?;
        *self.index_write()? = fresh;
        Ok(())
    }

    pub fn apply_watch_event(&self, event: &WorkspaceWatchEvent) -> Result<()> {
        let mut i = self.index_write()?;
        if let Some(old) = &event.old_path {
            i.remove(old);
        }
        if !matches!(event.kind, WatchEventKind::Removed) {
            i.update(&self.root, &event.path, &self.ignore);
        } else {
            i.remove(&event.path);
        }
        Ok(())
    }

    // ── yol çözümleme ────────────────────────────────────────

    /// Okuma/listeleme: kök dahil; son bileşen symlink ise hedef kökte kalmalı.
    fn resolve(&self, relative: &str) -> Result<PathBuf> {
        Ok(path_guard::resolve(&self.root, relative, true)?.full)
    }

    /// Değiştirme: kök ve `.git` altı reddedilir.
    fn resolve_mutable(&self, relative: &str) -> Result<PathBuf> {
        self.guard_mutation(relative, true)
    }

    /// Silme/yeniden adlandırma kaynağı: son bileşen İZLENMEZ (dışarıyı
    /// gösteren bir link, hedefe dokunmadan kaldırılabilir).
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
        p.strip_prefix(&self.root).unwrap_or(p).to_string_lossy().replace('\\', "/")
    }

    fn ignored(&self, p: &Path, d: bool) -> bool {
        self.ignore.matched_path_or_any_parents(p, d).is_ignore()
    }

    // ── okuma ────────────────────────────────────────────────

    pub fn tree(&self, relative: &str, depth: usize, include_hidden: bool) -> Result<Vec<WorkspaceEntry>> {
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
            if self.ignored(p, matches!(kind, EntryKind::Directory)) {
                continue;
            }
            out.push(WorkspaceEntry {
                path: self.rel(p),
                name: name.into(),
                kind,
                size: if matches!(kind, EntryKind::File) {
                    item.metadata().map(|m| m.len()).unwrap_or(0)
                } else {
                    0
                },
                hidden: name.starts_with('.'),
                ignored: false,
            });
        }
        out.sort_by_key(|e| (matches!(e.kind, EntryKind::File), e.path.clone()));
        Ok(out)
    }

    pub fn read_file(&self, relative: &str, max_bytes: usize) -> Result<FileDocument> {
        let p = self.resolve(relative)?;
        let meta = fs::metadata(&p).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                WorkspaceError::NotFound(relative.into())
            } else {
                e.into()
            }
        })?;
        if !meta.is_file() {
            return Err(WorkspaceError::IsDirectory);
        }
        let mut f = File::open(&p)?;
        let mut b = Vec::new();
        Read::by_ref(&mut f).take((max_bytes as u64).saturating_add(1)).read_to_end(&mut b)?;
        if b.len() > max_bytes {
            return Err(WorkspaceError::Io(io::Error::other("file exceeds read limit")));
        }
        let content = String::from_utf8(b).map_err(|_| WorkspaceError::NotText)?;
        Ok(FileDocument {
            path: relative.replace('\\', "/"),
            content,
            size: meta.len(),
            readonly: meta.permissions().readonly(),
            version: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0),
        })
    }

    // ── yazma ────────────────────────────────────────────────

    pub fn write_file(&self, relative: &str, content: &str) -> Result<FileDocument> {
        if content.len() > MAX_WRITE_BYTES {
            return Err(WorkspaceError::TooLarge { limit: MAX_WRITE_BYTES });
        }
        let p = self.resolve_mutable(relative)?;
        atomic_write(&p, content.as_bytes())?;
        self.refresh_one(relative)?;
        self.read_file(relative, content.len().saturating_add(1))
    }

    fn refresh_one(&self, relative: &str) -> Result<()> {
        let mut i = self.index_write()?;
        i.update(&self.root, relative, &self.ignore);
        Ok(())
    }

    pub fn create_file(&self, r: &str) -> Result<CreateResult> {
        let p = self.resolve_mutable(r)?;
        if fs::symlink_metadata(&p).is_ok() {
            return Err(WorkspaceError::AlreadyExists(r.into()));
        }
        atomic_write(&p, b"")?;
        self.refresh_one(r)?;
        Ok(CreateResult { path: r.replace('\\', "/"), kind: EntryKind::File })
    }

    pub fn create_dir(&self, r: &str) -> Result<CreateResult> {
        let p = self.resolve_mutable(r)?;
        if fs::symlink_metadata(&p).is_ok() {
            return Err(WorkspaceError::AlreadyExists(r.into()));
        }
        fs::create_dir_all(p)?;
        Ok(CreateResult { path: r.replace('\\', "/"), kind: EntryKind::Directory })
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
        fs::rename(a, b)?;
        self.refresh_index()?;
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
        self.refresh_index()?;
        Ok(())
    }

    pub fn search(&self, o: SearchOptions) -> Result<Vec<SearchMatch>> {
        let i = self.index_read()?;
        Ok(i.search(&o.query, o.case_sensitive, o.include_hidden, o.max_results as usize)
            .into_iter()
            .map(|(path, line, column, preview)| SearchMatch { path, line, column, preview })
            .collect())
    }

    pub fn git_status(&self) -> Result<GitStatus> {
        git::status(&self.root)
    }
}

fn load_ignore(root: &Path) -> Result<Gitignore> {
    let mut b = GitignoreBuilder::new(root);
    let p = root.join(".gitignore");
    if p.is_file() {
        let _ = b.add(p);
    }
    b.build().map_err(|e| WorkspaceError::Io(io::Error::other(e)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn traversal() {
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        assert!(matches!(w.read_file("../x", 10), Err(WorkspaceError::PathOutsideRoot)));
    }

    #[test]
    fn atomic_lifecycle() {
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        w.create_file("src/main.rs").unwrap();
        w.write_file("src/main.rs", "fn main(){}\n").unwrap();
        assert_eq!(w.read_file("src/main.rs", 100).unwrap().content, "fn main(){}\n");
        assert_eq!(
            w.search(SearchOptions { query: "main".into(), ..Default::default() }).unwrap().len(),
            1
        );
    }

    #[test]
    fn root_can_be_listed_but_never_modified() {
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        w.create_file("a.txt").unwrap();
        for p in ["", ".", "./"] {
            let list = w.tree(p, 1, false).unwrap();
            assert!(list.iter().any(|e| e.path == "a.txt"), "{p:?}");
            assert!(matches!(w.delete(p), Err(WorkspaceError::InvalidPath)), "{p:?}");
            assert!(matches!(w.rename(p, "x"), Err(WorkspaceError::InvalidPath)), "{p:?}");
            assert!(matches!(w.write_file(p, "x"), Err(WorkspaceError::InvalidPath)), "{p:?}");
        }
        assert!(d.path().join("a.txt").exists(), "kök içeriği silinmemeli");
    }

    #[test]
    fn dot_git_is_protected_for_every_mutation_but_readable() {
        let d = tempdir().unwrap();
        fs::create_dir_all(d.path().join(".git/hooks")).unwrap();
        fs::write(d.path().join(".git/config"), "[core]\n").unwrap();
        let w = Workspace::open(d.path()).unwrap();

        assert!(matches!(w.write_file(".git/config", "x"), Err(WorkspaceError::Protected(_))));
        assert!(matches!(w.write_file(".git/hooks/pre-commit", "#!/bin/sh"), Err(WorkspaceError::Protected(_))));
        assert!(matches!(w.create_file(".git/hooks/post-commit"), Err(WorkspaceError::Protected(_))));
        assert!(matches!(w.create_dir(".git/x"), Err(WorkspaceError::Protected(_))));
        assert!(matches!(w.delete(".git"), Err(WorkspaceError::Protected(_))));
        assert!(matches!(w.rename(".git/config", "c"), Err(WorkspaceError::Protected(_))));
        w.create_file("a").unwrap();
        assert!(matches!(w.rename("a", ".git/a"), Err(WorkspaceError::Protected(_))));
        assert_eq!(fs::read_to_string(d.path().join(".git/config")).unwrap(), "[core]\n");
        assert!(!d.path().join(".git/hooks/pre-commit").exists());
        // okuma serbest (karar Governor'da)
        assert!(w.read_file(".git/config", 100).is_ok());
    }

    #[test]
    fn oversized_write_is_rejected_and_nothing_is_written() {
        let d = tempdir().unwrap();
        let w = Workspace::open(d.path()).unwrap();
        let big = "a".repeat(MAX_WRITE_BYTES + 1);
        assert!(matches!(w.write_file("big.txt", &big), Err(WorkspaceError::TooLarge { .. })));
        assert!(!d.path().join("big.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn symlink_escape_is_blocked_end_to_end() {
        use std::os::unix::fs::symlink;
        let d = tempdir().unwrap();
        let out = tempdir().unwrap();
        fs::write(out.path().join("secret.txt"), "GIZLI").unwrap();
        let w = Workspace::open(d.path()).unwrap();
        symlink(out.path().join("secret.txt"), d.path().join("linkfile")).unwrap();
        symlink(out.path(), d.path().join("linkdir")).unwrap();

        assert!(matches!(w.read_file("linkfile", 100), Err(WorkspaceError::PathOutsideRoot)));
        assert!(matches!(w.write_file("linkdir/yeni/f.txt", "x"), Err(WorkspaceError::PathOutsideRoot)));
        assert!(matches!(w.create_dir("linkdir/yeni"), Err(WorkspaceError::PathOutsideRoot)));
        assert!(!out.path().join("yeni").exists(), "dışarıda hiçbir şey oluşmamalı");

        // dış link, hedefe dokunmadan kaldırılabilir
        w.delete("linkfile").unwrap();
        assert!(out.path().join("secret.txt").exists(), "hedef dosya silinmemeli");
        assert!(!d.path().join("linkfile").exists());
    }
}
