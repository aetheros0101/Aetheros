use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ShutdownController {
    token: CancellationToken,
}

impl ShutdownController {
    pub fn new() -> Self {
        Self {
            token:
                CancellationToken::new(),
        }
    }

    pub fn cancel(
        &self,
    ) {
        self.token.cancel();
    }

    pub async fn wait(
        &self,
    ) {
        self.token.cancelled().await;
    }

    pub async fn wait_timeout(
        &self,
        duration: std::time::Duration,
    ) -> bool {
        tokio::time::timeout(
            duration,
            self.token.cancelled(),
        )
        .await
        .is_ok()
    }

    pub fn child_token(
        &self,
    ) -> CancellationToken {
        self.token.child_token()
    }
}
