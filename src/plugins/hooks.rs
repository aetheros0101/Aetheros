use async_trait::async_trait;

#[async_trait]
pub trait PluginHooks:
    Send + Sync
{
    async fn on_load(
        &self,
    );

    async fn on_unload(
        &self,
    );
}
