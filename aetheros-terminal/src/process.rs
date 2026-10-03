use std::process::ExitStatus as StdExitStatus;
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExitStatus {
    code: Option<i32>,
    success: bool,
}

impl ExitStatus {
    pub(crate) fn from_std(status: StdExitStatus) -> Self {
        Self { code: status.code(), success: status.success() }
    }

    pub fn code(self) -> Option<i32> { self.code }
    pub fn success(self) -> bool { self.success }
}

#[derive(Debug, Clone)]
pub struct ProcessOutput {
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
}

impl ProcessOutput {
    pub fn stdout_string(&self) -> String { String::from_utf8_lossy(&self.stdout).into_owned() }
    pub fn stderr_string(&self) -> String { String::from_utf8_lossy(&self.stderr).into_owned() }
}

#[derive(Debug, Clone)]
pub struct ProcessResult {
    pub status: ExitStatus,
    pub output: ProcessOutput,
    pub duration: Duration,
}
