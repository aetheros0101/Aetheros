//pub mod otlp;
pub mod stdout;
pub mod console;

use async_trait::async_trait;

#[async_trait]
pub trait LogExporter:
    Send + Sync
{
    async fn export(
        &self,
        payload: String,
    );
}
