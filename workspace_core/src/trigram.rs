//! Trigram inverted index for faster substring / fuzzy-ish search.
//!
//! Built alongside the text index. Query is decomposed into trigrams; candidate
//! files must contain all query trigrams (AND), then exact line scan confirms.

use std::collections::{HashMap, HashSet};

#[derive(Debug, Default, Clone)]
pub struct TrigramIndex {
    /// trigram → set of file keys
    postings: HashMap<[u8; 3], HashSet<u32>>,
    /// file id → path
    paths: Vec<String>,
    /// path → file id
    path_ids: HashMap<String, u32>,
}

impl TrigramIndex {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn clear(&mut self) {
        self.postings.clear();
        self.paths.clear();
        self.path_ids.clear();
    }

    pub fn remove(&mut self, path: &str) {
        let key = path.replace('\\', "/");
        let Some(&id) = self.path_ids.get(&key) else {
            return;
        };
        self.path_ids.remove(&key);
        // Mark path slot empty (don't compact — ids stay stable for session)
        if (id as usize) < self.paths.len() {
            self.paths[id as usize].clear();
        }
        // Leave postings stale; query filters empty paths. Periodic rebuild cleans.
    }

    pub fn insert(&mut self, path: &str, text: &str) {
        let key = path.replace('\\', "/");
        self.remove(&key);
        let id = self.paths.len() as u32;
        self.paths.push(key.clone());
        self.path_ids.insert(key, id);

        let lower = text.to_lowercase();
        let bytes = lower.as_bytes();
        if bytes.len() < 3 {
            return;
        }
        let mut seen = HashSet::new();
        for i in 0..bytes.len() - 2 {
            let t = [bytes[i], bytes[i + 1], bytes[i + 2]];
            if seen.insert(t) {
                self.postings.entry(t).or_default().insert(id);
            }
        }
    }

    /// Candidate paths that contain every trigram of the query (lowercased).
    pub fn candidates(&self, query: &str) -> Vec<String> {
        let q = query.to_lowercase();
        let bytes = q.as_bytes();
        if bytes.len() < 3 {
            // Too short for trigram filter — return all non-empty paths
            return self
                .paths
                .iter()
                .filter(|p| !p.is_empty())
                .cloned()
                .collect();
        }

        let mut trigrams = Vec::new();
        let mut seen = HashSet::new();
        for i in 0..bytes.len() - 2 {
            let t = [bytes[i], bytes[i + 1], bytes[i + 2]];
            if seen.insert(t) {
                trigrams.push(t);
            }
        }

        // Start from rarest trigram
        let mut sets: Vec<&HashSet<u32>> = Vec::new();
        for t in &trigrams {
            match self.postings.get(t) {
                Some(s) => sets.push(s),
                None => return Vec::new(), // trigram missing → no matches
            }
        }
        sets.sort_by_key(|s| s.len());

        let mut result: HashSet<u32> = sets[0].clone();
        for s in &sets[1..] {
            result.retain(|id| s.contains(id));
            if result.is_empty() {
                break;
            }
        }

        result
            .into_iter()
            .filter_map(|id| {
                self.paths
                    .get(id as usize)
                    .filter(|p| !p.is_empty())
                    .cloned()
            })
            .collect()
    }

    pub fn file_count(&self) -> usize {
        self.path_ids.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trigram_filters_candidates() {
        let mut idx = TrigramIndex::new();
        idx.insert("a.rs", "fn hello_world() {}");
        idx.insert("b.rs", "fn goodbye() {}");
        let c = idx.candidates("hello");
        assert!(c.iter().any(|p| p == "a.rs"));
        assert!(!c.iter().any(|p| p == "b.rs"));
    }
}
