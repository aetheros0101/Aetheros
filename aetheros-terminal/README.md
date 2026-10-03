# Aetheros Terminal v0.6.0

Standalone process, PTY, session, event-bus and agent-tool execution layer for Aetheros.

## v0.6 — Agent Terminal Tool

v0.6 adds `AgentTerminalTool`, a stable agent-facing facade over `TerminalSessionManager`.

The intended layering is:

```text
Aetheros Agent
      |
      v
AgentTerminalTool
      |
      v
TerminalSessionManager
      |
      +-- Session
      +-- PTY
      +-- Event Bus
      +-- Process Backend
```

### Operations

`TerminalToolRequest` supports:

- `Create`
- `Execute`
- `OpenPty`
- `Read`
- `Write`
- `Interrupt`
- `Eof`
- `Resize`
- `Kill`
- `Close`
- `Remove`

`AgentTerminalTool::subscribe()` exposes the existing broadcast event stream for UI, telemetry and other observers.

## Safety / lifecycle properties

- PTY ownership is checked before spawning a new process.
- A second PTY for one session is rejected without spawning a stray process.
- `Read` has an explicit timeout and does not kill the session on timeout.
- The tool facade does not expose internal mutexes or PTY implementation details.
- The crate remains standalone; no changes to the Aetheros `src/` tree are required.

## Verify on Termux

```bash
cargo clean
cargo test
```

The development environment used to prepare this archive did not have Cargo installed, so the package must be compiled and tested in the Termux environment before being considered verified.
