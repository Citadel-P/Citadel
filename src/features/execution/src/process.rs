use futures_util::future::BoxFuture;
use std::{ffi::OsString, path::PathBuf, time::Duration};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

/// Hard limits applied to every child process owned by Citadel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProcessLimits {
    pub timeout: Duration,
    pub maximum_stdout_bytes: usize,
    pub maximum_stderr_bytes: usize,
    pub output_limit_policy: OutputLimitPolicy,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutputLimitPolicy {
    #[default]
    Error,
    Truncate,
}

impl Default for ProcessLimits {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(30),
            maximum_stdout_bytes: 64 * 1024,
            maximum_stderr_bytes: 16 * 1024,
            output_limit_policy: OutputLimitPolicy::Error,
        }
    }
}

/// A process invocation. Arguments are passed directly; no shell is involved.
///
/// `Debug` is intentionally not implemented because arguments, environment
/// values, or stdin may contain credentials.
pub struct ProcessRequest {
    pub output: Option<mpsc::Sender<ProcessChunk>>,
    pub program: OsString,
    pub arguments: Vec<OsString>,
    pub current_directory: Option<PathBuf>,
    pub environment: Vec<(OsString, OsString)>,
    pub stdin: Option<Vec<u8>>,
    pub limits: ProcessLimits,
}

impl ProcessRequest {
    #[must_use]
    pub fn new(program: impl Into<OsString>) -> Self {
        Self {
            output: None,
            program: program.into(),
            arguments: Vec::new(),
            current_directory: None,
            environment: Vec::new(),
            stdin: None,
            limits: ProcessLimits::default(),
        }
    }

    #[must_use]
    pub fn args<I, S>(mut self, arguments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<OsString>,
    {
        self.arguments = arguments.into_iter().map(Into::into).collect();
        self
    }

    #[must_use]
    pub fn current_dir(mut self, directory: impl Into<PathBuf>) -> Self {
        self.current_directory = Some(directory.into());
        self
    }

    #[must_use]
    pub fn env(mut self, key: impl Into<OsString>, value: impl Into<OsString>) -> Self {
        self.environment.push((key.into(), value.into()));
        self
    }

    #[must_use]
    pub fn stdin(mut self, stdin: Vec<u8>) -> Self {
        self.stdin = Some(stdin);
        self
    }

    #[must_use]
    pub fn limits(mut self, limits: ProcessLimits) -> Self {
        self.limits = limits;
        self
    }

    #[must_use]
    pub fn output(mut self, sender: mpsc::Sender<ProcessChunk>) -> Self {
        self.output = Some(sender);
        self
    }
}

/// Raw output: callers must redact before logging or persisting it.
pub struct ProcessChunk {
    pub stream: &'static str,
    pub bytes: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcessOutput {
    pub exit_code: Option<i32>,
    pub stdout: Vec<u8>,
    pub stderr: Vec<u8>,
    pub stdout_truncated: bool,
    pub stderr_truncated: bool,
}

impl ProcessOutput {
    #[must_use]
    pub const fn succeeded(&self) -> bool {
        matches!(self.exit_code, Some(0))
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("process could not be started: {0}")]
    Spawn(#[source] std::io::Error),
    #[error("process I/O failed: {0}")]
    Io(#[source] std::io::Error),
    #[error("process exceeded its {stream} output limit of {limit} bytes")]
    OutputLimit { stream: &'static str, limit: usize },
    #[error("process timed out after {0:?}")]
    Timeout(Duration),
    #[error("process was cancelled")]
    Cancelled,
}

/// Executes a bounded process request through an injected runtime implementation.
pub trait ProcessRunner: Send + Sync {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>>;
}
