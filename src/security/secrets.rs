use async_trait::async_trait;

#[async_trait]
pub trait SecretProvider:
    Send + Sync
{
    async fn get_secret(
        &self,
        key: &str,
    ) -> Option<String>;
}
