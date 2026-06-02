use async_trait::async_trait;

#[async_trait]
pub trait AgentTool:
    Send + Sync
{
    fn name(
        &self,
    ) -> &'static str;

    async fn invoke(
        &self,
        arguments:
            Vec<String>,
    ) -> Result<
        String,
        String,
    >;
}
