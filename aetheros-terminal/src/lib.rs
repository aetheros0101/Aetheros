//! Aetheros Terminal v0.4.0
//!
//! Standalone process and interactive PTY execution layer.
//!
//! The crate intentionally stays outside the Aetheros `src/` tree until the
//! terminal API is stable enough to integrate.

mod backend;
mod command;
mod environment;
mod errors;
mod event;
mod limits;
mod process;
mod pty;
mod session;
mod tool;

pub use backend::{ExecutionBackend, LocalProcessBackend};
pub use command::{CommandBuilder, CommandSpec};
pub use environment::Environment;
pub use errors::TerminalError;
pub use event::{TerminalEvent, TerminalEventBus};
pub use limits::ExecutionLimits;
pub use process::{ExitStatus, ProcessOutput, ProcessResult};
pub use pty::{PtyCommand, PtyEvent, PtySession, PtySessionManager, PtySizeSpec};
pub use session::{SessionId, TerminalSession, TerminalSessionManager};
pub use tool::{AgentTerminalTool, TerminalToolRequest, TerminalToolResponse};
