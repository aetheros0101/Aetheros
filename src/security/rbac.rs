// ============================================================
// src/security/rbac.rs
//
// Sprint 6: Tam RBAC sistemi
//
// ÖNCE: Role enum — sadece 4 variant, hiç permission yok
//
// SONRA:
//   Action enum     → tüm sistem eylemleri
//   Role::can()     → role-action permission matrisi
//   RbacGuard       → tek kontrol noktası
// ============================================================

use serde::{Deserialize, Serialize};

// ── Action ───────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Action {
    // Task
    TaskSubmit,
    TaskRead,
    TaskCancel,
    // Workflow
    WorkflowSubmit,
    WorkflowRead,
    // Agent
    AgentStart,
    AgentRead,
    // System
    SystemRead,
    SystemShutdown,
    // Admin
    UserManage,
    PolicyManage,
}

// ── Role ─────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Role {
    Admin,    // Her şey
    Operator, // Yaz + oku (shutdown hariç)
    Viewer,   // Sadece oku
    Agent,    // Task submit + read
}

impl Role {
    /// Bu role bu action'ı yapabilir mi?
    pub fn can(&self, action: &Action) -> bool {
        match self {
            Role::Admin => true, // Admin her şeye izinli

            Role::Operator => matches!(
                action,
                Action::TaskSubmit
                    | Action::TaskRead
                    | Action::TaskCancel
                    | Action::WorkflowSubmit
                    | Action::WorkflowRead
                    | Action::AgentStart
                    | Action::AgentRead
                    | Action::SystemRead
            ),

            Role::Viewer => matches!(
                action,
                Action::TaskRead
                    | Action::WorkflowRead
                    | Action::AgentRead
                    | Action::SystemRead
            ),

            Role::Agent => matches!(
                action,
                Action::TaskSubmit
                    | Action::TaskRead
                    | Action::AgentRead
            ),
        }
    }

    /// Role hiyerarşi seviyesi (yüksek = daha yetkili).
    pub fn level(&self) -> u8 {
        match self {
            Role::Admin    => 4,
            Role::Operator => 3,
            Role::Agent    => 2,
            Role::Viewer   => 1,
        }
    }

    pub fn is_admin(&self) -> bool {
        matches!(self, Role::Admin)
    }
}

// ── RbacGuard ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum AuthzError {
    Forbidden { role: String, action: String },
    Unauthenticated,
}

impl std::fmt::Display for AuthzError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Forbidden { role, action } => {
                write!(f, "Role '{}' cannot perform '{}'", role, action)
            }
            Self::Unauthenticated => write!(f, "Unauthenticated"),
        }
    }
}

pub struct RbacGuard;

impl RbacGuard {
    /// Tek kontrol noktası.
    pub fn authorize(
        role: &Role,
        action: &Action,
    ) -> Result<(), AuthzError> {
        if role.can(action) {
            Ok(())
        } else {
            Err(AuthzError::Forbidden {
                role: format!("{:?}", role),
                action: format!("{:?}", action),
            })
        }
    }
}
