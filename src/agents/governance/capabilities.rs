//! Agent yetenek çipleri ve politika profilleri (P2).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct AgentCapabilities {
    pub workflow_execution: bool,
    pub wasm_execution: bool,
    pub ai_reasoning: bool,
    pub remote_execution: bool,
    pub terminal_execution: bool,
    #[serde(default)]
    pub workspace_read: bool,
    #[serde(default)]
    pub workspace_write: bool,
    #[serde(default)]
    pub workspace_mutate: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentCapability {
    WorkflowExecution,
    WasmExecution,
    AiReasoning,
    RemoteExecution,
    TerminalExecution,
    WorkspaceRead,
    WorkspaceWrite,
    WorkspaceMutate,
}

impl AgentCapability {
    pub fn chip_label(&self) -> &'static str {
        match self {
            AgentCapability::WorkflowExecution => "Workflow",
            AgentCapability::WasmExecution => "WASM",
            AgentCapability::AiReasoning => "AI akıl yürütme",
            AgentCapability::RemoteExecution => "Uzak düğüm",
            AgentCapability::TerminalExecution => "Terminal",
            AgentCapability::WorkspaceRead => "Dosya okuma",
            AgentCapability::WorkspaceWrite => "Dosya yazma",
            AgentCapability::WorkspaceMutate => "Dosya silme/taşıma",
        }
    }

    pub const ALL: [AgentCapability; 8] = [
        AgentCapability::WorkflowExecution,
        AgentCapability::WasmExecution,
        AgentCapability::AiReasoning,
        AgentCapability::RemoteExecution,
        AgentCapability::TerminalExecution,
        AgentCapability::WorkspaceRead,
        AgentCapability::WorkspaceWrite,
        AgentCapability::WorkspaceMutate,
    ];
}

impl AgentCapabilities {
    pub fn allows(&self, cap: AgentCapability) -> bool {
        match cap {
            AgentCapability::WorkflowExecution => self.workflow_execution,
            AgentCapability::WasmExecution => self.wasm_execution,
            AgentCapability::AiReasoning => self.ai_reasoning,
            AgentCapability::RemoteExecution => self.remote_execution,
            AgentCapability::TerminalExecution => self.terminal_execution,
            AgentCapability::WorkspaceRead => self.workspace_read,
            AgentCapability::WorkspaceWrite => self.workspace_write,
            AgentCapability::WorkspaceMutate => self.workspace_mutate,
        }
    }

    pub fn enabled_list(&self) -> Vec<AgentCapability> {
        AgentCapability::ALL
            .iter()
            .copied()
            .filter(|c| self.allows(*c))
            .collect()
    }

    /// İki set kesişimi (host çipleri ∩ profil).
    pub fn intersect(&self, other: &Self) -> Self {
        Self {
            workflow_execution: self.workflow_execution && other.workflow_execution,
            wasm_execution: self.wasm_execution && other.wasm_execution,
            ai_reasoning: self.ai_reasoning && other.ai_reasoning,
            remote_execution: self.remote_execution && other.remote_execution,
            terminal_execution: self.terminal_execution && other.terminal_execution,
            workspace_read: self.workspace_read && other.workspace_read,
            workspace_write: self.workspace_write && other.workspace_write,
            workspace_mutate: self.workspace_mutate && other.workspace_mutate,
        }
    }

    /// Kurumsal hazır profil setleri.
    pub fn profile(name: &str) -> Self {
        PolicyProfile::parse(name).capabilities()
    }
}

/// Ortam / UI'dan seçilen politika profili.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyProfile {
    /// Yalnız okuma + AI.
    ReadOnly,
    /// Geliştirici: okuma/yazma + terminal.
    Developer,
    /// Bakım: mutate + workflow.
    Maintainer,
    /// Prod kilitli: minimal yüzey.
    ProdLocked,
    /// Host'un verdiği çipler (kısıtlama yok).
    Default,
}

impl PolicyProfile {
    pub fn parse(name: &str) -> Self {
        match name.trim().to_ascii_lowercase().as_str() {
            "read_only" | "readonly" | "ro" => PolicyProfile::ReadOnly,
            "developer" | "dev" => PolicyProfile::Developer,
            "maintainer" | "admin" => PolicyProfile::Maintainer,
            "prod_locked" | "prod" | "production" => PolicyProfile::ProdLocked,
            _ => PolicyProfile::Default,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            PolicyProfile::ReadOnly => "read_only",
            PolicyProfile::Developer => "developer",
            PolicyProfile::Maintainer => "maintainer",
            PolicyProfile::ProdLocked => "prod_locked",
            PolicyProfile::Default => "default",
        }
    }

    pub fn capabilities(self) -> AgentCapabilities {
        match self {
            PolicyProfile::ReadOnly => AgentCapabilities {
                ai_reasoning: true,
                workspace_read: true,
                ..Default::default()
            },
            PolicyProfile::Developer => AgentCapabilities {
                ai_reasoning: true,
                workspace_read: true,
                workspace_write: true,
                terminal_execution: true,
                ..Default::default()
            },
            PolicyProfile::Maintainer => AgentCapabilities {
                ai_reasoning: true,
                workspace_read: true,
                workspace_write: true,
                workspace_mutate: true,
                terminal_execution: true,
                workflow_execution: true,
                ..Default::default()
            },
            PolicyProfile::ProdLocked => AgentCapabilities {
                ai_reasoning: true,
                workspace_read: true,
                ..Default::default()
            },
            PolicyProfile::Default => AgentCapabilities {
                ai_reasoning: true,
                workspace_read: true,
                workspace_write: true,
                ..Default::default()
            },
        }
    }

    /// Host çipleri ile profil kesişimi — efektif yetki.
    pub fn resolve(self, host_caps: &AgentCapabilities) -> AgentCapabilities {
        self.capabilities().intersect(host_caps)
    }
}

impl Default for PolicyProfile {
    fn default() -> Self {
        PolicyProfile::Default
    }
}
