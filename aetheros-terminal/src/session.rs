use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use tokio::sync::Mutex as AsyncMutex;

use crate::{
    CommandSpec, ExecutionBackend, LocalProcessBackend, ProcessResult, PtyCommand, PtyEvent,
    PtySession, PtySessionManager, PtySizeSpec, TerminalError, TerminalEvent, TerminalEventBus,
};

static NEXT_SESSION_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(u64);

impl SessionId {
    fn new() -> Self {
        Self(NEXT_SESSION_ID.fetch_add(1, Ordering::Relaxed))
    }
}

#[derive(Debug, Clone)]
pub struct TerminalSession {
    pub id: SessionId,
    pub cwd: Option<PathBuf>,
}

/// Owns terminal sessions and provides one stable control surface for agents.
///
/// A session can be used for one-shot process execution through `execute`, or
/// attached to an interactive PTY through `open_pty`.
pub struct TerminalSessionManager<B = LocalProcessBackend> {
    backend: Arc<B>,
    pty_manager: PtySessionManager,
    sessions: Arc<Mutex<HashMap<SessionId, TerminalSession>>>,
    ptys: Arc<Mutex<HashMap<SessionId, Arc<AsyncMutex<PtySession>>>>>,
    events: TerminalEventBus,
}

impl TerminalSessionManager<LocalProcessBackend> {
    pub fn local() -> Self {
        Self::new(LocalProcessBackend::new())
    }
}

