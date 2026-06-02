use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error(
        "provider unavailable"
    )]
    ProviderUnavailable,

    #[error(
        "rate limit exceeded"
    )]
    RateLimited,

    #[error(
        "token budget exceeded"
    )]
    TokenBudgetExceeded,

    #[error(
        "invalid structured output"
    )]
    InvalidStructuredOutput,

    #[error(
        "provider failure: {message}"
    )]
    ProviderFailure {
        message: String,
    },
}
