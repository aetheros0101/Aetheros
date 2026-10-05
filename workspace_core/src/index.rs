use crate::error::Result;
use ignore::gitignore::Gitignore;
use std::{collections::HashMap, fs, path::Path};
use walkdir::WalkDir;

/// Telefonda belleği korumak için: indekse alınan tek dosya üst sınırı...
pub const MAX_INDEXED_FILE_BYTES: u64 = 1024 * 1024;
/// ...ve indeksin toplam metin boyutu. Aşılınca kalan dosyalar atlanır
/// (okuma/yazma etkilenmez, yalnız arama kapsamı daralır).
pub const MAX_INDEXED_TOTAL_BYTES: usize = 64 * 1024 * 1024;

#[derive(Debug, Clone, Default)]
pub struct WorkspaceIndex {
    files: HashMap<String, IndexedFile>,
    total_bytes: usize,
}

#[derive(Debug, Clone)]
struct IndexedFile {
    text: String,
    hidden: bool,
}

fn read_text(path: &Path) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > MAX_INDEXED_FILE_BYTES {
        return None;
    }
    let bytes = fs::read(path).ok()?;
    if bytes.contains(&0) {
        return None;
    }
    String::from_utf8(bytes).ok()
}

fn is_hidden(path: &Path) -> bool {
    path.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('.'))
}

impl WorkspaceIndex {
    pub fn build(root: &Path, ignore: &Gitignore) -> Result<Self> {
        let mut index = Self::default();
        // `.git` klasörü hiç indekslenmez (yapılandırma/kimlik bilgisi içerebilir,
        // aramada görünmemeli).
        let walker = WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !(e.file_type().is_dir() && e.file_name() == ".git"));

        for entry in walker.filter_map(|e| e.ok()) {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            if ignore.matched_path_or_any_parents(path, false).is_ignore() {
                continue;
            }
            let Some(text) = read_text(path) else { continue };
            if index.total_bytes + text.len() > MAX_INDEXED_TOTAL_BYTES {
                continue;
            }
            let Ok(rel) = path.strip_prefix(root) else { continue };
            let rel = rel.to_string_lossy().replace('\\', "/");
            index.total_bytes += text.len();
            index.files.insert(rel, IndexedFile { text, hidden: is_hidden(path) });
        }
        Ok(index)
    }

    pub fn update(&mut self, root: &Path, relative: &str, ignore: &Gitignore) {
        let path = root.join(relative);
        let key = relative.replace('\\', "/");
        self.remove(&key);
        if key.split('/').any(|c| c.eq_ignore_ascii_case(".git")) {
            return;
        }
        if !path.is_file() || ignore.matched_path_or_any_parents(&path, false).is_ignore() {
            return;
        }
        let Some(text) = read_text(&path) else { return };
        if self.total_bytes + text.len() > MAX_INDEXED_TOTAL_BYTES {
            return;
        }
        self.total_bytes += text.len();
        self.files.insert(key, IndexedFile { text, hidden: is_hidden(&path) });
    }

    pub fn remove(&mut self, relative: &str) {
        if let Some(old) = self.files.remove(&relative.replace('\\', "/")) {
            self.total_bytes = self.total_bytes.saturating_sub(old.text.len());
        }
    }

    pub fn search(&self, query: &str, case_sensitive: bool, include_hidden: bool, max: usize) -> Vec<(String, u32, u32, String)> {
        if query.is_empty() || max == 0 {
            return Vec::new();
        }
        let needle = if case_sensitive { query.to_owned() } else { query.to_lowercase() };
        let mut out = Vec::with_capacity(max.min(64));

        for (path, file) in &self.files {
            if !include_hidden && file.hidden {
                continue;
            }
            for (line_no, line) in file.text.lines().enumerate() {
                let hay = if case_sensitive { None } else { Some(line.to_lowercase()) };
                let searchable = hay.as_deref().unwrap_or(line);
                if let Some(pos) = searchable.find(&needle) {
                    let column = if case_sensitive {
                        pos
                    } else {
                        line.char_indices().take_while(|(i, _)| *i < pos).count()
                    };
                    out.push((
                        path.clone(),
                        line_no as u32 + 1,
                        column as u32 + 1,
                        line.trim().chars().take(240).collect(),
                    ));
                    if out.len() >= max {
                        return out;
                    }
                }
            }
        }
        out
    }

    pub fn len(&self) -> usize {
        self.files.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
    use tempfile::tempdir;

    fn no_ignore(root: &Path) -> Gitignore {
        GitignoreBuilder::new(root).build().unwrap()
    }

    #[test]
    fn dot_git_and_oversized_files_are_not_indexed() {
        let d = tempdir().unwrap();
        fs::create_dir_all(d.path().join(".git")).unwrap();
        fs::write(d.path().join(".git/config"), "url = https://user:TOKEN@x").unwrap();
        fs::write(d.path().join("ok.txt"), "merhaba").unwrap();
        fs::write(d.path().join("big.txt"), "a".repeat(MAX_INDEXED_FILE_BYTES as usize + 1)).unwrap();
        let idx = WorkspaceIndex::build(d.path(), &no_ignore(d.path())).unwrap();
        assert_eq!(idx.len(), 1);
        assert!(idx.search("TOKEN", false, true, 10).is_empty());
        assert_eq!(idx.search("merhaba", false, false, 10).len(), 1);
    }

    #[test]
    fn update_and_remove_keep_the_byte_budget_consistent() {
        let d = tempdir().unwrap();
        let ig = no_ignore(d.path());
        let mut idx = WorkspaceIndex::build(d.path(), &ig).unwrap();
        fs::write(d.path().join("a.txt"), "12345").unwrap();
        idx.update(d.path(), "a.txt", &ig);
        assert_eq!(idx.total_bytes, 5);
        fs::write(d.path().join("a.txt"), "12").unwrap();
        idx.update(d.path(), "a.txt", &ig);
        assert_eq!(idx.total_bytes, 2);
        idx.remove("a.txt");
        assert_eq!(idx.total_bytes, 0);
        // .git yolu güncellemeyle de girmez
        fs::create_dir_all(d.path().join(".git")).unwrap();
        fs::write(d.path().join(".git/x"), "gizli").unwrap();
        idx.update(d.path(), ".git/x", &ig);
        assert_eq!(idx.len(), 0);
    }
}
