// ============================================================
// src/tests/security_tests.rs
//
// SPRINT 6 — Auth + RBAC Testleri
// ============================================================

use crate::security::rbac::{
    Action,
    AuthzError,
    RbacGuard,
    Role,
};
use crate::security::auth::token::{
    ApiKey,
    Claims,
    TokenError,
    TokenManager,
};

// ── RBAC Permission Matrix Testleri ──────────────────────

#[test]
fn admin_can_do_everything() {
    let admin = Role::Admin;
    let actions = [
        Action::TaskSubmit,
        Action::TaskCancel,
        Action::WorkflowSubmit,
        Action::AgentStart,
        Action::SystemShutdown,
        Action::UserManage,
        Action::PolicyManage,
    ];
    for action in &actions {
        assert!(
            admin.can(action),
            "Admin {:?} yapabilmeli", action
        );
    }
}

#[test]
fn viewer_can_only_read() {
    let viewer = Role::Viewer;

    // İzinli
    assert!(viewer.can(&Action::TaskRead));
    assert!(viewer.can(&Action::WorkflowRead));
    assert!(viewer.can(&Action::AgentRead));
    assert!(viewer.can(&Action::SystemRead));

    // Yasak
    assert!(!viewer.can(&Action::TaskSubmit));
    assert!(!viewer.can(&Action::TaskCancel));
    assert!(!viewer.can(&Action::WorkflowSubmit));
    assert!(!viewer.can(&Action::AgentStart));
    assert!(!viewer.can(&Action::SystemShutdown));
    assert!(!viewer.can(&Action::UserManage));
}

#[test]
fn operator_cannot_shutdown_or_manage_users() {
    let op = Role::Operator;
    assert!(!op.can(&Action::SystemShutdown));
    assert!(!op.can(&Action::UserManage));
    assert!(!op.can(&Action::PolicyManage));

    // Ama şunlar olabilir
    assert!(op.can(&Action::TaskSubmit));
    assert!(op.can(&Action::WorkflowSubmit));
    assert!(op.can(&Action::AgentStart));
}

#[test]
fn agent_role_limited_to_task_operations() {
    let agent = Role::Agent;
    assert!(agent.can(&Action::TaskSubmit));
    assert!(agent.can(&Action::TaskRead));
    assert!(agent.can(&Action::AgentRead));

    assert!(!agent.can(&Action::TaskCancel));
    assert!(!agent.can(&Action::WorkflowSubmit));
    assert!(!agent.can(&Action::SystemShutdown));
}

#[test]
fn role_hierarchy_levels() {
    assert!(Role::Admin.level() > Role::Operator.level());
    assert!(Role::Operator.level() > Role::Agent.level());
    assert!(Role::Agent.level() > Role::Viewer.level());
}

// ── RbacGuard Testleri ────────────────────────────────────

#[test]
fn rbac_guard_allows_permitted_action() {
    let result = RbacGuard::authorize(
        &Role::Operator,
        &Action::TaskSubmit,
    );
    assert!(result.is_ok());
}

#[test]
fn rbac_guard_denies_forbidden_action() {
    let result = RbacGuard::authorize(
        &Role::Viewer,
        &Action::TaskSubmit,
    );
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        AuthzError::Forbidden { .. }
    ));
}

#[test]
fn rbac_guard_error_contains_role_and_action() {
    let err = RbacGuard::authorize(
        &Role::Viewer,
        &Action::SystemShutdown,
    )
    .unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("Viewer"));
    assert!(msg.contains("SystemShutdown"));
}

// ── Claims Testleri ───────────────────────────────────────

#[test]
fn claims_not_expired_when_fresh() {
    let claims = Claims::new("user-1", Role::Operator, 3600);
    assert!(!claims.is_expired());
    assert!(claims.ttl_remaining() > 3500);
}

#[test]
fn claims_expired_when_past() {
    use std::time::{SystemTime, UNIX_EPOCH};
    let mut claims = Claims::new("user-1", Role::Viewer, 3600);
    // exp'yi 1 saat geriye al
    let past = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
        .saturating_sub(7200); // 2 saat önce
    claims.exp = past;
    assert!(claims.is_expired());
    assert_eq!(claims.ttl_remaining(), 0);
}

// ── TokenManager Testleri ────────────────────────────────

#[test]
fn token_generate_and_verify() {
    let manager =
        TokenManager::new("test-secret", 3600);

    let token = manager.generate("alice", Role::Admin);
    assert!(!token.access_token.is_empty());
    assert_eq!(token.token_type, "Bearer");

    let claims =
        manager.verify(&token.access_token).unwrap();
    assert_eq!(claims.sub, "alice");
    assert_eq!(claims.role, Role::Admin);
}

#[test]
fn token_wrong_secret_fails_verification() {
    let manager1 = TokenManager::new("secret-1", 3600);
    let manager2 = TokenManager::new("secret-2", 3600);

    let token =
        manager1.generate("bob", Role::Operator);
    let result = manager2.verify(&token.access_token);

    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        TokenError::InvalidSignature
    ));
}

#[test]
fn token_malformed_returns_error() {
    let manager = TokenManager::new("secret", 3600);
    let result = manager.verify("not.a.valid.jwt.at.all");
    assert!(matches!(
        result.unwrap_err(),
        TokenError::InvalidSignature | TokenError::Malformed
    ));
}

#[test]
fn token_empty_returns_error() {
    let manager = TokenManager::new("secret", 3600);
    let result = manager.verify("");
    assert!(result.is_err());
}

// ── ApiKey Testleri ───────────────────────────────────────

#[test]
fn api_key_verify_correct_key() {
    let key = ApiKey::new("my-secret-api-key");
    assert!(key.verify("my-secret-api-key"));
}

#[test]
fn api_key_verify_wrong_key_fails() {
    let key = ApiKey::new("correct-key");
    assert!(!key.verify("wrong-key"));
    assert!(!key.verify(""));
    assert!(!key.verify("correct-key-extra"));
}

#[test]
fn api_key_hash_is_deterministic() {
    let key1 = ApiKey::new("test-key");
    let key2 = ApiKey::new("test-key");
    assert_eq!(key1.hash(), key2.hash());
}

#[test]
fn api_key_different_keys_different_hashes() {
    let key1 = ApiKey::new("key-one");
    let key2 = ApiKey::new("key-two");
    assert_ne!(key1.hash(), key2.hash());
}


#[test]
fn jwt_roundtrip_uses_valid_jwt_encoding() {
    let manager = TokenManager::new("test-secret", 3600);
    let token = manager.generate("user-1", Role::Operator);
    let claims = manager.verify(&token.access_token).unwrap();
    assert_eq!(claims.sub, "user-1");
    assert_eq!(claims.role, Role::Operator);
    assert_eq!(token.access_token.split('.').count(), 3);
}

#[test]
fn jwt_tampering_is_rejected() {
    let manager = TokenManager::new("test-secret", 3600);
    let token = manager.generate("user-1", Role::Operator);
    let mut parts: Vec<_> = token.access_token.split('.').map(str::to_string).collect();
    parts[1].push('A');
    assert_eq!(manager.verify(&parts.join(".")), Err(TokenError::InvalidSignature));
}
