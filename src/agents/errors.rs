use thiserror::Error;

#[derive(Debug, Error)]
pub enum AgentError {
    #[error(
        "planning failed"
    )]
    PlanningFailed,

    #[error(
        "tool invocation failed"
    )]
    ToolInvocationFailed,

    #[error(
        "capability denied"
    )]
    CapabilityDenied,

    #[error(
        "execution budget exceeded"
    )]
    BudgetExceeded,

    #[error(
        "agent cancelled"
    )]
    Cancelled,
}
