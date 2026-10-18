//! Binary-safe execution shared by local helpers and Agent transports.
use super::*;
use citadel_contracts::citadel::containers::v1::{
    ExecExit, ExecOutput, ExecServerMessage, exec_server_message::Msg,
};
use citadel_docker_api::models::ExecConfig;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

pub enum DockerExecEvent {
    Output { data: Vec<u8>, stderr: bool },
    Exit(i32),
}
pub type DockerExecStream =
    futures_util::stream::BoxStream<'static, Result<DockerExecEvent, DockerExecError>>;

#[derive(Debug, thiserror::Error)]
pub enum DockerExecError {
    #[error("Binary execution canceled.")]
    Cancelled,
    #[error("{0}")]
    Transport(&'static str),
    #[error("{0}")]
    InvalidFrame(&'static str),
    #[error("Docker binary frame exceeds the limit.")]
    FrameTooLarge,
}

impl DockerExecError {
    fn into_status(self) -> tonic::Status {
        match self {
            Self::Cancelled => tonic::Status::cancelled(self.to_string()),
            Self::Transport(_) => tonic::Status::unavailable(self.to_string()),
            Self::InvalidFrame(_) => tonic::Status::data_loss(self.to_string()),
            Self::FrameTooLarge => tonic::Status::resource_exhausted(self.to_string()),
        }
    }
}

fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Cancelled,
        "Binary execution canceled.",
        false,
    )
}

use super::owned_exec::{LAUNCH, ProcessIdentity, parse_identity};
use tokio::sync::{Semaphore, mpsc, oneshot};
static EXEC_SLOTS: Semaphore = Semaphore::const_new(64);
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);

