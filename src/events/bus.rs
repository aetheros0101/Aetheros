use tokio::sync::broadcast;

use crate::events::runtime::RuntimeEvent;
use crate::events::task::TaskEvent;
use crate::events::telemetry::TelemetryEvent;
use crate::events::worker::WorkerEvent;

#[derive(Debug, Clone)]
pub enum SystemEvent {
    Runtime(RuntimeEvent),
    Task(TaskEvent),
    Worker(WorkerEvent),
    Telemetry(TelemetryEvent),
}

#[derive(Clone)]
pub struct EventBus {
    sender: broadcast::Sender<SystemEvent>,
}

impl EventBus {
    pub fn new(
        capacity: usize,
    ) -> Self {
        let (sender, _) =
            broadcast::channel(capacity);

        Self { sender }
    }

    pub fn publish(
        &self,
        event: SystemEvent,
    ) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(
        &self,
    ) -> broadcast::Receiver<SystemEvent> {
        self.sender.subscribe()
    }
}
