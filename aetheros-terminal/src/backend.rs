use std::time::Instant;

use tokio::io::AsyncReadExt;
use tokio::process::Command;

use crate::{CommandSpec, ProcessOutput, ProcessResult, TerminalError};

#[async_trait::async_trait]
pub trait ExecutionBackend: Send + Sync {
    async fn execute(&self, command: CommandSpec) -> Result<ProcessResult, TerminalError>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct LocalProcessBackend;

impl LocalProcessBackend {
    pub fn new() -> Self {
        Self
    }
}

async fn read_bounded<R>(mut reader: R, limit: usize) -> Result<Vec<u8>, TerminalError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut output = Vec::with_capacity(limit.min(8192));
    let mut chunk = [0u8; 8192];
    loop {
        let n = reader.read(&mut chunk).await.map_err(TerminalError::Io)?;
        if n == 0 {
            break;
        }
        if output.len().saturating_add(n) > limit {
            return Err(TerminalError::OutputLimitExceeded(limit));
        }
        output.extend_from_slice(&chunk[..n]);
    }
    Ok(output)
}

#[async_trait::async_trait]
impl ExecutionBackend for LocalProcessBackend {
    async fn execute(&self, spec: CommandSpec) -> Result<ProcessResult, TerminalError> {
        spec.validate()?;

        let mut command = Command::new(&spec.program);
        command.args(&spec.args);
        command.stdin(std::process::Stdio::null());
        command.stdout(std::process::Stdio::piped());
        command.stderr(std::process::Stdio::piped());
        if let Some(cwd) = &spec.cwd {
            command.current_dir(cwd);
        }
        spec.environment.apply(&mut command);

        let started = Instant::now();
        let mut child = command.spawn().map_err(TerminalError::Spawn)?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| TerminalError::Io(std::io::Error::other("stdout pipe unavailable")))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| TerminalError::Io(std::io::Error::other("stderr pipe unavailable")))?;

        let stdout_task = tokio::spawn(read_bounded(stdout, spec.limits.max_stdout_bytes));
        let stderr_task = tokio::spawn(read_bounded(stderr, spec.limits.max_stderr_bytes));

        let wait_result = match spec.limits.timeout {
            Some(timeout) => {
                tokio::select! {
                    result = child.wait() => result.map_err(TerminalError::Io),
                    _ = tokio::time::sleep(timeout) => {
                        let _ = child.kill().await;
                        let _ = child.wait().await;
                        return Err(TerminalError::Timeout(timeout));
                    }
                }
            }
            None => child.wait().await.map_err(TerminalError::Io),
        }?;

        let stdout = stdout_task
            .await
            .map_err(|e| TerminalError::Io(std::io::Error::other(e)))??;
        let stderr = stderr_task
            .await
            .map_err(|e| TerminalError::Io(std::io::Error::other(e)))??;

        Ok(ProcessResult {
            status: crate::ExitStatus::from_std(wait_result),
            output: ProcessOutput { stdout, stderr },
            duration: started.elapsed(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommandBuilder, ExecutionLimits};
    use std::time::Duration;

    #[tokio::test]
    async fn captures_stdout_and_stderr() {
        let backend = LocalProcessBackend::new();
        let command = if cfg!(windows) {
            CommandBuilder::new("cmd")
                .unwrap()
                .args(["/C", "echo hello"])
                .build()
        } else {
            CommandBuilder::new("sh")
                .unwrap()
                .args(["-c", "printf hello; printf error >&2"])
                .build()
        };
        let result = backend.execute(command).await.unwrap();
        assert!(result.status.success());
        assert_eq!(result.output.stdout_string(), "hello");
        assert_eq!(result.output.stderr_string(), "error");
    }

    #[tokio::test]
    async fn timeout_terminates_process() {
        let backend = LocalProcessBackend::new();
        let command = if cfg!(windows) {
            CommandBuilder::new("ping")
                .unwrap()
                .args(["127.0.0.1", "-n", "5"])
                .build()
        } else {
            CommandBuilder::new("sh")
                .unwrap()
                .args(["-c", "sleep 5"])
                .limits(ExecutionLimits::default().timeout(Duration::from_millis(50)))
                .build()
        };
        let result = backend.execute(command).await;
        if cfg!(windows) {
            assert!(result.is_ok());
        } else {
            assert!(matches!(result, Err(TerminalError::Timeout(_))));
        }
    }
}
