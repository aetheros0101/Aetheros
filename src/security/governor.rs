use crate::security::policy::SecurityPolicy;

pub struct SecurityGovernor;

impl SecurityGovernor {
    pub fn allowed(
        policy:
            &SecurityPolicy,
    ) -> bool {
        policy.audit_required
    }
}
