// ============================================================
// src/security/mod.rs
//
// Sprint 6: identity modülü aktif edildi.
// ============================================================

pub mod audit;
pub mod capabilities;
pub mod capability_engine;
pub mod risk_engine;
pub mod identity;      // ← aktif edildi
pub mod isolation;
pub mod rbac;
pub mod secrets;
pub mod signatures;
pub mod policy;
pub mod authentication;
pub mod authorization;
pub mod governor;
pub mod command_policy;
pub mod auth;

// Sık kullanılan tipler — tek import'la erişim
pub use rbac::{Action, AuthzError, RbacGuard, Role};
pub use capability_engine::{CapabilityDecision, CapabilityEngine};
pub use risk_engine::{RiskAssessment, RiskEngine};
pub use governor::{GovernorDecision, HighRiskPolicy, SecurityGovernor};
pub use auth::token::{
    ApiKey,
    ApiKeyStore,
    AuthToken,
    Claims,
    TokenError,
    TokenManager,
};
