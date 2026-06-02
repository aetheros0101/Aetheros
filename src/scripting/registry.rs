// ============================================================
// src/scripting/registry.rs
//
// İsimle kayıt + lookup.
// DashMap: lock-free concurrent okuma/yazma ✓
// ============================================================

use dashmap::DashMap;

use crate::scripting::definition::ScriptDefinition;

pub struct ScriptRegistry {
    scripts: DashMap<String, ScriptDefinition>,
}

impl ScriptRegistry {
    pub fn new() -> Self {
        Self {
            scripts: DashMap::new(),
        }
    }

    /// Script'i ismiyle kaydet.
    pub fn register(&self, script: ScriptDefinition) {
        self.scripts
            .insert(script.name.clone(), script);
    }

    /// İsimle script'i bul.
    pub fn get(
        &self,
        name: &str,
    ) -> Option<ScriptDefinition> {
        self.scripts.get(name).map(|s| s.clone())
    }

    pub fn remove(&self, name: &str) {
        self.scripts.remove(name);
    }

    pub fn list(&self) -> Vec<ScriptDefinition> {
        self.scripts
            .iter()
            .map(|e| e.value().clone())
            .collect()
    }

    pub fn count(&self) -> usize {
        self.scripts.len()
    }

    pub fn contains(&self, name: &str) -> bool {
        self.scripts.contains_key(name)
    }
}
