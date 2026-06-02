pub mod api;
pub mod backpressure;
pub mod bootstrap;
pub mod config;
pub mod dispatcher;
pub mod lifecycle;
pub mod runtime;
pub mod scheduler;
pub mod shutdown;

use async_trait::async_trait;

use crate::errors::runtime::RuntimeError;

#[async_trait]
pub trait RuntimeService {
    async fn start(&self) -> Result<(), RuntimeError>;

    async fn shutdown(&self) -> Result<(), RuntimeError>;
}
