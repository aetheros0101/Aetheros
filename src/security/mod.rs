// ============================================================
// src/security/mod.rs
//
// Sprint 6: identity modülü aktif edildi.
// ============================================================

pub mod audit;
pub mod capabilities;
pub mod identity;      // ← aktif edildi
pub mod isolation;
pub mod rbac;
pub mod secrets;
pub mod signatures;
pub mod policy;
pub mod authentication;
pub mod authorization;
pub mod governor;
pub mod auth;

// Sık kullanılan tipler — tek import'la erişim
pub use rbac::{Action, AuthzError, RbacGuard, Role};
pub use auth::token::{
    ApiKey,
    ApiKeyStore,
    AuthToken,
    Claims,
    TokenError,
    TokenManager,
};
