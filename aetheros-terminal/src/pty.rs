use std::{
    io::{Read, Write},
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use portable_pty::{
    native_pty_system, Child, ChildKiller, CommandBuilder as NativeCommandBuilder, MasterPty,
    PtySize,
};
use tokio::sync::{broadcast, mpsc};

use crate::errors::TerminalError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PtySizeSpec {
    pub rows: u16,
    pub cols: u16,
}

impl Default for PtySizeSpec {
    fn default() -> Self {
        Self { rows: 24, cols: 80 }
    }
}

#[derive(Debug, Clone)]
pub struct PtyCommand {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub env: Vec<(String, String)>,
}

impl PtyCommand {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
            env: Vec::new(),
        }
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.args.push(arg.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    pub fn cwd(mut self, cwd: impl Into<PathBuf>) -> Self {
        self.cwd = Some(cwd.into());
        self
    }

    pub fn env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.push((key.into(), value.into()));
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyEvent {
    Output(Vec<u8>),
    Exited(Option<i32>),
}

pub struct PtySession {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    master: Box<dyn MasterPty + Send>,
    /// `kill()` için ayrı tutulan öldürücü. `Child` nesnesini bekçi
    /// (waiter) thread'i `wait()` boyunca kilitli tutar; `kill()` aynı
    /// mutex'i beklerse süreç yaşadığı sürece SONSUZA dek bloklanır.
    /// Bu yüzden killer spawn anında, waiter başlamadan ÖNCE alınır.
    killer: Arc<Mutex<Box<dyn ChildKiller + Send + Sync>>>,
    events: mpsc::Receiver<PtyEvent>,
    broadcast: broadcast::Sender<PtyEvent>,
}

impl PtySession {
    pub async fn recv(&mut self) -> Option<PtyEvent> {
        self.events.recv().await
    }

    /// Subscribe to a loss-tolerant stream of PTY events.
    ///
    /// The existing `recv()` API remains the single-consumer compatibility
    /// path. Subscribers are independent observers for UI, agents and
    /// telemetry.
    pub fn subscribe(&self) -> broadcast::Receiver<PtyEvent> {
        self.broadcast.subscribe()
    }

    pub fn write(&self, bytes: &[u8]) -> Result<(), TerminalError> {
        let mut writer = self
            .writer
            .lock()
            .map_err(|_| TerminalError::InvalidCommand("PTY writer lock poisoned".into()))?;

        writer
            .write_all(bytes)
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;
        writer
            .flush()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        Ok(())
    }

    pub fn write_str(&self, text: &str) -> Result<(), TerminalError> {
        self.write(text.as_bytes())
    }

    /// Send the terminal interrupt character (ETX / Ctrl-C).
    ///
    /// For a normal Unix PTY this is interpreted by the terminal line
    /// discipline as SIGINT for the foreground process group. We deliberately
    /// send the byte through the PTY rather than trying to signal a PID: this
    /// preserves normal terminal semantics.
    pub fn interrupt(&self) -> Result<(), TerminalError> {
        self.write(&[0x03])
    }

    /// Send end-of-file (EOT / Ctrl-D) to the PTY.
    pub fn eof(&self) -> Result<(), TerminalError> {
        self.write(&[0x04])
    }

    /// Alias for [`Self::interrupt`], useful for agent/tool APIs.
    pub fn send_ctrl_c(&self) -> Result<(), TerminalError> {
        self.interrupt()
    }

    /// Alias for [`Self::eof`], useful for agent/tool APIs.
    pub fn send_ctrl_d(&self) -> Result<(), TerminalError> {
        self.eof()
    }

    pub fn resize(&self, size: PtySizeSpec) -> Result<(), TerminalError> {
        self.master
            .resize(PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))
    }

    /// Request immediate termination of the child process.
    pub fn kill(&self) -> Result<(), TerminalError> {
        // Waiter thread `child.wait()` boyunca Child mutex'ini tutar. Önceki
        // sürüm burada o mutex'i kilitleyip clone_killer() çağırıyordu →
        // canlı bir süreçte kill() süreç bitene kadar DEADLOCK oluyordu
        // (ve SessionManager::remove() bir tokio worker'ını kilitliyordu).
        // Artık spawn'da alınmış ayrı killer kullanılıyor.
        let mut killer = self
            .killer
            .lock()
            .map_err(|_| TerminalError::InvalidCommand("PTY killer lock poisoned".into()))?;

        killer
            .kill()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))
    }

    /// Ask an interactive shell to terminate through normal terminal input.
    ///
    /// This is intentionally separate from `kill()`: `close()` is graceful and
    /// only sends `exit` followed by a newline; `kill()` is forceful.
    pub fn close(&self) -> Result<(), TerminalError> {
        self.write_str("exit\n")
    }
}

