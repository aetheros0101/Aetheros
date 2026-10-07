//! Agent memory — working / episodic / summary katmanları + opsiyonel vektör portu.
//!
//! Host uyumu: `store(key, Value)` / `get(key)` davranışı korunur (working katmanı).

use chrono::{DateTime, Utc};
use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

/// Bellek katmanı.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum MemoryLayer {
    /// Adım çıktıları, geçici scratch.
    #[default]
    Working,
    /// Önemli olaylar / tool sonuç özetleri (daha uzun ömür).
    Episodic,
    /// Sıkıştırılmış bağlam (planner prompt'una girer).
    Summary,
}

impl MemoryLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryLayer::Working => "working",
            MemoryLayer::Episodic => "episodic",
            MemoryLayer::Summary => "summary",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.trim().to_ascii_lowercase().as_str() {
            "episodic" => MemoryLayer::Episodic,
            "summary" => MemoryLayer::Summary,
            _ => MemoryLayer::Working,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMemoryRecord {
    pub key: String,
    pub value: Value,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub layer: MemoryLayer,
    /// Karakter tahmini (compact için).
    #[serde(default)]
    pub chars: usize,
}

impl AgentMemoryRecord {
    pub fn new(key: impl Into<String>, value: Value, layer: MemoryLayer) -> Self {
        let key = key.into();
        let chars = value_chars(&value);
        Self {
            key,
            value,
            created_at: Utc::now(),
            tags: Vec::new(),
            layer,
            chars,
        }
    }
}

fn value_chars(v: &Value) -> usize {
    match v {
        Value::String(s) => s.chars().count(),
        other => other.to_string().chars().count(),
    }
}

/// Working tavanı aşıldığında otomatik compact tetiklenir.
const DEFAULT_WORKING_CHAR_BUDGET: usize = 48_000;
const DEFAULT_WORKING_ENTRY_CAP: usize = 80;

pub struct AgentMemory {
    /// key → kayıt (tüm katmanlar).
    entries: DashMap<String, AgentMemoryRecord>,
    working_char_budget: usize,
    working_entry_cap: usize,
    /// Opsiyonel semantik arama portu.
    vector: Option<Arc<dyn VectorMemoryPort>>,
}

impl AgentMemory {
    pub fn new() -> Self {
        Self {
            entries: DashMap::new(),
            working_char_budget: DEFAULT_WORKING_CHAR_BUDGET,
            working_entry_cap: DEFAULT_WORKING_ENTRY_CAP,
            vector: None,
        }
    }

    pub fn with_limits(mut self, char_budget: usize, entry_cap: usize) -> Self {
        self.working_char_budget = char_budget;
        self.working_entry_cap = entry_cap;
        self
    }

    pub fn with_vector(mut self, port: Arc<dyn VectorMemoryPort>) -> Self {
        self.vector = Some(port);
        self
    }

    // ── Host uyumlu API ──────────────────────────────────────

    /// Adım sonucunu working katmanına yazar (runtime).
    pub fn store(&self, key: impl Into<String>, value: Value) {
        self.store_in(key, value, MemoryLayer::Working, &[]);
    }

    pub fn put(&self, key: impl Into<String>, value: Value) {
        self.store(key, value);
    }

    pub fn get(&self, key: &str) -> Option<Value> {
        self.entries.get(key).map(|r| r.value.clone())
    }

    pub fn remove(&self, key: &str) -> bool {
        let removed = self.entries.remove(key).is_some();
        if removed {
            if let Some(v) = &self.vector {
                v.remove(key);
            }
        }
        removed
    }

    pub fn snapshot(&self) -> Vec<(String, Value)> {
        self.entries
            .iter()
            .map(|e| (e.key().clone(), e.value().value.clone()))
            .collect()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    // ── Katmanlı API ─────────────────────────────────────────

    pub fn store_in(
        &self,
        key: impl Into<String>,
        value: Value,
        layer: MemoryLayer,
        tags: &[&str],
    ) {
        let mut rec = AgentMemoryRecord::new(key, value, layer);
        rec.tags = tags.iter().map(|s| (*s).to_string()).collect();
        let key = rec.key.clone();
        self.entries.insert(key.clone(), rec.clone());

        if let Some(v) = &self.vector {
            v.upsert(&rec);
        }

        if layer == MemoryLayer::Working {
            self.maybe_compact_working();
        }
    }

    pub fn store_episodic(&self, key: impl Into<String>, value: Value) {
        self.store_in(key, value, MemoryLayer::Episodic, &["episodic"]);
    }

    pub fn get_record(&self, key: &str) -> Option<AgentMemoryRecord> {
        self.entries.get(key).map(|r| r.clone())
    }

    pub fn records_by_layer(&self, layer: MemoryLayer) -> Vec<AgentMemoryRecord> {
        let mut out: Vec<_> = self
            .entries
            .iter()
            .filter(|e| e.layer == layer)
            .map(|e| e.clone())
            .collect();
        out.sort_by_key(|r| r.created_at);
        out
    }

    pub fn clear_layer(&self, layer: MemoryLayer) {
        self.entries.retain(|_, r| r.layer != layer);
    }

    pub fn working_chars(&self) -> usize {
        self.records_by_layer(MemoryLayer::Working)
            .iter()
            .map(|r| r.chars)
            .sum()
    }

    /// Working dolunca en eski girdileri özetleyip Summary'ye taşır.
    pub fn maybe_compact_working(&self) {
        let working = self.records_by_layer(MemoryLayer::Working);
        let chars: usize = working.iter().map(|r| r.chars).sum();
        if working.len() <= self.working_entry_cap && chars <= self.working_char_budget {
            return;
        }
        let _ = self.compact_working();
    }

    /// Working → Summary sıkıştırma. Dönen metin planner context'ine eklenebilir.
    pub fn compact_working(&self) -> String {
        let working = self.records_by_layer(MemoryLayer::Working);
        if working.is_empty() {
            return String::new();
        }

        // En eski %60'ı özetle, yenileri bırak.
        let cut = (working.len() * 3 / 5).max(1).min(working.len());
        let (old, _keep) = working.split_at(cut);

        let mut lines: Vec<String> = Vec::with_capacity(old.len());
        for r in old {
            let preview = match &r.value {
                Value::String(s) => truncate_chars(s, 200),
                other => truncate_chars(&other.to_string(), 200),
            };
            lines.push(format!("- {}: {}", r.key, preview));
            self.remove(&r.key);
        }

        let summary_text = lines.join("\n");
        // Saniye çözünürlüklü anahtar aynı saniyedeki ikinci compact'te
        // önceki özeti sessizce eziyordu; mikrosaniye + çakışma kontrolü.
        let stamp = Utc::now().timestamp_micros();
        let mut key = format!("summary/{stamp}");
        let mut n = 1u32;
        while self.entries.contains_key(&key) {
            key = format!("summary/{stamp}-{n}");
            n += 1;
        }
        self.store_in(
            key,
            Value::String(summary_text.clone()),
            MemoryLayer::Summary,
            &["auto_compact"],
        );
        summary_text
    }

    /// Planner / AI için bağlam bloğu: summary + son working + episodic ipuçları.
    pub fn context_block(&self, max_chars: usize) -> String {
        let mut parts: Vec<String> = Vec::new();
        let mut used = 0usize;

        for r in self.records_by_layer(MemoryLayer::Summary).into_iter().rev() {
            let s = format!("[summary] {}", value_as_str(&r.value));
            if used + s.chars().count() > max_chars {
                break;
            }
            used += s.chars().count();
            parts.push(s);
        }

        for r in self
            .records_by_layer(MemoryLayer::Episodic)
            .into_iter()
            .rev()
            .take(10)
        {
            let s = format!("[episodic:{}] {}", r.key, truncate_chars(&value_as_str(&r.value), 160));
            if used + s.chars().count() > max_chars {
                break;
            }
            used += s.chars().count();
            parts.push(s);
        }

        for r in self
            .records_by_layer(MemoryLayer::Working)
            .into_iter()
            .rev()
            .take(20)
        {
            let s = format!("[working:{}] {}", r.key, truncate_chars(&value_as_str(&r.value), 160));
            if used + s.chars().count() > max_chars {
                break;
            }
            used += s.chars().count();
            parts.push(s);
        }

        parts.reverse();
        parts.join("\n")
    }

    /// Vektör portu varsa semantik arama.
    pub fn semantic_search(&self, query: &str, limit: usize) -> Vec<AgentMemoryRecord> {
        match &self.vector {
            Some(v) => v.search(query, limit),
            None => {
                // Fallback: key/value alt string eşleşmesi
                let q = query.to_ascii_lowercase();
                self.entries
                    .iter()
                    .filter(|e| {
                        e.key.to_ascii_lowercase().contains(&q)
                            || value_as_str(&e.value).to_ascii_lowercase().contains(&q)
                    })
                    .map(|e| e.clone())
                    .take(limit)
                    .collect()
            }
        }
    }
}

impl Default for AgentMemory {
    fn default() -> Self {
        Self::new()
    }
}

fn value_as_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn truncate_chars(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max).collect();
    out.push('…');
    out
}

// ── Vektör bellek portu (P2 opsiyonel) ────────────────────────

/// Host embedding servisi / yerel model bağlar.
pub trait VectorMemoryPort: Send + Sync {
    fn upsert(&self, record: &AgentMemoryRecord);
    fn search(&self, query: &str, limit: usize) -> Vec<AgentMemoryRecord>;
    fn remove(&self, key: &str);
}

/// Basit bellek-içi "vektör" — gerçek embedding yok; test / fallback.
#[derive(Default)]
pub struct InMemoryVectorPort {
    items: DashMap<String, AgentMemoryRecord>,
}

impl VectorMemoryPort for InMemoryVectorPort {
    fn upsert(&self, record: &AgentMemoryRecord) {
        self.items.insert(record.key.clone(), record.clone());
    }

    fn search(&self, query: &str, limit: usize) -> Vec<AgentMemoryRecord> {
        let q = query.to_ascii_lowercase();
        let mut scored: Vec<(usize, AgentMemoryRecord)> = self
            .items
            .iter()
            .filter_map(|e| {
                let text = format!("{} {}", e.key, value_as_str(&e.value)).to_ascii_lowercase();
                if !text.contains(&q) && !q.split_whitespace().any(|w| text.contains(w)) {
                    return None;
                }
                let score = q.split_whitespace().filter(|w| text.contains(*w)).count();
                Some((score, e.clone()))
            })
            .collect();
        scored.sort_by(|a, b| b.0.cmp(&a.0));
        scored.into_iter().take(limit).map(|(_, r)| r).collect()
    }

    fn remove(&self, key: &str) {
        self.items.remove(key);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn count(mem: &AgentMemory, layer: MemoryLayer) -> usize {
        mem.records_by_layer(layer).len()
    }

    #[test]
    fn store_get_remove_roundtrip() {
        let mem = AgentMemory::new();
        assert!(mem.is_empty());
        mem.store("k", json!("v"));
        assert_eq!(mem.len(), 1);
        assert_eq!(mem.get("k"), Some(json!("v")));
        assert!(mem.remove("k"));
        assert!(!mem.remove("k"));
        assert_eq!(mem.get("k"), None);
    }

    #[test]
    fn store_overwrites_same_key() {
        let mem = AgentMemory::new();
        mem.store("k", json!(1));
        mem.store("k", json!(2));
        assert_eq!(mem.len(), 1);
        assert_eq!(mem.get("k"), Some(json!(2)));
    }

    #[test]
    fn snapshot_returns_plain_values() {
        // Regresyon: snapshot() AgentMemoryRecord değil serde_json::Value döndürmeli.
        let mem = AgentMemory::new();
        mem.store("a", json!({"x": 1}));
        let snap = mem.snapshot();
        assert_eq!(snap.len(), 1);
        assert_eq!(snap[0].0, "a");
        assert_eq!(snap[0].1, json!({"x": 1}));
    }

    #[test]
    fn layers_are_separated_and_clearable() {
        let mem = AgentMemory::new();
        mem.store("w", json!("working"));
        mem.store_episodic("e", json!("episodic"));
        assert_eq!(count(&mem, MemoryLayer::Working), 1);
        assert_eq!(count(&mem, MemoryLayer::Episodic), 1);
        let rec = mem.get_record("e").expect("episodic record");
        assert_eq!(rec.layer, MemoryLayer::Episodic);
        assert_eq!(rec.tags, vec!["episodic".to_string()]);

        mem.clear_layer(MemoryLayer::Working);
        assert_eq!(count(&mem, MemoryLayer::Working), 0);
        assert_eq!(count(&mem, MemoryLayer::Episodic), 1);
    }

    #[test]
    fn working_chars_counts_characters_not_bytes() {
        let mem = AgentMemory::new();
        mem.store("k", json!("ğüşiöç")); // 6 karakter, 12 bayt
        assert_eq!(mem.working_chars(), 6);
    }

    #[test]
    fn compact_on_empty_working_is_noop() {
        let mem = AgentMemory::new();
        assert_eq!(mem.compact_working(), "");
        assert!(mem.is_empty());
    }

    #[test]
    fn entry_cap_triggers_compaction_into_summary() {
        let mem = AgentMemory::new().with_limits(1_000_000, 5);
        for i in 0..6 {
            mem.store(format!("k{i}"), json!(format!("value {i}")));
        }
        // 6 girdi > cap 5 → en eski %60 (3 adet) özetlenir.
        assert_eq!(count(&mem, MemoryLayer::Working), 3);
        assert_eq!(count(&mem, MemoryLayer::Summary), 1);
    }

    #[test]
    fn char_budget_triggers_compaction() {
        let mem = AgentMemory::new().with_limits(100, 1_000);
        mem.store("a", json!("x".repeat(60)));
        assert_eq!(count(&mem, MemoryLayer::Summary), 0);
        mem.store("b", json!("y".repeat(60)));
        assert_eq!(count(&mem, MemoryLayer::Summary), 1);
    }

    #[test]
    fn repeated_compactions_keep_every_summary() {
        // Regresyon: aynı saniyedeki compact'ler önceki özeti eziyordu.
        let mem = AgentMemory::new().with_limits(1_000_000, 2);
        for i in 0..4 {
            mem.store(format!("k{i}"), json!(i));
        }
        assert_eq!(count(&mem, MemoryLayer::Summary), 2);
    }

    #[test]
    fn episodic_is_never_compacted() {
        let mem = AgentMemory::new().with_limits(1_000_000, 2);
        mem.store_episodic("e", json!("keep me"));
        for i in 0..6 {
            mem.store(format!("k{i}"), json!(i));
        }
        assert_eq!(mem.get("e"), Some(json!("keep me")));
    }

    #[test]
    fn context_block_zero_budget_is_empty() {
        let mem = AgentMemory::new();
        mem.store("k", json!("value"));
        assert_eq!(mem.context_block(0), "");
    }

    #[test]
    fn context_block_labels_layers() {
        let mem = AgentMemory::new();
        mem.store("w", json!("work"));
        mem.store_episodic("e", json!("event"));
        let block = mem.context_block(10_000);
        assert!(block.contains("[working:w] work"));
        assert!(block.contains("[episodic:e] event"));
    }

    #[test]
    fn semantic_search_fallback_matches_key_and_value() {
        let mem = AgentMemory::new();
        mem.store("Project/Alpha", json!("Hello World"));
        mem.store("other", json!("nothing"));
        assert_eq!(mem.semantic_search("hello", 10).len(), 1);
        assert_eq!(mem.semantic_search("ALPHA", 10).len(), 1);
        assert!(mem.semantic_search("zzz", 10).is_empty());
        assert_eq!(mem.semantic_search("o", 1).len(), 1); // limit uygulanır
    }

    #[test]
    fn vector_port_is_used_and_kept_in_sync() {
        let mem = AgentMemory::new().with_vector(Arc::new(InMemoryVectorPort::default()));
        mem.store("doc", json!("rust agents runtime"));
        assert_eq!(mem.semantic_search("runtime", 5).len(), 1);
        // Regresyon: remove() vektör portunu da temizlemeli.
        assert!(mem.remove("doc"));
        assert!(mem.semantic_search("runtime", 5).is_empty());
    }

    #[test]
    fn in_memory_vector_port_ranks_by_matching_words() {
        let port = InMemoryVectorPort::default();
        port.upsert(&AgentMemoryRecord::new("one", json!("alpha"), MemoryLayer::Working));
        port.upsert(&AgentMemoryRecord::new("two", json!("alpha beta"), MemoryLayer::Working));
        let hits = port.search("alpha beta", 5);
        assert_eq!(hits.len(), 2);
        assert_eq!(hits[0].key, "two");
    }

    #[test]
    fn truncate_chars_is_utf8_safe() {
        assert_eq!(truncate_chars("ğüşiöç", 3), "ğüş…");
        assert_eq!(truncate_chars("ab", 5), "ab");
    }

    #[test]
    fn memory_layer_parse_defaults_to_working() {
        assert_eq!(MemoryLayer::parse(" Episodic "), MemoryLayer::Episodic);
        assert_eq!(MemoryLayer::parse("SUMMARY"), MemoryLayer::Summary);
        assert_eq!(MemoryLayer::parse("garbage"), MemoryLayer::Working);
    }
}
