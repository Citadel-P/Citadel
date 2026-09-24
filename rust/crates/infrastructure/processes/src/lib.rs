#![forbid(unsafe_code)]
use citadel_execution::{
    OutputLimitPolicy, ProcessChunk, ProcessError, ProcessLimits, ProcessOutput, ProcessRequest,
    ProcessRunner,
};
use futures_util::future::BoxFuture;
use std::process::{ExitStatus, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWriteExt};
use tokio::process::Command;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
const READ_BUFFER_BYTES: usize = 8 * 1024;
#[derive(Default)]
pub struct SystemProcess;
impl ProcessRunner for SystemProcess {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(run(request, cancellation))
    }
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

    // A caller may drop this future when its transport disconnects. Keep pipe
    // tasks owned by this invocation instead of detaching them with live pipes.
    let mut tasks = vec![stdout_reader.abort_handle(), stderr_reader.abort_handle()];
    tasks.extend(
        stdin_writer
            .as_ref()
            .map(tokio::task::JoinHandle::abort_handle),
    );
    let _tasks = PipeTasks(tasks);
    let _output_guard = output_cancellation.clone().drop_guard();

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

struct PipeTasks(Vec<tokio::task::AbortHandle>);
impl Drop for PipeTasks {
    fn drop(&mut self) {
        for task in &self.0 {
            task.abort();
        }
    }
}
