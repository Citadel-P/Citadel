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

impl DockerClient {
    pub async fn execute_binary(
        &self,
        id: &str,
        mut config: ExecConfig,
        cancellation: &CancellationToken,
    ) -> Result<DockerExecStream, RuntimeCapabilityError> {
        if config.attach_stdin == Some(true) {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::InvalidRequest,
                "Binary execution does not accept stdin.",
                false,
            ));
        }
        config.attach_stdin = Some(false);
        let tty = config.tty.unwrap_or(false);
        let open = async {
            validate_identifier(id)?;
            let created = self.create_exec(id, serde_json::to_value(config)?).await?;
            validate_identifier(&created)?;
            let version = self.negotiated_version().await?;
            let path = format!("/exec/{}/start", urlencoding::encode(&created));
            let response = self
                .client
                .post(format!("{}/v{version}{path}", self.base_url))
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
            Ok::<_, DockerError>((response.upgrade().await?, created))
        };
        let (mut socket, exec_id) = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled()),
            result = open => result.map_err(crate::connectors::docker::runtime::normalize_docker_error)?
        };
        let docker = self.clone();
        let cancellation = cancellation.clone();
        Ok(Box::pin(async_stream::try_stream! {
            loop {
                if tty {
                    let mut data = vec![0; 64 * 1024];
                    let count = tokio::select! {
                        biased;
                        () = cancellation.cancelled() => Err(DockerExecError::Cancelled),
                        result = socket.read(&mut data) => result.map_err(|_| DockerExecError::Transport("Docker binary stream disconnected.")),
                    }?;
                    if count == 0 { break; }
                    data.truncate(count);
                    yield DockerExecEvent::Output { data, stderr: false };
                    continue;
                }
                let mut header = [0u8; 8];
                let count = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => Err(DockerExecError::Cancelled),
                    result = socket.read(&mut header[..1]) => result.map_err(|_| DockerExecError::Transport("Docker binary stream disconnected."))
                }?;
                if count == 0 { break; }
                tokio::select! {
                    biased;
                    () = cancellation.cancelled() => Err(DockerExecError::Cancelled),
                    result = socket.read_exact(&mut header[1..]) => result.map(|_| ()).map_err(|_| DockerExecError::InvalidFrame("Truncated Docker binary header.")),
                }?;
                if !matches!(header[0], 1 | 2) || header[1..4] != [0,0,0] {
                    Err(DockerExecError::InvalidFrame("Invalid Docker binary header."))?;
                }
                let mut remaining = u32::from_be_bytes(header[4..8].try_into().expect("four length bytes")) as usize;
                if remaining > 1024 * 1024 {
                    Err(DockerExecError::FrameTooLarge)?;
                }
                while remaining > 0 {
                    let mut data = vec![0u8; remaining.min(64 * 1024)];
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => Err(DockerExecError::Cancelled),
                        result = socket.read_exact(&mut data) => result.map(|_| ()).map_err(|_| DockerExecError::InvalidFrame("Truncated Docker binary frame."))
                    }?;
                    remaining -= data.len();
                    yield DockerExecEvent::Output { data, stderr: header[0] == 2 };
                }
            }
            let exit = tokio::select! {
                biased;
                () = cancellation.cancelled() => Err(DockerExecError::Cancelled),
                result = docker.exec_inspect(&exec_id) => result.map_err(|_| DockerExecError::Transport("Could not confirm Docker binary execution completion.")),
            }?;
            if exit.running != Some(false) { Err(DockerExecError::Transport("Docker binary execution is still running."))?; }
            let code = exit.exit_code.ok_or(DockerExecError::Transport("Docker omitted the exit code."))?;
            yield DockerExecEvent::Exit(code);
        }))
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
