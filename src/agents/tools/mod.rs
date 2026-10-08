//! Agent araçları: workspace, terminal.

pub mod tool_invocation;
// `runtime::runtime`-tarzı adlandırma bilinçli (genel API yolu); yeniden adlandırma Faz 1/2.
pub mod terminal_tool;
#[allow(clippy::module_inception)]
pub mod tools;
pub mod user_terminal;
pub mod workspace_tool;

pub use terminal_tool::TerminalAgentTool;
pub use tool_invocation::ToolInvocation;
pub use tools::{AgentTool, CallVerdict, RiskLevel};
pub use workspace_tool::{WorkspaceAgentTool, WorkspaceToolKind};
