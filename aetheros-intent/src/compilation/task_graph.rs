//! Task graph — agent'a verilebilir iş tanımı + DAG.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap, HashSet};
use uuid::Uuid;

use crate::compilation::dependency::{DependencyEdge, DependencyKind};
use crate::errors::{IntentError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskRole {
    Analyze,
    Architecture,
    Frontend,
    Backend,
    UiAuth,
    Database,
    Api,
    Integration,
    Testing,
    Verification,
    Edit,
    Research,
    Implement,
    Custom,
}

impl TaskRole {
    pub fn label(self) -> &'static str {
        match self {
            TaskRole::Analyze => "Analyze",
            TaskRole::Architecture => "Architecture",
            TaskRole::Frontend => "Frontend",
            TaskRole::Backend => "Backend",
            TaskRole::UiAuth => "UI/Auth",
            TaskRole::Database => "Database",
            TaskRole::Api => "API",
            TaskRole::Integration => "Integration",
            TaskRole::Testing => "Testing",
            TaskRole::Verification => "Verification",
            TaskRole::Edit => "Edit",
            TaskRole::Research => "Research",
            TaskRole::Implement => "Implement",
            TaskRole::Custom => "Custom",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    #[default]
    Pending,
    Ready,
    Running,
    Blocked,
    Done,
    Skipped,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum TaskPriority {
    Low,
    #[default]
    Medium,
    High,
    Critical,
}

/// Agent'ın "bu task neden var?" sorusuna cevap veren doğrulama kancası.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskVerification {
    /// Nasıl doğrulanacak (test, manuel, lint…).
    pub method: Option<String>,
    /// Beklenen sonuç özeti.
    pub expected: Option<String>,
    #[serde(default)]
    pub done: bool,
}

/// Derlenmiş, agent'a verilebilir görev düğümü.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskNode {
    pub id: Uuid,
    pub role: TaskRole,
    pub title: String,
    #[serde(default)]
    pub description: String,

    /// Kaynak functional / NFR id (RequirementSet içinden).
    #[serde(default)]
    pub source_requirement: Option<Uuid>,
    #[serde(default)]
    pub acceptance_criteria: Vec<String>,

    #[serde(default)]
    pub status: TaskStatus,
    #[serde(default)]
    pub parent: Option<Uuid>,
    /// Kenar listesine ek olarak hızlı erişim (from → self).
    #[serde(default)]
    pub dependencies: Vec<Uuid>,

    #[serde(default)]
    pub domain_tags: Vec<String>,
    /// Agent capability ipuçları (workspace_read, terminal…).
    #[serde(default)]
    pub capabilities: Vec<String>,

    #[serde(default)]
    pub effort: u32,
    #[serde(default)]
    pub priority: TaskPriority,

    #[serde(default)]
    pub verification: TaskVerification,
}

impl TaskNode {
    pub fn new(role: TaskRole, title: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            role,
            title: title.into(),
            description: String::new(),
            source_requirement: None,
            acceptance_criteria: Vec::new(),
            status: TaskStatus::Pending,
            parent: None,
            dependencies: Vec::new(),
            domain_tags: Vec::new(),
            capabilities: Vec::new(),
            effort: 1,
            priority: TaskPriority::Medium,
            verification: TaskVerification::default(),
        }
    }

    pub fn with_source(mut self, req_id: Uuid) -> Self {
        self.source_requirement = Some(req_id);
        self
    }

    pub fn with_acceptance(mut self, criteria: Vec<String>) -> Self {
        self.acceptance_criteria = criteria;
        self
    }

    pub fn with_capabilities(mut self, caps: &[&str]) -> Self {
        self.capabilities = caps.iter().map(|s| (*s).to_string()).collect();
        self
    }
}

/// Mevcut AetherOS Task Layer'a aktarım için sade DTO.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledTask {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub role: TaskRole,
    pub source_requirement: Option<Uuid>,
    pub acceptance_criteria: Vec<String>,
    pub capabilities: Vec<String>,
    pub depends_on: Vec<Uuid>,
    pub priority: TaskPriority,
    pub verification: TaskVerification,
}

