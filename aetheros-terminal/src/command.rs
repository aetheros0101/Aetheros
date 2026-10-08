use std::path::{Path, PathBuf};

use crate::{Environment, ExecutionLimits, TerminalError};

#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub cwd: Option<PathBuf>,
    pub environment: Environment,
    pub limits: ExecutionLimits,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Result<Self, TerminalError> {
        let program = program.into();
        if program.trim().is_empty() {
            return Err(TerminalError::InvalidCommand("program is empty".into()));
        }
        Ok(Self {
            program,
            args: Vec::new(),
            cwd: None,
            environment: Environment::default(),
            limits: ExecutionLimits::default(),
        })
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

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }

    pub fn env(mut self, environment: Environment) -> Self {
        self.environment = environment;
        self
    }

    pub fn limits(mut self, limits: ExecutionLimits) -> Self {
        self.limits = limits;
        self
    }

    pub(crate) fn validate(&self) -> Result<(), TerminalError> {
        if let Some(cwd) = &self.cwd {
            if !Path::new(cwd).is_dir() {
                return Err(TerminalError::InvalidWorkingDirectory(cwd.clone()));
            }
        }
        Ok(())
    }
}

pub struct CommandBuilder(CommandSpec);

impl CommandBuilder {
    pub fn new(program: impl Into<String>) -> Result<Self, TerminalError> {
        Ok(Self(CommandSpec::new(program)?))
    }

    pub fn arg(mut self, arg: impl Into<String>) -> Self {
        self.0 = self.0.arg(arg);
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.0 = self.0.args(args);
        self
    }

    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.0 = self.0.cwd(path);
        self
    }

    pub fn env(mut self, environment: Environment) -> Self {
        self.0 = self.0.env(environment);
        self
    }

    pub fn limits(mut self, limits: ExecutionLimits) -> Self {
        self.0 = self.0.limits(limits);
        self
    }

    pub fn build(self) -> CommandSpec {
        self.0
    }
}
