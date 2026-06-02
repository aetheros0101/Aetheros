use async_trait::async_trait;

#[async_trait]
pub trait Plugin:
    Send + Sync
{
    fn name(
        &self,
    ) -> &'static str;

    async fn initialize(
        &self,
    );
}