impl DockerClient {
    pub async fn execute_binary(
        &self,
        id: &str,
        mut config: ExecConfig,
        cancellation: &CancellationToken,
    ) -> Result<DockerExecStream, RuntimeCapabilityError> {
        if config.attach_stdin == Some(true) || config.cmd.as_ref().is_none_or(Vec::is_empty) {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "Binary execution requires a command and does not accept stdin.",
                false,
            ));
        }
        if cancellation.is_cancelled() {
            return Err(cancelled());
        }
        let unavailable = || {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::ResourceExhausted,
                "Exec capacity exceeded or shutting down.",
                false,
            )
        };
        let permit = EXEC_SLOTS.try_acquire().map_err(|_| unavailable())?;
        let reservation = self.terminal_tasks.reserve().ok_or_else(unavailable)?;
        let shutdown = self.terminal_tasks.cancellation();
        let cancel = cancellation.child_token();
        let guard = cancel.clone().drop_guard();
        let driver_cancel = cancel.clone();
        let user = config.user.clone();
        let stdout = config.attach_stdout == Some(true);
        let output_enabled = stdout || config.attach_stderr == Some(true);
        let tty = config.tty.unwrap_or(false);
        config.attach_stdin = Some(false);
        // The private startup record always uses stdout; suppress only command
        // stdout afterwards when the caller did not request it.
        config.attach_stdout = Some(true);
        let mut command = vec![
            "/bin/sh".into(),
            "-c".into(),
            LAUNCH.into(),
            "citadel-exec".into(),
        ];
        command.extend(config.cmd.take().expect("validated command"));
        config.cmd = Some(command);
        let client = self.clone();
        let id = id.to_owned();
        let (opened, opening) = oneshot::channel();
        let (sender, mut receiver) = mpsc::channel(8);
        reservation.spawn(async move {
            let _permit = permit;
            // Complete startup independently of a dropped caller, so an exec
            // started by Docker is still identified and cleaned up.
            let result =
                tokio::time::timeout(OPEN_TIMEOUT, client.open_owned_binary(&id, config, tty))
                    .await;
            let (mut stream, exec_id, identity) = match result {
                Ok(Ok(value)) => value,
                result => {
                    let error = match result {
                        Ok(Err(error)) => error,
                        _ => RuntimeCapabilityError::new(
                            RuntimeErrorKind::Timeout,
                            "Opening binary exec timed out.",
                            false,
                        ),
                    };
                    let _ = opened.send(Err(error));
                    return;
                }
            };
            let mut exited = false;
            let approval = tokio::select! {
                biased;
                () = driver_cancel.cancelled() => Err(cancelled()),
                () = shutdown.cancelled() => Err(cancelled()),
                result = tokio::time::timeout(Duration::from_secs(2), client.approve_exec(&id, &identity, user.as_deref())) =>
                    result.map_err(|_| RuntimeCapabilityError::new(RuntimeErrorKind::Timeout, "Approving binary exec timed out.", false))
                        .and_then(|result| result.map_err(crate::connectors::docker::runtime::normalize_docker_error)),
            };
            let approval = if driver_cancel.is_cancelled() || shutdown.is_cancelled() { Err(cancelled()) } else { approval };
            let approved = approval.is_ok();
            if opened.send(approval).is_ok() && approved {
                let drive = async {
                    while let Some(item) = stream.next().await {
                        let item = item?;
                        let done = matches!(item, DockerExecEvent::Exit(_));
                        if matches!(item, DockerExecEvent::Output { .. })
                            && (!output_enabled
                                || (!tty
                                    && !stdout
                                    && matches!(
                                        item,
                                        DockerExecEvent::Output { stderr: false, .. }
                                    )))
                        {
                            continue;
                        }
                        if sender.send(Ok(item)).await.is_err() {
                            return Ok(false);
                        }
                        if done {
                            return Ok(true);
                        }
                    }
                    Ok::<_, DockerExecError>(false)
                };
                let result = tokio::select! {
                    biased;
                    () = driver_cancel.cancelled() => Err(DockerExecError::Cancelled),
                    () = shutdown.cancelled() => Err(DockerExecError::Cancelled),
                    () = sender.closed() => Ok(false),
                    result = drive => result,
                };
                exited = matches!(result, Ok(true));
                if let Err(error) = result {
                    let _ = sender.try_send(Err(error));
                }
            }
            if !exited
                && !matches!(
                    tokio::time::timeout(
                        CLEANUP_TIMEOUT,
                        client.cleanup_exec(&id, &exec_id, &identity, user.as_deref())
                    )
                    .await,
                    Ok(Ok(()))
                )
            {
                tracing::warn!(%exec_id, "Could not confirm binary exec cleanup");
            }
        });
        tokio::select! {
            biased;
            () = cancel.cancelled() => return Err(cancelled()),
            result = opening => result.unwrap_or_else(|_| Err(unavailable()))?,
        }
        Ok(Box::pin(async_stream::stream! {
            let _guard = guard;
            while let Some(item) = receiver.recv().await { yield item; }
        }))
    }

    async fn open_owned_binary(
        &self,
        id: &str,
        config: ExecConfig,
        tty: bool,
    ) -> Result<(DockerExecStream, String, ProcessIdentity), RuntimeCapabilityError> {
        let open = async {
            validate_identifier(id)?;
            let exec_id = self.create_exec(id, serde_json::to_value(config)?).await?;
            validate_identifier(&exec_id)?;
            let version = self.negotiated_version().await?;
            let response = self
                .client
                .post(format!(
                    "{}/v{version}/exec/{}/start",
                    self.base_url,
                    urlencoding::encode(&exec_id)
                ))
                .header("Connection", "Upgrade")
                .header("Upgrade", "tcp")
                .json(&serde_json::json!({"Detach":false,"Tty":tty}))
                .send()
                .await?;
            if response.status() != StatusCode::SWITCHING_PROTOCOLS {
                return Err(DockerError::Api {
                    status: response.status(),
                    message: "Docker did not upgrade binary execution.".into(),
                });
            }
            Ok((response.upgrade().await?, exec_id))
        };
        let (socket, exec_id) = open
            .await
            .map_err(crate::connectors::docker::runtime::normalize_docker_error)?;
        let mut stream = self.decode_binary(socket, exec_id.clone(), tty);
        let invalid = || {
            RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                "Could not identify the binary exec process session.",
                false,
            )
        };
        let mut line = Vec::new();
        let mut early_stderr = Vec::new();
        let mut early_bytes = 0usize;
        loop {
            let Some(Ok(DockerExecEvent::Output { data, stderr })) = stream.next().await else {
                return Err(invalid());
            };
            // Docker drains stdout/stderr independently. A fast command's stderr
            // can arrive before the earlier identity write on stdout.
            if stderr {
                early_bytes += data.len();
                if early_bytes > 1024 * 1024 {
                    return Err(invalid());
                }
                early_stderr.push(data);
                continue;
            }
            let end = data.iter().position(|b| *b == b'\n');
            let count = end.map_or(data.len(), |n| n + 1);
            if line.len() + count > 128 {
                return Err(invalid());
            }
            line.extend_from_slice(&data[..count]);
            if end.is_some() {
                let identity = parse_identity(&line).ok_or_else(invalid)?;
                let pending = data[count..].to_vec();
                let output = Box::pin(async_stream::stream! {
                    for data in early_stderr { yield Ok(DockerExecEvent::Output {data, stderr: true}); }
                    if !pending.is_empty() { yield Ok(DockerExecEvent::Output {data: pending, stderr: false}); }
                    while let Some(item) = stream.next().await { yield item; }
                });
                return Ok((output, exec_id, identity));
            }
        }
    }

    fn decode_binary(
        &self,
        mut socket: reqwest::Upgraded,
        exec_id: String,
        tty: bool,
    ) -> DockerExecStream {
        let docker = self.clone();
        Box::pin(async_stream::try_stream! {
            loop {
                if tty {
                    let mut data = vec![0; 64 * 1024];
                    let count = socket.read(&mut data).await.map_err(|_| DockerExecError::Transport("Docker binary stream disconnected."))?;
                    if count == 0 { break; }
                    data.truncate(count);
                    yield DockerExecEvent::Output { data, stderr: false };
                    continue;
                }
                let mut header = [0u8; 8];
                let count = socket.read(&mut header[..1]).await.map_err(|_| DockerExecError::Transport("Docker binary stream disconnected."))?;
                if count == 0 { break; }
                socket.read_exact(&mut header[1..]).await.map_err(|_| DockerExecError::InvalidFrame("Truncated Docker binary header."))?;
                if !matches!(header[0], 1 | 2) || header[1..4] != [0,0,0] {
                    Err(DockerExecError::InvalidFrame("Invalid Docker binary header."))?;
                }
                let mut remaining = u32::from_be_bytes(header[4..8].try_into().expect("four length bytes")) as usize;
                if remaining > 1024 * 1024 { Err(DockerExecError::FrameTooLarge)?; }
                while remaining > 0 {
                    let mut data = vec![0u8; remaining.min(64 * 1024)];
                    socket.read_exact(&mut data).await.map_err(|_| DockerExecError::InvalidFrame("Truncated Docker binary frame."))?;
                    remaining -= data.len();
                    yield DockerExecEvent::Output { data, stderr: header[0] == 2 };
                }
            }
            let exit = docker.exec_inspect(&exec_id).await.map_err(|_| DockerExecError::Transport("Could not confirm Docker binary execution completion."))?;
            if exit.running != Some(false) { Err(DockerExecError::Transport("Docker binary execution is still running."))?; }
            let code = exit.exit_code.ok_or(DockerExecError::Transport("Docker omitted the exit code."))?;
            yield DockerExecEvent::Exit(code);
        })
    }

    pub(crate) async fn exec_binary_stream(
        &self,
        id: &str,
        command: &[String],
        cancellation: &CancellationToken,
    ) -> Result<
        futures_util::stream::BoxStream<'static, Result<ExecServerMessage, tonic::Status>>,
        RuntimeCapabilityError,
    > {
        let stream = self
            .execute_binary(
                id,
                ExecConfig {
                    cmd: Some(command.to_vec()),
                    attach_stdout: Some(true),
                    attach_stderr: Some(true),
                    tty: Some(false),
                    ..Default::default()
                },
                cancellation,
            )
            .await?;
        Ok(Box::pin(stream.map(|event| {
            event
                .map(|event| ExecServerMessage {
                    msg: Some(match event {
                        DockerExecEvent::Output { data, stderr } => Msg::Output(ExecOutput {
                            data,
                            stream: i32::from(stderr),
                        }),
                        DockerExecEvent::Exit(exit_code) => Msg::Exit(ExecExit { exit_code }),
                    }),
                })
                .map_err(DockerExecError::into_status)
        })))
    }
}
