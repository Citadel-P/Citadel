//! An attached Docker socket is not a process owner: disconnecting it leaves exec
//! running. Keep an owned driver through bounded, identity-checked cleanup.
use super::{DockerClient, DockerError, StatusCode, validate_identifier};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, terminal::*};
use std::time::Duration;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Semaphore, mpsc, oneshot},
    time::{Instant, timeout},
};
use tokio_util::sync::CancellationToken;

const IDLE_TIMEOUT: Duration = Duration::from_secs(15 * 60);
const OPEN_TIMEOUT: Duration = Duration::from_secs(5);
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(3);
static SLOTS: Semaphore = Semaphore::const_new(64);

use super::owned_exec::{LAUNCH, ProcessIdentity, parse_identity};

fn failure(message: &'static str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}

impl DockerClient {
    pub async fn execute_terminal(
        &self,
        id: &str,
        command: &[String],
        cancel: &CancellationToken,
    ) -> Result<TerminalSession, DockerError> {
        self.execute_terminal_with_idle(id, command, cancel, IDLE_TIMEOUT)
            .await
    }

    async fn execute_terminal_with_idle(
        &self,
        id: &str,
        command: &[String],
        cancel: &CancellationToken,
        idle: Duration,
    ) -> Result<TerminalSession, DockerError> {
        validate_identifier(id)?;
        if cancel.is_cancelled() {
            return Err(DockerError::Api {
                status: StatusCode::REQUEST_TIMEOUT,
                message: "Terminal opening canceled.".into(),
            });
        }
        let unavailable = || DockerError::Api {
            status: StatusCode::SERVICE_UNAVAILABLE,
            message: "Terminal capacity exceeded or shutting down.".into(),
        };
        let permit = SLOTS.try_acquire().map_err(|_| unavailable())?;
        let reservation = self.terminal_tasks.reserve().ok_or_else(unavailable)?;
        let shutdown = self.terminal_tasks.cancellation();
        let cancel = cancel.child_token();
        let guard = cancel.clone().drop_guard();
        let (input, receiver) = input_channel();
        let (sender, mut output) = mpsc::channel(8);
        let (opened, opening) = oneshot::channel();
        let client = self.clone();
        let id = id.to_owned();
        let command = command.to_vec();
        let driver_cancel = cancel.clone();
        reservation.spawn(async move {
            let _permit = permit;
            // Don't drop an in-flight exec-start on caller cancellation. Finish
            // learning its identity, then clean it up even if nobody receives it.
            let result = timeout(OPEN_TIMEOUT, client.open_owned_terminal(&id, &command)).await;
            let (mut socket, exec_id, identity) = match result {
                Ok(Ok(session)) => session,
                result => {
                    let error = match result {
                        Ok(Err(error)) => error,
                        _ => DockerError::Api {
                            status: StatusCode::GATEWAY_TIMEOUT,
                            message: "Opening terminal timed out.".into(),
                        },
                    };
                    let _ = opened.send(Err(error));
                    return;
                }
            };
            let mut exited = false;
            let approval = tokio::select! {
                biased;
                () = driver_cancel.cancelled() => Err(unavailable()),
                () = shutdown.cancelled() => Err(unavailable()),
                result = timeout(Duration::from_secs(2), client.approve_exec(&id, &identity, None)) =>
                    result.unwrap_or_else(|_| Err(DockerError::Api { status: StatusCode::GATEWAY_TIMEOUT, message: "Approving terminal timed out.".into() })),
            };
            let approval = if driver_cancel.is_cancelled() || shutdown.is_cancelled() { Err(unavailable()) } else { approval };
            let approved = approval.is_ok();
            if opened.send(approval).is_ok() && approved {
                let result = tokio::select! {
                    biased;
                    () = driver_cancel.cancelled() => Ok(false),
                    () = shutdown.cancelled() => Ok(false),
                    () = sender.closed() => Ok(false),
                    result = client.drive_terminal(&mut socket, &exec_id, receiver, &sender, idle) => result,
                };
                exited = matches!(result, Ok(true));
                if let Err(error) = result {
                    let _ = sender.try_send(Err(error));
                }
            }
            if exited { return; }
            // Cleanup is independent of the canceled request and remains tracked
            // by the host. Never stop/kill the application container itself.
            let cleanup = client.cleanup_exec(&id, &exec_id, &identity, None);
            match timeout(CLEANUP_TIMEOUT, cleanup).await {
                Ok(Ok(())) => {}
                _ => tracing::warn!(%exec_id, "Could not confirm terminal process cleanup"),
            }
        });
        let opened = tokio::select! {
            biased;
            () = cancel.cancelled() => Err(DockerError::Api { status: StatusCode::REQUEST_TIMEOUT, message: "Terminal opening canceled.".into() }),
            result = opening => result.unwrap_or_else(|_| Err(unavailable())),
        };
        opened?;
        let output = Box::pin(async_stream::stream! {
            let _guard = guard;
            while let Some(item) = output.recv().await {
                yield item;
            }
        });
        Ok(TerminalSession { input, output })
    }

    async fn open_owned_terminal(
        &self,
        id: &str,
        command: &[String],
    ) -> Result<(reqwest::Upgraded, String, ProcessIdentity), DockerError> {
        let mut wrapped = vec![
            "/bin/sh".to_owned(),
            "-c".into(),
            LAUNCH.into(),
            "citadel-terminal".into(),
        ];
        wrapped.extend_from_slice(command);
        let exec_id = self.create_exec(id, serde_json::json!({"AttachStdin":true,"AttachStdout":true,"AttachStderr":true,"Tty":true,"Cmd":wrapped})).await?;
        let mut socket = self.start_terminal_socket(&exec_id).await?;
        let mut line = Vec::new();
        while line.len() < 128 {
            let byte = socket.read_u8().await.map_err(DockerError::ProtocolIo)?;
            line.push(byte);
            if byte == b'\n' {
                break;
            }
        }
        let identity = parse_identity(&line).ok_or_else(|| DockerError::Api {
            status: StatusCode::BAD_GATEWAY,
            message: "Could not identify the terminal process session.".into(),
        })?;
        Ok((socket, exec_id, identity))
    }

    pub(super) async fn start_terminal_socket(
        &self,
        exec_id: &str,
    ) -> Result<reqwest::Upgraded, DockerError> {
        validate_identifier(exec_id)?;
        let version = self.negotiated_version().await?;
        let response = self
            .client
            .post(format!(
                "{}/v{version}/exec/{}/start",
                self.base_url,
                urlencoding::encode(exec_id)
            ))
            .header("Connection", "Upgrade")
            .header("Upgrade", "tcp")
            .json(&serde_json::json!({"Detach":false,"Tty":true}))
            .send()
            .await?;
        if response.status() != StatusCode::SWITCHING_PROTOCOLS {
            return Err(DockerError::Api {
                status: response.status(),
                message: "Docker did not upgrade the terminal connection.".into(),
            });
        }
        Ok(response.upgrade().await?)
    }

    async fn drive_terminal(
        &self,
        socket: &mut reqwest::Upgraded,
        exec_id: &str,
        mut input: mpsc::Receiver<TerminalInput>,
        output: &mpsc::Sender<Result<TerminalOutput, RuntimeCapabilityError>>,
        idle: Duration,
    ) -> Result<bool, RuntimeCapabilityError> {
        let mut buffer = vec![0; 16 * 1024];
        let mut deadline = Instant::now() + idle;
        let mut input_closed = false;
        loop {
            tokio::select! {
                biased;
                () = tokio::time::sleep_until(deadline) => {
                    let _ = output.try_send(Ok(TerminalOutput::Data(b"\r\n[terminal disconnected after inactivity]\r\n".to_vec())));
                    break;
                },
                read = socket.read(&mut buffer) => {
                    let count = read.map_err(|_| failure("Docker terminal read failed."))?;
                    if count == 0 {
                        let state = timeout(self.request_timeout, self.exec_inspect(exec_id)).await
                            .map_err(|_| failure("Terminal exit inspection timed out."))?
                            .map_err(|_| failure("Could not confirm Docker terminal completion."))?;
                        if state.running != Some(false) { return Err(failure("Docker terminal is still running.")); }
                        let code = state.exit_code.ok_or_else(|| failure("Docker omitted the terminal exit code."))?;
                        let _ = tokio::time::timeout_at(deadline, output.send(Ok(TerminalOutput::Exit(code)))).await;
                        return Ok(true);
                    }
                    if !input_closed { deadline = Instant::now() + idle; }
                    tokio::select! {
                        biased;
                        () = tokio::time::sleep_until(deadline) => break,
                        result = output.send(Ok(TerminalOutput::Data(buffer[..count].to_vec()))) =>
                            result.map_err(|_| failure("Terminal output is closed."))?,
                    }
                },
                item = input.recv(), if !input_closed => {
                    match item {
                        None => {
                            input_closed = true;
                            deadline = Instant::now() + Duration::from_secs(2);
                            timeout(self.request_timeout, socket.write_all(&[4])).await
                                .map_err(|_| failure("Closing terminal input timed out."))?
                                .map_err(|_| failure("Closing terminal input failed."))?;
                        },
                        Some(TerminalInput::Stdin(bytes)) => {
                            timeout(self.request_timeout, socket.write_all(&bytes)).await
                                .map_err(|_| failure("Docker terminal input timed out."))?
                                .map_err(|_| failure("Docker terminal input failed."))?;
                            deadline = Instant::now() + idle;
                        },
                        Some(TerminalInput::Resize {cols,rows}) => {
                            // Resize/layout events do not extend idle sessions.
                            if !matches!(timeout(self.request_timeout, self.resize_exec(exec_id,cols,rows)).await, Ok(Ok(()))) {
                                tracing::warn!(%exec_id, "Docker terminal resize failed");
                            }
                        },
                    }
                },
            }
        }
        Ok(false)
    }
}

#[cfg(test)]
mod tests;
