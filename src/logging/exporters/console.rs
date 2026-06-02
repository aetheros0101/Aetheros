use async_trait::async_trait;

use crate::logging::exporters::LogExporter;

pub struct ConsoleExporter;

#[async_trait]
impl LogExporter
    for ConsoleExporter
{
    async fn export(
        &self,
        payload: String,
    ) {
        println!(
            "{}",
            payload
        );
    }
}
