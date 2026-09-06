#![forbid(unsafe_code)]

mod redaction;
pub use redaction::SecretRedactor;

use std::ffi::OsString;
use std::path::PathBuf;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const READ_BUFFER_BYTES: usize = 8 * 1024;

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

#[derive(Debug, Clone, Copy)]
enum Completion {
    Exited(ExitStatus),
    OutputLimit(&'static str, usize),
    Timeout,
    Cancelled,
}

/// Runs one child process with bounded output, timeout, and cooperative
/// cancellation. On every non-exit completion path the child is killed and
/// reaped before this method returns.
pub async fn run(
    request: ProcessRequest,
    cancellation: &CancellationToken,
) -> Result<ProcessOutput, ProcessError> {
    validate_limits(request.limits)?;
    if cancellation.is_cancelled() {
        return Err(ProcessError::Cancelled);
    }

    let mut command = Command::new(&request.program);
    command
        .args(&request.arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    if request.stdin.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    if let Some(directory) = &request.current_directory {
        command.current_dir(directory);
    }
    for (key, value) in &request.environment {
        command.env(key, value);
    }

    let mut child = command.spawn().map_err(ProcessError::Spawn)?;
    let stdout = child.stdout.take().expect("piped stdout is available");
    let stderr = child.stderr.take().expect("piped stderr is available");
    let (limit_sender, mut limit_receiver) = mpsc::channel(1);
    let output_cancellation = cancellation.child_token();
    let stdout_reader = tokio::spawn(read_bounded(
        stdout,
        request.limits.maximum_stdout_bytes,
        "stdout",
        request.limits.output_limit_policy,
        limit_sender.clone(),
        request.output.clone(),
        output_cancellation.clone(),
    ));
    let stderr_reader = tokio::spawn(read_bounded(
        stderr,
        request.limits.maximum_stderr_bytes,
        "stderr",
        request.limits.output_limit_policy,
        limit_sender,
        request.output,
        output_cancellation.clone(),
    ));
    let stdin_writer = request.stdin.map(|input| {
        let mut stdin = child.stdin.take().expect("piped stdin is available");
        tokio::spawn(async move {
            stdin.write_all(&input).await?;
            stdin.shutdown().await
        })
    });

    let timeout = tokio::time::sleep(request.limits.timeout);
    tokio::pin!(timeout);
    let completion = tokio::select! {
        biased;
        () = cancellation.cancelled() => Completion::Cancelled,
        Some((stream, limit)) = limit_receiver.recv() => Completion::OutputLimit(stream, limit),
        () = &mut timeout => Completion::Timeout,
        result = child.wait() => {
            Completion::Exited(result.map_err(ProcessError::Io)?)
        }
    };

    if !matches!(completion, Completion::Exited(_)) {
        output_cancellation.cancel();
        let _ = child.start_kill();
        child.wait().await.map_err(ProcessError::Io)?;
    }

    if let Some(writer) = stdin_writer {
        match writer.await {
            Ok(Ok(())) => {}
            // A killed or early-exiting child commonly closes stdin first.
            Ok(Err(_)) if !matches!(completion, Completion::Exited(_)) => {}
            Ok(Err(error)) => return Err(ProcessError::Io(error)),
            Err(error) => {
                return Err(ProcessError::Io(std::io::Error::other(error.to_string())));
            }
        }
    }
    let (stdout, stdout_truncated) = join_reader(stdout_reader).await?;
    let (stderr, stderr_truncated) = join_reader(stderr_reader).await?;

    match completion {
        Completion::Exited(status) => Ok(ProcessOutput {
            exit_code: status.code(),
            stdout,
            stderr,
            stdout_truncated,
            stderr_truncated,
        }),
        Completion::OutputLimit(stream, limit) => Err(ProcessError::OutputLimit { stream, limit }),
        Completion::Timeout => Err(ProcessError::Timeout(request.limits.timeout)),
        Completion::Cancelled => Err(ProcessError::Cancelled),
    }
}

fn validate_limits(limits: ProcessLimits) -> Result<(), ProcessError> {
    if limits.timeout.is_zero() {
        return Err(ProcessError::Timeout(Duration::ZERO));
    }
    if limits.maximum_stdout_bytes == 0 {
        return Err(ProcessError::OutputLimit {
            stream: "stdout",
            limit: 0,
        });
    }
    if limits.maximum_stderr_bytes == 0 {
        return Err(ProcessError::OutputLimit {
            stream: "stderr",
            limit: 0,
        });
    }
    Ok(())
}

async fn read_bounded<R>(
    mut reader: R,
    limit: usize,
    stream: &'static str,
    policy: OutputLimitPolicy,
    limit_sender: mpsc::Sender<(&'static str, usize)>,
    output_sender: Option<mpsc::Sender<ProcessChunk>>,
    cancellation: CancellationToken,
) -> Result<(Vec<u8>, bool), std::io::Error>
where
    R: AsyncRead + Unpin,
{
    let mut output = Vec::with_capacity(limit.min(READ_BUFFER_BYTES));
    let mut truncated = false;
    let mut buffer = [0_u8; READ_BUFFER_BYTES];
    loop {
        let count = reader.read(&mut buffer).await?;
        if count == 0 {
            return Ok((output, truncated));
        }
        let remaining = limit.saturating_sub(output.len());
        let retained = count.min(remaining);
        if retained > 0
            && let Some(sender) = &output_sender
        {
            tokio::select! {
                biased;
                () = cancellation.cancelled() => {},
                result = sender.send(ProcessChunk { stream, bytes: buffer[..retained].to_vec() }) => {
                    result.map_err(|_| std::io::Error::new(std::io::ErrorKind::BrokenPipe, "Output consumer closed."))?;
                }
            }
        }
        if count > remaining {
            output.extend_from_slice(&buffer[..remaining]);
            truncated = true;
            if policy == OutputLimitPolicy::Error {
                let _ = limit_sender.try_send((stream, limit));
            }
            // Continue draining until the owner kills the process. This avoids
            // a pipe deadlock if the notification races process completion.
            continue;
        }
        output.extend_from_slice(&buffer[..count]);
    }
}

async fn join_reader(
    reader: tokio::task::JoinHandle<Result<(Vec<u8>, bool), std::io::Error>>,
) -> Result<(Vec<u8>, bool), ProcessError> {
    reader
        .await
        .map_err(|error| ProcessError::Io(std::io::Error::other(error.to_string())))?
        .map_err(ProcessError::Io)
}
