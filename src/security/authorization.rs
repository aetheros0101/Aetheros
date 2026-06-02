use crate::security::rbac::Role;

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
