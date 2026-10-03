use std::time::Duration;

#[derive(Debug, Clone)]
pub struct ExecutionLimits {
    pub timeout: Option<Duration>,
    pub max_stdout_bytes: usize,
    pub max_stderr_bytes: usize,
}

impl Default for ExecutionLimits {
    fn default() -> Self {
        Self {
            timeout: Some(Duration::from_secs(120)),
            max_stdout_bytes: 4 * 1024 * 1024,
            max_stderr_bytes: 4 * 1024 * 1024,
        }
    }
}

impl ExecutionLimits {
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn no_timeout(mut self) -> Self {
        self.timeout = None;
        self
    }

    pub fn max_output(mut self, stdout: usize, stderr: usize) -> Self {
        self.max_stdout_bytes = stdout;
        self.max_stderr_bytes = stderr;
        self
    }
}
