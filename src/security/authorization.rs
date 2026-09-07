use crate::security::rbac::Role;

/// NOT: Bu, minimal/eski bir kontrol noktasıdır. Gerçek yetkilendirme
/// artık `crate::security::rbac::RbacGuard` üzerinden, `Action` bazlı
/// ve `src/api/middleware.rs`'teki `require_auth`/`require_admin`
/// middleware'leriyle uygulanır. Bu tip yeni kod tarafından
/// kullanılmamalı — geriye dönük uyumluluk için tutuluyor.
pub struct AuthorizationPolicy;

impl AuthorizationPolicy {
    pub fn allowed(
        role: &Role,
    ) -> bool {
        matches!(
            role,
            Role::Admin
                | Role::Operator
        )
    }
}
