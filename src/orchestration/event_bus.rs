use tokio::sync::broadcast;

use crate::orchestration::events::OrchestrationEvent;

pub struct OrchestrationEventBus {
    sender:
        broadcast::Sender<
            OrchestrationEvent,
        >,
}

impl OrchestrationEventBus {
    pub fn new() -> Self {
        let (sender, _) =
            broadcast::channel(1024);

        Self { sender }
    }

    pub fn publish(
        &self,
        event:
            OrchestrationEvent,
    ) {
        let _ =
            self.sender.send(
                event,
            );
    }

    pub fn subscribe(
        &self,
    ) -> broadcast::Receiver<
        OrchestrationEvent,
    > {
        self.sender.subscribe()
    }
}
