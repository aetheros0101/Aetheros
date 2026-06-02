#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkerState {
    Starting,
    Idle,
    Busy,
    Stopping,
    Stopped,
}
