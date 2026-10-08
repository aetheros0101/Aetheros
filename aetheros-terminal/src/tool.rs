use std::path::PathBuf;

use tokio::sync::broadcast;

use crate::{
    CommandSpec, ProcessResult, PtyCommand, PtyEvent, PtySizeSpec, SessionId, TerminalError,
    TerminalEvent, TerminalSession, TerminalSessionManager,
};

/// Agent-facing command surface over the terminal session manager.
///
/// The tool deliberately exposes operations instead of the internal manager
/// state. This keeps an agent integration stable while the PTY/session
/// implementation evolves independently.
#[derive(Debug)]
pub enum TerminalToolRequest {
    Create {
        cwd: Option<PathBuf>,
    },
    Execute {
        session_id: SessionId,
        command: CommandSpec,
    },
    OpenPty {
        session_id: SessionId,
        command: PtyCommand,
    },
    Read {
        session_id: SessionId,
        timeout_ms: u64,
    },
    Write {
        session_id: SessionId,
        data: Vec<u8>,
    },
    Interrupt {
        session_id: SessionId,
    },
    Eof {
        session_id: SessionId,
    },
    Resize {
        session_id: SessionId,
        size: PtySizeSpec,
    },
    Kill {
        session_id: SessionId,
    },
    Close {
        session_id: SessionId,
    },
    Remove {
        session_id: SessionId,
    },
}

/// Agent-facing result values.
#[derive(Debug)]
pub enum TerminalToolResponse {
    SessionCreated(TerminalSession),
    Executed(ProcessResult),
    PtyOpened,
    Event(Option<PtyEvent>),
    Written,
    Interrupted,
    EofSent,
    Resized,
    Killed,
    Closed,
    SessionRemoved(Option<TerminalSession>),
}

/// Stable tool facade for Aetheros agents.
pub struct AgentTerminalTool<B = crate::LocalProcessBackend> {
    manager: TerminalSessionManager<B>,
}

impl AgentTerminalTool<crate::LocalProcessBackend> {
    pub fn local() -> Self {
        Self {
            manager: TerminalSessionManager::local(),
        }
    }
}

