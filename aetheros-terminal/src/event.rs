use tokio::sync::broadcast;

use crate::{PtyEvent, SessionId};

/// Events emitted by the terminal subsystem.
///
/// The bus is intentionally observational: publishing an event never blocks
/// terminal execution. Slow subscribers may receive `Lagged` from the
/// underlying broadcast channel and should recover by resyncing their state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalEvent {
    SessionCreated { session_id: SessionId },
    SessionRemoved { session_id: SessionId },
    PtyAttached { session_id: SessionId },
    PtyDetached { session_id: SessionId },
    PtyOutput { session_id: SessionId, data: Vec<u8> },
    PtyExited { session_id: SessionId, code: Option<i32> },
}

impl TerminalEvent {
    pub fn session_id(&self) -> SessionId {
        match self {
            Self::SessionCreated { session_id }
            | Self::SessionRemoved { session_id }
            | Self::PtyAttached { session_id }
            | Self::PtyDetached { session_id }
            | Self::PtyOutput { session_id, .. }
            | Self::PtyExited { session_id, .. } => *session_id,
        }
    }

    pub fn from_pty(session_id: SessionId, event: PtyEvent) -> Self {
        match event {
            PtyEvent::Output(data) => Self::PtyOutput { session_id, data },
            PtyEvent::Exited(code) => Self::PtyExited { session_id, code },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TerminalEventBus {
    sender: broadcast::Sender<TerminalEvent>,
}

impl Default for TerminalEventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl TerminalEventBus {
    pub fn new() -> Self {
        Self::with_capacity(512)
    }

    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity.max(1));
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TerminalEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: TerminalEvent) {
        let _ = self.sender.send(event);
    }
}
