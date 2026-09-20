//! Binary exec is deliberately separate from the line-oriented log decoder.
use super::*;
use citadel_contracts::citadel::containers::v1::{
    ExecExit, ExecOutput, ExecServerMessage, exec_server_message::Msg,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use tokio::io::AsyncReadExt;
use tokio_util::sync::CancellationToken;

impl DockerClient {
    pub(crate) async fn exec_binary_stream(
        &self,
        id: &str,
        command: &[String],
        cancellation: &CancellationToken,
    ) -> Result<
        futures_util::stream::BoxStream<'static, Result<ExecServerMessage, tonic::Status>>,
        RuntimeCapabilityError,
    > {
        let open = async {
            validate_identifier(id)?;
            let created = self.create_exec(id, serde_json::json!({"AttachStdin":false,"AttachStdout":true,"AttachStderr":true,"Tty":false,"Cmd":command})).await?;
            validate_identifier(&created)?;
            let version = self.negotiated_version().await?;
            let path = format!("/exec/{}/start", urlencoding::encode(&created));
            let response = self
                .client
                .post(format!("http://localhost/v{version}{path}"))
                .header("Connection", "Upgrade")
                .header("Upgrade", "tcp")
                .json(&serde_json::json!({"Detach":false,"Tty":false}))
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
            () = cancellation.cancelled() => return Err(RuntimeCapabilityError::new(RuntimeErrorKind::Timeout,"Binary execution canceled.",false)),
            result = open => result.map_err(crate::docker::runtime::normalize_docker_error)?
        };
        let docker = self.clone();
        let cancellation = cancellation.clone();
        Ok(Box::pin(async_stream::try_stream! {
            loop {
                let mut header = [0u8; 8];
                // A clean EOF is valid only at a frame boundary. Read the remainder exactly.
                let count = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => Err(tonic::Status::cancelled("Binary execution canceled.")),
                    result = socket.read(&mut header[..1]) => result.map_err(|_| tonic::Status::unavailable("Docker binary stream disconnected."))
                }?;
                if count == 0 { break; }
                socket.read_exact(&mut header[1..]).await.map_err(|_| tonic::Status::data_loss("Truncated Docker binary header."))?;
                if !matches!(header[0], 1 | 2) || header[1..4] != [0,0,0] {
                    Err(tonic::Status::data_loss("Invalid Docker binary header."))?;
                }
                let mut remaining = u32::from_be_bytes(header[4..8].try_into().expect("four length bytes")) as usize;
                if remaining > 1024 * 1024 { Err(tonic::Status::resource_exhausted("Docker binary frame exceeds the limit."))?; }
                while remaining > 0 {
                    let mut data = vec![0u8; remaining.min(64 * 1024)];
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => Err(tonic::Status::cancelled("Binary execution canceled.")),
                        result = socket.read_exact(&mut data) => result.map(|_| ()).map_err(|_| tonic::Status::data_loss("Truncated Docker binary frame."))
                    }?;
                    remaining -= data.len();
                    yield ExecServerMessage { msg: Some(Msg::Output(ExecOutput { data, stream: i32::from(header[0] == 2) })) };
                }
            }
            let exit = docker.exec_inspect(&exec_id).await.map_err(|_| tonic::Status::unavailable("Could not confirm Docker binary execution completion."))?;
            if exit.running != Some(false) { Err(tonic::Status::unavailable("Docker binary execution is still running."))?; }
            let exit_code = exit.exit_code.ok_or_else(|| tonic::Status::data_loss("Docker omitted the exit code."))?;
            yield ExecServerMessage { msg: Some(Msg::Exit(ExecExit { exit_code })) };
        }))
    }
}
