use std::sync::Arc;

use dashmap::DashMap;

use tokio_util::sync::CancellationToken;

use crate::types::ids::TaskId;

#[derive(Clone)]
pub struct CancellationRegistry {
    tokens:
        Arc<
            DashMap<
                TaskId,
                CancellationToken,
            >,
        >,
}

impl CancellationRegistry {
    pub fn new() -> Self {
        Self {
            tokens: Arc::new(
                DashMap::new(),
            ),
        }
    }

    pub fn register(
        &self,
        task_id: TaskId,
    ) -> CancellationToken {
        let token =
            CancellationToken::new();

        self.tokens.insert(
            task_id,
            token.clone(),
        );

        token
    }

    pub fn cancel(
        &self,
        task_id: &TaskId,
    ) {
        if let Some(token) =
            self.tokens.get(task_id)
        {
            token.cancel();
        }
    }

    pub fn remove(
        &self,
        task_id: &TaskId,
    ) {
        self.tokens.remove(task_id);
    }
}
