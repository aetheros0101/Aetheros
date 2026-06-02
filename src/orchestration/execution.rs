use chrono::{
    DateTime,
    Utc,
};

use serde::{
    Deserialize,
    Serialize,
};

use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use async_trait::async_trait;

use crate::errors::runtime::RuntimeError;

#[derive(Clone)]
pub struct ExecutionContext {
    pub execution_id:
        Uuid,

    pub correlation_id:
        String,

    pub cancellation:
        CancellationToken,

    pub started_at:
        DateTime<Utc>,
}

#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct ExecutionResult<T> {
    pub execution_id:
        Uuid,

    pub success: bool,

    pub output: T,
}

#[async_trait]
pub trait ExecutionUnit:
    Send + Sync
{
    type Input;
    type Output;

    async fn execute(
        &self,
        input: Self::Input,
    ) -> Result<
        Self::Output,
        RuntimeError,
    >;
}
