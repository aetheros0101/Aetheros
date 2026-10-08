// ============================================================
// src/security/mod.rs
//
// Sprint 6: identity modülü aktif edildi.
// ============================================================

pub mod audit;
pub mod auth;
pub mod capabilities;
pub mod capability_engine;
pub mod command_policy;
pub mod governor;
pub mod path_confinement;
pub mod policy;
pub mod rbac;
pub mod risk_engine;

// Sık kullanılan tipler — tek import'la erişim
pub use auth::token::{ApiKey, ApiKeyStore, AuthToken, Claims, TokenError, TokenManager};
pub use capability_engine::{CapabilityDecision, CapabilityEngine};
pub use governor::{GovernorDecision, HighRiskPolicy, SecurityGovernor};
pub use rbac::{Action, AuthzError, RbacGuard, Role};
pub use risk_engine::{RiskAssessment, RiskEngine};