impl From<&TaskNode> for CompiledTask {
    fn from(n: &TaskNode) -> Self {
        Self {
            id: n.id,
            title: n.title.clone(),
            description: n.description.clone(),
            role: n.role,
            source_requirement: n.source_requirement,
            acceptance_criteria: n.acceptance_criteria.clone(),
            capabilities: n.capabilities.clone(),
            depends_on: n.dependencies.clone(),
            priority: n.priority,
            verification: n.verification.clone(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskGraph {
    pub nodes: HashMap<Uuid, TaskNode>,
    pub edges: Vec<DependencyEdge>,
    pub root: Option<Uuid>,
}

impl TaskGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_node(&mut self, node: TaskNode) -> Uuid {
        let id = node.id;
        if self.root.is_none() {
            self.root = Some(id);
        }
        self.nodes.insert(id, node);
        id
    }

    pub fn add_edge(&mut self, from: Uuid, to: Uuid, kind: DependencyKind) -> Result<()> {
        if !self.nodes.contains_key(&from) || !self.nodes.contains_key(&to) {
            return Err(IntentError::TaskGraph(
                "edge endpoints must exist in graph".into(),
            ));
        }
        // Döngü kontrolü DEĞİŞİKLİKTEN ÖNCE: hatada graf bozulmadan kalır.
        if from == to || self.reaches(to, from) {
            return Err(IntentError::DependencyCycle(format!("{from} -> {to}")));
        }
        self.edges.push(DependencyEdge { from, to, kind });
        if let Some(n) = self.nodes.get_mut(&to) {
            if !n.dependencies.contains(&from) {
                n.dependencies.push(from);
            }
        }
        Ok(())
    }

    /// `start`'tan kenarlar boyunca `target`'a ulaşılabilir mi?
    fn reaches(&self, start: Uuid, target: Uuid) -> bool {
        let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for e in &self.edges {
            adj.entry(e.from).or_default().push(e.to);
        }
        let mut seen = HashSet::new();
        let mut stack = vec![start];
        while let Some(n) = stack.pop() {
            if n == target {
                return true;
            }
            if seen.insert(n) {
                if let Some(next) = adj.get(&n) {
                    stack.extend(next.iter().copied());
                }
            }
        }
        false
    }

    pub fn children(&self, parent: Uuid) -> Vec<Uuid> {
        self.nodes
            .values()
            .filter(|n| n.parent == Some(parent))
            .map(|n| n.id)
            .collect()
    }

    /// Deterministik topolojik sıra: hazır düğümler arasında (başlık, id)
    /// sırasıyla seçilir; aynı graf her zaman aynı sırayı verir.
    pub fn topological_order(&self) -> Result<Vec<Uuid>> {
        let mut indeg: HashMap<Uuid, usize> = self.nodes.keys().map(|id| (*id, 0)).collect();
        let mut adj: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
        for e in &self.edges {
            *indeg.entry(e.to).or_insert(0) += 1;
            adj.entry(e.from).or_default().push(e.to);
        }
        let key = |id: &Uuid| {
            (
                self.nodes.get(id).map(|n| n.title.clone()).unwrap_or_default(),
                *id,
            )
        };
        let mut ready: BTreeSet<(String, Uuid)> = indeg
            .iter()
            .filter(|(_, d)| **d == 0)
            .map(|(id, _)| key(id))
            .collect();
        let mut order = Vec::new();
        while let Some((_, n)) = ready.pop_first() {
            order.push(n);
            if let Some(nexts) = adj.get(&n) {
                for t in nexts {
                    if let Some(d) = indeg.get_mut(t) {
                        *d -= 1;
                        if *d == 0 {
                            ready.insert(key(t));
                        }
                    }
                }
            }
        }
        if order.len() != self.nodes.len() {
            return Err(IntentError::DependencyCycle(
                "cycle prevents full topological order".into(),
            ));
        }
        Ok(order)
    }

    /// Hiyerarşi derinliği: kök = 1, `parent` zinciri boyunca en uzun yol.
    pub fn hierarchy_depth(&self) -> usize {
        let mut best = 0;
        for id in self.nodes.keys() {
            let mut depth = 1;
            let mut cur = *id;
            let mut guard = 0;
            while let Some(p) = self.nodes.get(&cur).and_then(|n| n.parent) {
                depth += 1;
                cur = p;
                guard += 1;
                if guard > self.nodes.len() {
                    break; // parent döngüsüne karşı
                }
            }
            best = best.max(depth);
        }
        best
    }

    /// Paralellik genişliği: bağımlılık katmanlarındaki (longest-path level)
    /// en kalabalık katmanın düğüm sayısı.
    pub fn max_parallel_width(&self) -> Result<usize> {
        let order = self.topological_order()?;
        let mut level: HashMap<Uuid, usize> = HashMap::new();
        for id in &order {
            let l = self
                .nodes
                .get(id)
                .map(|n| {
                    n.dependencies
                        .iter()
                        .filter_map(|d| level.get(d))
                        .max()
                        .map_or(0, |m| m + 1)
                })
                .unwrap_or(0);
            level.insert(*id, l);
        }
        let mut counts: HashMap<usize, usize> = HashMap::new();
        for l in level.values() {
            *counts.entry(*l).or_insert(0) += 1;
        }
        Ok(counts.values().copied().max().unwrap_or(0))
    }

    pub fn detect_cycle(&self) -> Result<()> {
        self.topological_order().map(|_| ())
    }

    pub fn ready_nodes(&self) -> Vec<Uuid> {
        let done: HashSet<Uuid> = self
            .nodes
            .values()
            .filter(|n| matches!(n.status, TaskStatus::Done | TaskStatus::Skipped))
            .map(|n| n.id)
            .collect();
        self.nodes
            .values()
            .filter(|n| matches!(n.status, TaskStatus::Pending | TaskStatus::Ready))
            .filter(|n| n.dependencies.iter().all(|d| done.contains(d)))
            .map(|n| n.id)
            .collect()
    }

    pub fn set_status(&mut self, id: Uuid, status: TaskStatus) -> Result<()> {
        let n = self
            .nodes
            .get_mut(&id)
            .ok_or_else(|| IntentError::UnknownNode(id.to_string()))?;
        n.status = status;
        Ok(())
    }

    /// Mevcut Task Layer adaptörü için düz liste (topo sırada).
    pub fn to_compiled_tasks(&self) -> Result<Vec<CompiledTask>> {
        let order = self.topological_order()?;
        Ok(order
            .iter()
            .filter_map(|id| self.nodes.get(id).map(CompiledTask::from))
            .collect())
    }

    pub fn outline(&self) -> String {
        let mut lines = Vec::new();
        if let Some(root) = self.root {
            self.outline_rec(root, 0, &mut lines, &mut HashSet::new());
        }
        lines.join("\n")
    }

    fn outline_rec(
        &self,
        id: Uuid,
        depth: usize,
        lines: &mut Vec<String>,
        seen: &mut HashSet<Uuid>,
    ) {
        if !seen.insert(id) {
            return;
        }
        if let Some(n) = self.nodes.get(&id) {
            let src = n
                .source_requirement
                .map(|u| format!(" ←{u}"))
                .unwrap_or_default();
            lines.push(format!(
                "{}{} [{}]{}",
                "  ".repeat(depth),
                n.title,
                n.role.label(),
                src
            ));
            let mut kids = self.children(id);
            kids.sort_by_key(|k| self.nodes.get(k).map(|n| n.title.clone()).unwrap_or_default());
            for c in kids {
                self.outline_rec(c, depth + 1, lines, seen);
            }
        }
    }
}