impl<B> TerminalSessionManager<B>
where
    B: ExecutionBackend + 'static,
{
    pub fn new(backend: B) -> Self {
        Self {
            backend: Arc::new(backend),
            pty_manager: PtySessionManager::new(),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            ptys: Arc::new(Mutex::new(HashMap::new())),
            events: TerminalEventBus::new(),
        }
    }

    pub fn with_pty_size(backend: B, size: PtySizeSpec) -> Self {
        Self {
            backend: Arc::new(backend),
            pty_manager: PtySessionManager::with_size(size),
            sessions: Arc::new(Mutex::new(HashMap::new())),
            ptys: Arc::new(Mutex::new(HashMap::new())),
            events: TerminalEventBus::new(),
        }
    }

    pub fn create(&self, cwd: Option<PathBuf>) -> TerminalSession {
        let session = TerminalSession {
            id: SessionId::new(),
            cwd,
        };
        self.sessions
            .lock()
            .expect("session mutex poisoned")
            .insert(session.id, session.clone());
        self.events.publish(TerminalEvent::SessionCreated {
            session_id: session.id,
        });
        session
    }

    pub fn remove(&self, id: SessionId) -> Option<TerminalSession> {
        if let Some(pty) = self
            .ptys
            .lock()
            .expect("pty map mutex poisoned")
            .remove(&id)
        {
            if let Ok(pty) = pty.try_lock() {
                let _ = pty.kill();
            }
            self.events
                .publish(TerminalEvent::PtyDetached { session_id: id });
        }

        let removed = self
            .sessions
            .lock()
            .expect("session mutex poisoned")
            .remove(&id);

        if removed.is_some() {
            self.events
                .publish(TerminalEvent::SessionRemoved { session_id: id });
        }

        removed
    }

    pub fn get(&self, id: SessionId) -> Option<TerminalSession> {
        self.sessions
            .lock()
            .expect("session mutex poisoned")
            .get(&id)
            .cloned()
    }

    pub fn has_pty(&self, id: SessionId) -> bool {
        self.ptys
            .lock()
            .expect("pty map mutex poisoned")
            .contains_key(&id)
    }

    /// Subscribe to the terminal-wide event stream.
    ///
    /// Events are broadcast to all current subscribers; new subscribers only
    /// receive events published after subscription.
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<TerminalEvent> {
        self.events.subscribe()
    }

    pub async fn execute(
        &self,
        session: &TerminalSession,
        mut command: CommandSpec,
    ) -> Result<ProcessResult, TerminalError> {
        self.ensure_session(session.id)?;

        if command.cwd.is_none() {
            command.cwd = session.cwd.clone();
        }

        self.backend.execute(command).await
    }

    /// Attach an interactive PTY to an existing terminal session.
    pub fn open_pty(
        &self,
        session: &TerminalSession,
        mut command: PtyCommand,
    ) -> Result<(), TerminalError> {
        self.ensure_session(session.id)?;

        if command.cwd.is_none() {
            command.cwd = session.cwd.clone();
        }

        // Check ownership before spawning a process. A rejected second PTY must
        // not create a stray shell/process as a side effect.
        {
            let ptys = self.ptys.lock().expect("pty map mutex poisoned");
            if ptys.contains_key(&session.id) {
                return Err(TerminalError::InvalidCommand(
                    "session already has an interactive PTY".into(),
                ));
            }
        }

        let pty = self.pty_manager.spawn(command)?;
        let handle = Arc::new(AsyncMutex::new(pty));

        let pty_events = handle
            .try_lock()
            .map_err(|_| {
                TerminalError::InvalidCommand("PTY lock unavailable during attach".into())
            })?
            .subscribe();

        let mut ptys = self.ptys.lock().expect("pty map mutex poisoned");
        // Defensive second check closes the race between the pre-check and the
        // insertion. If another caller won, terminate the just-created PTY.
        if ptys.contains_key(&session.id) {
            drop(ptys);
            if let Ok(pty) = handle.try_lock() {
                let _ = pty.kill();
            }
            return Err(TerminalError::InvalidCommand(
                "session already has an interactive PTY".into(),
            ));
        }

        ptys.insert(session.id, handle);
        drop(ptys);

        let bus = self.events.clone();
        let session_id = session.id;
        tokio::spawn(async move {
            let mut events = pty_events;
            loop {
                match events.recv().await {
                    Ok(event) => bus.publish(TerminalEvent::from_pty(session_id, event)),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });

        self.events
            .publish(TerminalEvent::PtyAttached { session_id });
        Ok(())
    }

    /// Receive the next event from an interactive session.
    pub async fn recv(&self, id: SessionId) -> Result<Option<PtyEvent>, TerminalError> {
        let pty = self.pty_handle(id)?;
        let mut pty = pty.lock().await;
        Ok(pty.recv().await)
    }

    pub async fn write(&self, id: SessionId, bytes: &[u8]) -> Result<(), TerminalError> {
        let pty = self.pty_handle(id)?;
        let pty = pty.lock().await;
        pty.write(bytes)
    }

    pub async fn write_str(&self, id: SessionId, text: &str) -> Result<(), TerminalError> {
        self.write(id, text.as_bytes()).await
    }

    pub async fn interrupt(&self, id: SessionId) -> Result<(), TerminalError> {
        let pty = self.pty_handle(id)?;
        let pty = pty.lock().await;
        pty.interrupt()
    }

    pub async fn eof(&self, id: SessionId) -> Result<(), TerminalError> {
        let pty = self.pty_handle(id)?;
        let pty = pty.lock().await;
        pty.eof()
    }

    pub async fn resize(&self, id: SessionId, size: PtySizeSpec) -> Result<(), TerminalError> {
        let pty = self.pty_handle(id)?;
        let pty = pty.lock().await;
        pty.resize(size)
    }

    pub async fn kill(&self, id: SessionId) -> Result<(), TerminalError> {
        let pty = self.pty_handle(id)?;
        let pty = pty.lock().await;
        pty.kill()
    }

    pub async fn close(&self, id: SessionId) -> Result<(), TerminalError> {
        let pty = self.pty_handle(id)?;
        let pty = pty.lock().await;
        pty.close()
    }

    /// Detach the PTY from the manager without killing the underlying process.
    ///
    /// This is useful when a higher layer wants to transfer ownership. Normal
    /// agent workflows should prefer `kill` or `close` followed by `remove`.
    pub fn detach_pty(&self, id: SessionId) -> bool {
        let detached = self
            .ptys
            .lock()
            .expect("pty map mutex poisoned")
            .remove(&id)
            .is_some();

        if detached {
            self.events
                .publish(TerminalEvent::PtyDetached { session_id: id });
        }

        detached
    }

    fn ensure_session(&self, id: SessionId) -> Result<(), TerminalError> {
        if self
            .sessions
            .lock()
            .expect("session mutex poisoned")
            .contains_key(&id)
        {
            Ok(())
        } else {
            Err(TerminalError::InvalidCommand(format!(
                "unknown terminal session: {:?}",
                id
            )))
        }
    }

    fn pty_handle(&self, id: SessionId) -> Result<Arc<AsyncMutex<PtySession>>, TerminalError> {
        self.ensure_session(id)?;

        self.ptys
            .lock()
            .expect("pty map mutex poisoned")
            .get(&id)
            .cloned()
            .ok_or_else(|| {
                TerminalError::InvalidCommand(format!("session {:?} has no interactive PTY", id))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CommandBuilder;
    use std::time::Duration;

    #[tokio::test]
    async fn executes_command_in_session() {
        let manager = TerminalSessionManager::local();
        let session = manager.create(Some(std::env::temp_dir()));

        let command = if cfg!(windows) {
            CommandBuilder::new("cmd")
                .unwrap()
                .args(vec!["/C".to_string(), "hello".to_string()])
                .build()
        } else {
            CommandBuilder::new("echo").unwrap().arg("hello").build()
        };

        let result = manager.execute(&session, command).await.unwrap();
        assert!(result.status.success());
        assert!(result.output.stdout_string().contains("hello"));
    }

    #[tokio::test]
    async fn manages_interactive_pty_lifecycle() {
        let manager = TerminalSessionManager::local();
        let session = manager.create(Some(std::env::temp_dir()));

        let shell = if cfg!(windows) {
            PtyCommand::new("cmd").arg("/Q")
        } else {
            PtyCommand::new("sh")
        };

        manager.open_pty(&session, shell).unwrap();
        assert!(manager.has_pty(session.id));

        manager
            .write_str(session.id, "printf 'manager-ok\\n'; exit\n")
            .await
            .unwrap();

        let mut saw_output = false;
        let mut saw_exit = false;

        for _ in 0..40 {
            match tokio::time::timeout(Duration::from_millis(250), manager.recv(session.id)).await {
                Ok(Ok(Some(PtyEvent::Output(bytes)))) => {
                    if String::from_utf8_lossy(&bytes).contains("manager-ok") {
                        saw_output = true;
                    }
                }
                Ok(Ok(Some(PtyEvent::Exited(_)))) => {
                    saw_exit = true;
                    break;
                }
                Ok(Ok(None)) => break,
                Ok(Err(error)) => panic!("unexpected PTY error: {error}"),
                Err(_) => continue,
            }
        }

        assert!(saw_output);
        assert!(saw_exit);

        let removed = manager.remove(session.id);
        assert!(removed.is_some());
        assert!(!manager.has_pty(session.id));
    }

    #[tokio::test]
    async fn rejects_second_pty_for_same_session() {
        let manager = TerminalSessionManager::local();
        let session = manager.create(None);

        let shell = if cfg!(windows) {
            PtyCommand::new("cmd").arg("/Q")
        } else {
            PtyCommand::new("sh")
        };

        manager.open_pty(&session, shell.clone()).unwrap();
        let second = manager.open_pty(&session, shell);
        assert!(second.is_err());

        manager.kill(session.id).await.unwrap();
        manager.remove(session.id);
    }

    #[tokio::test]
    async fn event_bus_observes_session_and_pty_events() {
        let manager = TerminalSessionManager::local();
        let mut events = manager.subscribe();
        let session = manager.create(Some(std::env::temp_dir()));

        match events.recv().await.unwrap() {
            TerminalEvent::SessionCreated { session_id } => assert_eq!(session_id, session.id),
            other => panic!("unexpected event: {other:?}"),
        }

        manager
            .open_pty(
                &session,
                if cfg!(windows) {
                    PtyCommand::new("cmd").arg("/Q")
                } else {
                    PtyCommand::new("sh")
                },
            )
            .unwrap();

        match events.recv().await.unwrap() {
            TerminalEvent::PtyAttached { session_id } => assert_eq!(session_id, session.id),
            other => panic!("unexpected event: {other:?}"),
        }

        manager
            .write_str(session.id, "printf 'event-ok\n'; exit\n")
            .await
            .unwrap();

        let mut saw_output = false;
        let mut saw_exit = false;
        for _ in 0..40 {
            match tokio::time::timeout(Duration::from_millis(250), events.recv()).await {
                Ok(Ok(TerminalEvent::PtyOutput { session_id, data })) => {
                    assert_eq!(session_id, session.id);
                    if String::from_utf8_lossy(&data).contains("event-ok") {
                        saw_output = true;
                    }
                }
                Ok(Ok(TerminalEvent::PtyExited { session_id, .. })) => {
                    assert_eq!(session_id, session.id);
                    saw_exit = true;
                    break;
                }
                Ok(Ok(_)) => {}
                Ok(Err(error)) => panic!("event bus error: {error}"),
                Err(_) => continue,
            }
        }

        assert!(saw_output, "event bus did not observe PTY output");
        assert!(saw_exit, "event bus did not observe PTY exit");

        manager.remove(session.id);
        match events.recv().await.unwrap() {
            TerminalEvent::PtyDetached { session_id } => assert_eq!(session_id, session.id),
            other => panic!("unexpected event: {other:?}"),
        }
        match events.recv().await.unwrap() {
            TerminalEvent::SessionRemoved { session_id } => assert_eq!(session_id, session.id),
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn unknown_session_is_rejected() {
        let manager = TerminalSessionManager::local();
        let session = TerminalSession {
            id: SessionId(9_999_999),
            cwd: None,
        };

        let command = CommandBuilder::new(if cfg!(windows) { "cmd" } else { "echo" })
            .unwrap()
            .arg("hello")
            .build();

        assert!(manager.execute(&session, command).await.is_err());
        assert!(manager.write_str(session.id, "hello").await.is_err());
    }
}