impl<B> AgentTerminalTool<B>
where
    B: crate::ExecutionBackend + 'static,
{
    pub fn new(manager: TerminalSessionManager<B>) -> Self {
        Self { manager }
    }

    pub fn manager(&self) -> &TerminalSessionManager<B> {
        &self.manager
    }

    /// Subscribe to terminal-wide observations. This is intentionally kept
    /// outside `TerminalToolRequest` because a broadcast receiver is a live
    /// capability, not a serializable tool result.
    pub fn subscribe(&self) -> broadcast::Receiver<TerminalEvent> {
        self.manager.subscribe()
    }

    pub async fn call(
        &self,
        request: TerminalToolRequest,
    ) -> Result<TerminalToolResponse, TerminalError> {
        match request {
            TerminalToolRequest::Create { cwd } => Ok(TerminalToolResponse::SessionCreated(
                self.manager.create(cwd),
            )),
            TerminalToolRequest::Execute {
                session_id,
                command,
            } => {
                let session = self.require_session(session_id)?;
                let result = self.manager.execute(&session, command).await?;
                Ok(TerminalToolResponse::Executed(result))
            }
            TerminalToolRequest::OpenPty {
                session_id,
                command,
            } => {
                let session = self.require_session(session_id)?;
                self.manager.open_pty(&session, command)?;
                Ok(TerminalToolResponse::PtyOpened)
            }
            TerminalToolRequest::Read {
                session_id,
                timeout_ms,
            } => {
                let event = tokio::time::timeout(
                    std::time::Duration::from_millis(timeout_ms),
                    self.manager.recv(session_id),
                )
                .await
                .map_err(|_| {
                    TerminalError::Timeout(std::time::Duration::from_millis(timeout_ms))
                })??;
                Ok(TerminalToolResponse::Event(event))
            }
            TerminalToolRequest::Write { session_id, data } => {
                self.manager.write(session_id, &data).await?;
                Ok(TerminalToolResponse::Written)
            }
            TerminalToolRequest::Interrupt { session_id } => {
                self.manager.interrupt(session_id).await?;
                Ok(TerminalToolResponse::Interrupted)
            }
            TerminalToolRequest::Eof { session_id } => {
                self.manager.eof(session_id).await?;
                Ok(TerminalToolResponse::EofSent)
            }
            TerminalToolRequest::Resize { session_id, size } => {
                self.manager.resize(session_id, size).await?;
                Ok(TerminalToolResponse::Resized)
            }
            TerminalToolRequest::Kill { session_id } => {
                self.manager.kill(session_id).await?;
                Ok(TerminalToolResponse::Killed)
            }
            TerminalToolRequest::Close { session_id } => {
                self.manager.close(session_id).await?;
                Ok(TerminalToolResponse::Closed)
            }
            TerminalToolRequest::Remove { session_id } => Ok(TerminalToolResponse::SessionRemoved(
                self.manager.remove(session_id),
            )),
        }
    }

    fn require_session(&self, id: SessionId) -> Result<TerminalSession, TerminalError> {
        self.manager.get(id).ok_or_else(|| {
            TerminalError::InvalidCommand(format!("unknown terminal session: {:?}", id))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandBuilder;
    use std::time::Duration;

    #[tokio::test]
    async fn creates_and_executes_through_agent_tool() {
        let tool = AgentTerminalTool::local();

        let response = tool
            .call(TerminalToolRequest::Create {
                cwd: Some(std::env::temp_dir()),
            })
            .await
            .unwrap();

        let session = match response {
            TerminalToolResponse::SessionCreated(session) => session,
            other => panic!("unexpected response: {other:?}"),
        };

        let command = if cfg!(windows) {
            CommandBuilder::new("cmd")
                .unwrap()
                .args(["/C", "hello"])
                .build()
        } else {
            CommandBuilder::new("echo").unwrap().arg("hello").build()
        };

        let response = tool
            .call(TerminalToolRequest::Execute {
                session_id: session.id,
                command,
            })
            .await
            .unwrap();

        match response {
            TerminalToolResponse::Executed(result) => {
                assert!(result.status.success());
                assert!(result.output.stdout_string().contains("hello"));
            }
            other => panic!("unexpected response: {other:?}"),
        }

        tool.call(TerminalToolRequest::Remove {
            session_id: session.id,
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn agent_tool_controls_interactive_pty() {
        let tool = AgentTerminalTool::local();
        let session = match tool
            .call(TerminalToolRequest::Create { cwd: None })
            .await
            .unwrap()
        {
            TerminalToolResponse::SessionCreated(session) => session,
            other => panic!("unexpected response: {other:?}"),
        };

        let shell = if cfg!(windows) {
            PtyCommand::new("cmd").arg("/Q")
        } else {
            PtyCommand::new("sh")
        };

        tool.call(TerminalToolRequest::OpenPty {
            session_id: session.id,
            command: shell,
        })
        .await
        .unwrap();

        tool.call(TerminalToolRequest::Write {
            session_id: session.id,
            data: b"printf 'tool-ok\\n'; exit\n".to_vec(),
        })
        .await
        .unwrap();

        let mut saw_output = false;
        let mut saw_exit = false;
        for _ in 0..40 {
            let response = tool
                .call(TerminalToolRequest::Read {
                    session_id: session.id,
                    timeout_ms: 250,
                })
                .await;

            match response {
                Ok(TerminalToolResponse::Event(Some(PtyEvent::Output(data)))) => {
                    if String::from_utf8_lossy(&data).contains("tool-ok") {
                        saw_output = true;
                    }
                }
                Ok(TerminalToolResponse::Event(Some(PtyEvent::Exited(_)))) => {
                    saw_exit = true;
                    break;
                }
                Err(TerminalError::Timeout(_)) => continue,
                Ok(TerminalToolResponse::Event(None)) => break,
                other => panic!("unexpected response: {other:?}"),
            }
        }

        assert!(saw_output, "agent tool did not observe PTY output");
        assert!(saw_exit, "agent tool did not observe PTY exit");

        tool.call(TerminalToolRequest::Remove {
            session_id: session.id,
        })
        .await
        .unwrap();
    }

    #[tokio::test]
    async fn unknown_session_is_rejected_by_tool() {
        let tool = AgentTerminalTool::local();
        let session = match tool
            .call(TerminalToolRequest::Create { cwd: None })
            .await
            .unwrap()
        {
            TerminalToolResponse::SessionCreated(session) => session,
            other => panic!("unexpected response: {other:?}"),
        };
        let unknown = session.id;
        tool.call(TerminalToolRequest::Remove {
            session_id: unknown,
        })
        .await
        .unwrap();

        let command = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "echo" })
            .unwrap()
            .arg("hello")
            .build();

        let result = tool
            .call(TerminalToolRequest::Execute {
                session_id: unknown,
                command,
            })
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn read_timeout_is_reported_without_killing_session() {
        let tool = AgentTerminalTool::local();

        let session = match tool
            .call(TerminalToolRequest::Create { cwd: None })
            .await
            .unwrap()
        {
            TerminalToolResponse::SessionCreated(session) => session,
            other => panic!("unexpected response: {other:?}"),
        };

        let shell = if cfg!(windows) {
            PtyCommand::new("cmd").arg("/Q")
        } else {
            PtyCommand::new("cat")
        };

        tool.call(TerminalToolRequest::OpenPty {
            session_id: session.id,
            command: shell,
        })
        .await
        .unwrap();

        let result = tool
            .call(TerminalToolRequest::Read {
                session_id: session.id,
                timeout_ms: 10,
            })
            .await;

        assert!(matches!(
            result,
            Err(TerminalError::Timeout(duration))
                if duration == Duration::from_millis(10)
        ));

        // Timeout must not destroy the interactive session.
        assert!(tool.manager().has_pty(session.id));

        tool.call(TerminalToolRequest::Kill {
            session_id: session.id,
        })
        .await
        .unwrap();

        tool.call(TerminalToolRequest::Remove {
            session_id: session.id,
        })
        .await
        .unwrap();
    }
}
