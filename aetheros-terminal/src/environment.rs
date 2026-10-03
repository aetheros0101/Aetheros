use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Environment {
    vars: BTreeMap<String, String>,
    clear: bool,
}

impl Environment {
    pub fn new() -> Self { Self::default() }

    pub fn clear(mut self) -> Self {
        self.clear = true;
        self
    }

    pub fn set(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.vars.insert(key.into(), value.into());
        self
    }

    pub(crate) fn apply(&self, command: &mut tokio::process::Command) {
        if self.clear {
            command.env_clear();
        }
        for (key, value) in &self.vars {
            command.env(key, value);
        }
    }
}
