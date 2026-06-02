use serde::{
    Deserialize,
    Serialize,
};

use async_trait::async_trait;

#[async_trait]
pub trait SignatureVerifier:
    Send + Sync
{
    async fn verify(
        &self,
        payload: &[u8],
    ) -> bool;
}
#[derive(
    Debug,
    Clone,
    Serialize,
    Deserialize,
)]
pub struct WasmSignature {
    pub algorithm:
        String,

    pub signature:
        String,
}