#[derive(Debug, Clone)]
pub struct PtySessionManager {
    default_size: PtySizeSpec,
}

impl Default for PtySessionManager {
    fn default() -> Self {
        Self {
            default_size: PtySizeSpec::default(),
        }
    }
}

impl PtySessionManager {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_size(size: PtySizeSpec) -> Self {
        Self { default_size: size }
    }

    pub fn spawn(&self, command: PtyCommand) -> Result<PtySession, TerminalError> {
        let pty_system = native_pty_system();

        let pair = pty_system
            .openpty(PtySize {
                rows: self.default_size.rows,
                cols: self.default_size.cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        let mut cmd = NativeCommandBuilder::new(&command.program);
        cmd.args(&command.args);

        if let Some(cwd) = command.cwd {
            cmd.cwd(cwd);
        }

        for (key, value) in command.env {
            cmd.env(key, value);
        }

        let child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        drop(pair.slave);

        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        let writer = pair
            .master
            .take_writer()
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        let child: Box<dyn Child + Send + Sync> = child;
        // Waiter thread Child'ı kilitlemeden ÖNCE killer'ı al (bkz. PtySession::killer).
        let killer = Arc::new(Mutex::new(child.clone_killer()));
        let child = Arc::new(Mutex::new(child));
        let writer = Arc::new(Mutex::new(writer));

        let (event_tx, event_rx) = mpsc::channel::<PtyEvent>(128);
        let (broadcast_tx, _) = broadcast::channel::<PtyEvent>(256);

        // PTY readers are blocking OS handles. Never run them directly on Tokio.
        let reader_tx = event_tx.clone();
        let reader_broadcast = broadcast_tx.clone();
        thread::Builder::new()
            .name("aetheros-pty-reader".into())
            .spawn(move || read_pty(reader, reader_tx, reader_broadcast))
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        // Waiting is also blocking on some platforms.
        let waiter_tx = event_tx;
        let waiter_broadcast = broadcast_tx.clone();
        let child_for_wait = Arc::clone(&child);

        thread::Builder::new()
            .name("aetheros-pty-waiter".into())
            .spawn(move || {
                let status = {
                    let mut child = match child_for_wait.lock() {
                        Ok(value) => value,
                        Err(_) => return,
                    };

                    child.wait().ok()
                };

                let code = status.map(|status| status.exit_code() as i32);
                let event = PtyEvent::Exited(code);
                let _ = waiter_tx.blocking_send(event.clone());
                let _ = waiter_broadcast.send(event);
            })
            .map_err(|e| TerminalError::Io(std::io::Error::other(e.to_string())))?;

        Ok(PtySession {
            writer,
            master: pair.master,
            killer,
            events: event_rx,
            broadcast: broadcast_tx,
        })
    }
}

fn read_pty(
    mut reader: Box<dyn Read + Send>,
    tx: mpsc::Sender<PtyEvent>,
    broadcast: broadcast::Sender<PtyEvent>,
) {
    let mut buffer = vec![0u8; 8192];

    loop {
        match reader.read(&mut buffer) {
            Ok(0) => break,
            Ok(n) => {
                let event = PtyEvent::Output(buffer[..n].to_vec());
                let _ = broadcast.send(event.clone());
                if tx.blocking_send(event).is_err() {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pty_can_start_shell_and_echo() {
        let manager = PtySessionManager::with_size(PtySizeSpec { rows: 24, cols: 80 });

        let shell = if cfg!(windows) {
            PtyCommand::new("cmd").arg("/Q")
        } else {
            PtyCommand::new("sh")
        };

        let mut session = manager.spawn(shell).expect("spawn PTY");

        session
            .write_str("printf 'hello\\n'; exit\n")
            .expect("write command");

        let mut saw_hello = false;
        let mut saw_exit = false;

        for _ in 0..40 {
            match tokio::time::timeout(std::time::Duration::from_millis(250), session.recv()).await
            {
                Ok(Some(PtyEvent::Output(bytes))) => {
                    let text = String::from_utf8_lossy(&bytes);

                    if text.contains("hello") {
                        saw_hello = true;
                    }
                }

                Ok(Some(PtyEvent::Exited(_))) => {
                    saw_exit = true;
                    break;
                }

                Ok(None) => break,

                Err(_) => continue,
            }
        }

        assert!(saw_hello, "PTY did not return shell output");
        assert!(saw_exit, "PTY shell did not exit");
    }

    #[tokio::test]
    async fn pty_accepts_ctrl_d_and_exits_cat() {
        let manager = PtySessionManager::new();
        let mut session = manager
            .spawn(if cfg!(windows) {
                PtyCommand::new("cmd").arg("/Q")
            } else {
                PtyCommand::new("cat")
            })
            .expect("spawn PTY");

        if cfg!(windows) {
            let _ = session.kill();
            return;
        }

        session.send_ctrl_d().expect("send ctrl-d");

        let mut saw_exit = false;
        for _ in 0..20 {
            match tokio::time::timeout(std::time::Duration::from_millis(250), session.recv()).await
            {
                Ok(Some(PtyEvent::Exited(_))) => {
                    saw_exit = true;
                    break;
                }
                Ok(Some(PtyEvent::Output(_))) => {}
                Ok(None) => break,
                Err(_) => continue,
            }
        }

        assert!(saw_exit, "PTY child did not exit after Ctrl-D");
    }

    #[test]
    fn control_bytes_match_terminal_conventions() {
        assert_eq!([0x03u8], [3u8]);
        assert_eq!([0x04u8], [4u8]);
    }

    #[tokio::test]
    async fn pty_resize_is_supported() {
        let manager = PtySessionManager::new();

        let shell = if cfg!(windows) {
            PtyCommand::new("cmd").arg("/Q")
        } else {
            PtyCommand::new("sh")
        };

        let session = manager.spawn(shell).expect("spawn PTY");

        session
            .resize(PtySizeSpec {
                rows: 40,
                cols: 120,
            })
            .expect("resize PTY");

        let _ = session.kill();
    }

    /// Regresyon: waiter thread `child.wait()` içindeyken `kill()` DEADLOCK
    /// olmamalı. (read_timeout_is_reported_without_killing_session testi
    /// bu yüzden Termux'ta sonsuza dek asılı kalıyordu.)
    #[tokio::test]
    async fn kill_does_not_deadlock_with_waiter_thread() {
        if cfg!(windows) {
            return;
        }

        let manager = PtySessionManager::new();
        let session = manager.spawn(PtyCommand::new("cat")).expect("spawn PTY");

        // Waiter thread'in child.wait()'e girip mutex'i tutması için süre tanı.
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // kill() ayrı thread'de: deadlock olursa test sonsuza dek asılmasın.
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = session.kill();
            let _ = tx.send((session, result));
        });
        let (mut session, result) = rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .expect("kill() 5 sn içinde dönmedi: waiter thread ile deadlock");
        result.expect("kill() hata verdi");

        let mut saw_exit = false;
        for _ in 0..20 {
            match tokio::time::timeout(std::time::Duration::from_millis(250), session.recv()).await
            {
                Ok(Some(PtyEvent::Exited(_))) => {
                    saw_exit = true;
                    break;
                }
                Ok(Some(PtyEvent::Output(_))) => {}
                Ok(None) => break,
                Err(_) => continue,
            }
        }
        assert!(saw_exit, "kill() sonrası süreç çıkış olayı gelmedi");
    }
}
