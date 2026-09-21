use citadel_contracts::citadel::{containers::v1::*, edge::v1::EdgeCommandKind};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, terminal::*};
use futures_util::future::BoxFuture;
use prost::Message;
use tokio_util::sync::CancellationToken;

pub(crate) fn opening(id: &str, shell: TerminalShell) -> ExecClientMessage {
    ExecClientMessage {
        msg: Some(exec_client_message::Msg::Open(ExecOpen {
            container_id: id.into(),
            cmd: vec![shell.command().into()],
            tty: true,
        })),
    }
}
pub(crate) fn encode_input(input: TerminalInput) -> ExecClientMessage {
    ExecClientMessage {
        msg: Some(match input {
            TerminalInput::Stdin(data) => exec_client_message::Msg::Stdin(ExecStdin { data }),
            TerminalInput::Resize { cols, rows } => exec_client_message::Msg::Resize(ExecResize {
                cols: cols.into(),
                rows: rows.into(),
            }),
        }),
    }
}
pub(crate) fn decode_output(
    message: ExecServerMessage,
) -> Result<TerminalOutput, RuntimeCapabilityError> {
    match message.msg {
        Some(exec_server_message::Msg::Output(output))
            if output.data.len() <= MAX_TERMINAL_OUTPUT =>
        {
            Ok(TerminalOutput::Data(output.data))
        }
        Some(exec_server_message::Msg::Exit(exit)) => Ok(TerminalOutput::Exit(exit.exit_code)),
        // Raw Agent errors can contain host paths or execution details; don't expose them to viewers.
        Some(exec_server_message::Msg::Error(_)) => Err(failure("Agent terminal failed.")),
        _ => Err(failure("Invalid or oversized Agent terminal frame.")),
    }
}
fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}

impl ContainerTerminalPort for crate::connectors::docker::DockerClient {
    fn container_terminal<'a>(
        &'a self,
        id: &'a str,
        shell: TerminalShell,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<TerminalSession, RuntimeCapabilityError>> {
        Box::pin(async move {
            tokio::select! {
                biased;
                ()=cancel.cancelled()=>Err(cancelled()),
                result=tokio::time::timeout(self.request_timeout(),self.open_terminal(id,shell,cancel))=>
                    result.map_err(|_|RuntimeCapabilityError::new(RuntimeErrorKind::Timeout,"Opening terminal timed out.",false))?.map_err(crate::connectors::docker::runtime::normalize_docker_error),
            }
        })
    }
}

pub(crate) fn local_session(
    mut socket: reqwest::Upgraded,
    client: crate::connectors::docker::DockerClient,
    exec_id: String,
    timeout: std::time::Duration,
    cancel: CancellationToken,
) -> TerminalSession {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    let (input, mut receiver) = input_channel();
    let output = Box::pin(async_stream::try_stream! {
        let mut buffer = vec![0;16*1024];
        loop {
            let event=tokio::select! {
                biased;
                ()=cancel.cancelled()=>break,
                input=receiver.recv()=>Ok(input),
                read=socket.read(&mut buffer)=>Err(read),
            };
            match event {
                Err(read)=>{
                    let read=read.map_err(|_|failure("Docker terminal read failed."))?;
                    if read==0 {break;}
                    yield TerminalOutput::Data(buffer[..read].to_vec());
                },
                Ok(None)=>break,
                Ok(Some(input))=>{
                    let is_resize = matches!(&input, TerminalInput::Resize { .. });
                    let write=async {
                        match input {
                            TerminalInput::Stdin(data)=>socket.write_all(&data).await.map_err(|_|failure("Docker terminal write failed.")),
                            TerminalInput::Resize {cols,rows}=>{
                                client.resize_exec(&exec_id, cols, rows).await.map_err(|_|failure("Docker terminal resize failed."))
                            }
                        }
                    };
                    let result=tokio::select! {
                        biased;
                        ()=cancel.cancelled()=>break,
                        result=tokio::time::timeout(timeout,write)=>result,
                    };
                    let result = result.map_err(|_|failure("Docker terminal input timed out.")).and_then(|result| result);
                    if let Err(error) = result {
                        if is_resize {
                            // A rejected or failed resize is
                            // best-effort and must not tear down the shell.
                            tracing::warn!(%error, "Docker terminal resize failed");
                        } else {
                            Err(error)?;
                        }
                    }
                }
            }
        }
    });
    TerminalSession { input, output }
}

impl ContainerTerminalPort for crate::connectors::edge::EdgeRuntime {
    fn container_terminal<'a>(
        &'a self,
        id: &'a str,
        shell: TerminalShell,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<TerminalSession, RuntimeCapabilityError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let mut command = self
                .session
                .command(
                    EdgeCommandKind::ContainerExec,
                    opening(id, shell).encode_to_vec(),
                    std::time::Duration::from_secs(24 * 60 * 60),
                    true,
                )
                .map_err(|e| failure(&e.to_string()))?;
            let (input, mut receiver) = input_channel();
            let cancel = cancel.clone();
            let output = Box::pin(async_stream::try_stream! {
                loop {
                    let event = tokio::select! {
                        biased;
                        ()=cancel.cancelled()=>break,
                        item=receiver.recv()=>Ok(item),
                        bytes=command.next(&cancel)=>Err(bytes),
                    };
                    let bytes = match event {
                        Ok(item)=>{
                            let Some(item)=item else {break};
                            command.send_input(encode_input(item).encode_to_vec()).map_err(|e|failure(&e.to_string()))?;
                            continue;
                        },
                        Err(bytes)=>bytes.map_err(|e|failure(&e.to_string()))?,
                    };
                    let Some(bytes)=bytes else {break};
                    let message=ExecServerMessage::decode(bytes.as_slice()).map_err(|_|failure("Invalid Agent terminal frame."))?;
                    let item=decode_output(message)?;
                    let exited=matches!(item,TerminalOutput::Exit(_));
                    yield item;
                    if exited {break;}
                }
            });
            Ok(TerminalSession { input, output })
        })
    }
}

fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Cancelled,
        "Terminal session canceled.",
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn terminal_frames_are_bounded_and_agent_errors_are_not_exposed() {
        assert!(decode_output(ExecServerMessage { msg: None }).is_err());
        assert!(
            decode_output(ExecServerMessage {
                msg: Some(exec_server_message::Msg::Output(ExecOutput {
                    data: vec![0; MAX_TERMINAL_OUTPUT + 1],
                    stream: 0
                }))
            })
            .is_err()
        );
        let error = decode_output(ExecServerMessage {
            msg: Some(exec_server_message::Msg::Error(ExecError {
                message: "private host path".into(),
            })),
        });
        assert!(matches!(error,Err(error) if !error.to_string().contains("private")));
        assert!(matches!(
            decode_output(ExecServerMessage {
                msg: Some(exec_server_message::Msg::Exit(ExecExit { exit_code: 7 }))
            })
            .unwrap(),
            TerminalOutput::Exit(7)
        ));
    }
}
