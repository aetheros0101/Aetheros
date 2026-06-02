use tokio::sync::broadcast;

use crate::orchestration::events::OrchestrationEvent;

pub struct DurableEventStream {
    sender:
        broadcast::Sender<
            OrchestrationEvent,
        >,
}

impl DurableEventStream {
    pub fn new() -> Self {
        let (
            sender,
            _,
        ) = broadcast::channel(
            4096,
        );

        Self { sender }
    }

    pub fn publish(
        &self,
        event:
            OrchestrationEvent,
    ) {
        let _ = self
            .sender
            .send(event);
    }

    pub fn subscribe(
        &self,
    ) -> broadcast::Receiver<
        OrchestrationEvent,
    > {
        self.sender
            .subscribe()
    }
}
