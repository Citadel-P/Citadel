use citadel_contracts::citadel::{
    containers::v1::{ContainerLogRequest, ContainerLogResponse},
    edge::v1::EdgeCommandKind,
};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, logs::*};
use futures_util::{StreamExt, future::BoxFuture};
use prost::Message;
use tokio_util::sync::CancellationToken;

impl ContainerLogPort for crate::connectors::docker::DockerClient {
    fn container_logs<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeLogStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            tokio::select! {
                biased;
                ()=cancel.cancelled()=>Err(cancelled()),
                result=tokio::time::timeout(self.request_timeout(),self.open_logs(id,cancel))=>
                    result.map_err(|_|failure("Opening container logs timed out."))?.map_err(crate::connectors::docker::runtime::normalize_docker_error),
            }
        })
    }
}
impl ContainerLogPort for crate::connectors::edge::EdgeRuntime {
    fn container_logs<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeLogStream, RuntimeCapabilityError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                return Err(cancelled());
            }
            let mut command = self
                .session
                .command(
                    EdgeCommandKind::ContainerLogsStream,
                    ContainerLogRequest {
                        container_id: id.into(),
                        follow: Some(true),
                        tail: 100,
                    }
                    .encode_to_vec(),
                    std::time::Duration::from_secs(24 * 60 * 60),
                    true,
                )
                .map_err(crate::connectors::edge::EdgeError::runtime)?;
            let cancel = cancel.clone();
            Ok(Box::pin(async_stream::try_stream! {
                loop {
                    let next=tokio::select! {biased;()=cancel.cancelled()=>break,next=command.next(&cancel)=>next};
                    let Some(bytes)=next.map_err(crate::connectors::edge::EdgeError::runtime)? else {break};
                    let frame=ContainerLogResponse::decode(bytes.as_slice()).map_err(|_|failure("Invalid Agent log frame."))?;
                    if frame.log.len()>MAX_LOG_FRAME {Err(failure("Agent log frame exceeds the limit."))?;}
                    yield frame.log;
                }
            }) as RuntimeLogStream)
        })
    }
}

impl LogReadPort for crate::connectors::docker::DockerClient {
    fn read_logs<'a>(
        &'a self,
        resource: LogResource<'a>,
        tail: u16,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<LogSnapshot, RuntimeCapabilityError>> {
        Box::pin(async move {
            validate_tail(tail)?;
            tokio::select! {
                biased;
                () = cancel.cancelled() => Err(cancelled()),
                result = tokio::time::timeout(self.request_timeout(), async {
                    let stream = self.open_resource_logs(resource, false, tail, cancel).await.map_err(crate::connectors::docker::runtime::normalize_docker_error)?;
                    snapshot(stream).await
                }) => result.map_err(|_|failure("Reading logs timed out."))?,
            }
        })
    }
}

impl LogReadPort for crate::connectors::edge::EdgeRuntime {
    fn read_logs<'a>(
        &'a self,
        resource: LogResource<'a>,
        tail: u16,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<LogSnapshot, RuntimeCapabilityError>> {
        Box::pin(async move {
            validate_tail(tail)?;
            match resource {
                LogResource::Service(id) => {
                    let response: citadel_contracts::citadel::swarm::v1::SwarmLogsResponse =
                        crate::connectors::agent::execution::unary(
                            &self.session,
                            EdgeCommandKind::SwarmServiceLogs,
                            citadel_contracts::citadel::swarm::v1::SwarmLogsRequest {
                                resource_id: id.into(),
                                tail: tail.into(),
                            },
                            cancel,
                        )
                        .await?;
                    checked_snapshot(response.lines, response.truncated)
                }
                LogResource::Container(id) => {
                    let mut command = self
                        .session
                        .command(
                            EdgeCommandKind::ContainerLogsStream,
                            ContainerLogRequest {
                                container_id: id.into(),
                                follow: Some(false),
                                tail: tail.into(),
                            }
                            .encode_to_vec(),
                            std::time::Duration::from_secs(30),
                            true,
                        )
                        .map_err(crate::connectors::edge::EdgeError::runtime)?;
                    let cancel = cancel.clone();
                    let stream = Box::pin(async_stream::try_stream! {
                        while let Some(bytes) = command.next(&cancel).await.map_err(crate::connectors::edge::EdgeError::runtime)? {
                            let response = ContainerLogResponse::decode(bytes.as_slice()).map_err(|_|failure("Invalid Agent log frame."))?;
                            if response.log.len()>MAX_LOG_FRAME {Err(failure("Agent log frame exceeds the limit."))?;}
                            yield response.log;
                        }
                    });
                    snapshot(stream).await
                }
            }
        })
    }
}

pub(crate) fn validate_tail(tail: u16) -> Result<(), RuntimeCapabilityError> {
    if !(1..=200).contains(&tail) {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "Tail must be between 1 and 200.",
            false,
        ));
    }
    Ok(())
}

pub(crate) fn checked_snapshot(
    lines: Vec<String>,
    truncated: bool,
) -> Result<LogSnapshot, RuntimeCapabilityError> {
    if lines.len() > MAX_LOG_FRAME || lines.iter().map(String::len).sum::<usize>() > MAX_LOG_FRAME {
        return Err(failure("Agent log response exceeds the limit."));
    }
    Ok(LogSnapshot { lines, truncated })
}

pub(crate) async fn snapshot(
    mut stream: RuntimeLogStream,
) -> Result<LogSnapshot, RuntimeCapabilityError> {
    let mut bytes = Vec::new();
    let mut truncated = false;
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let remaining = MAX_LOG_FRAME - bytes.len();
        bytes.extend_from_slice(&chunk[..remaining.min(chunk.len())]);
        if chunk.len() > remaining {
            truncated = true;
            break;
        }
    }
    // Decode only after joining frames, since a UTF-8 character may cross frames.
    Ok(LogSnapshot {
        lines: String::from_utf8_lossy(&bytes)
            .split_inclusive('\n')
            .map(str::to_owned)
            .collect(),
        truncated,
    })
}
pub(crate) fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}

fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Cancelled,
        "Log subscription canceled.",
        false,
    )
}

// Docker's non-TTY logs have an eight-byte multiplex header. TTY output is raw.
// Keep only one bounded partial frame; reject invalid/truncated frames visibly.
pub(crate) fn decode_frames(
    mut input: futures_util::stream::BoxStream<
        'static,
        Result<bytes::Bytes, crate::connectors::docker::DockerError>,
    >,
    tty: bool,
    cancel: CancellationToken,
) -> RuntimeLogStream {
    Box::pin(async_stream::try_stream! {
        let mut pending=Vec::new();
        loop {
            let next=tokio::select!{biased;()=cancel.cancelled()=>break,next=input.next()=>next};
            let Some(next)=next else {if !pending.is_empty(){Err(failure("Docker log stream ended in a partial frame."))?;}break;};
            let mut chunk=next.map_err(crate::connectors::docker::runtime::normalize_docker_error)?;
            if tty {
                while !chunk.is_empty() {
                    let output=chunk.split_to(chunk.len().min(16*1024)).to_vec();
                    if chunk.is_empty() {chunk=bytes::Bytes::new();}
                    yield output;
                }
                continue;
            }
            while !chunk.is_empty() {
                let wanted=if pending.len()<8 {8} else {
                    if !matches!(pending[0],0..=2)||pending[1..4]!=[0,0,0] {Err(failure("Invalid Docker log frame header."))?;}
                    let size=u32::from_be_bytes(pending[4..8].try_into().unwrap()) as usize;
                    if size>MAX_LOG_FRAME {Err(failure("Docker log frame exceeds the limit."))?;}
                    8+size
                };
                let amount=(wanted-pending.len()).min(chunk.len());
                pending.extend_from_slice(&chunk.split_to(amount));
                if pending.len()==wanted && wanted>=8 && (wanted>8 || pending[4..8]==[0,0,0,0]) {
                    if !matches!(pending[0],0..=2)||pending[1..4]!=[0,0,0] {Err(failure("Invalid Docker log frame header."))?;}
                    let output = (pending.len()>8).then(|| pending[8..].to_vec());
                    citadel_runtime::reset_stream_buffer(&mut pending);
                    if chunk.is_empty() { chunk=bytes::Bytes::new(); }
                    if let Some(output)=output {yield output;}
                }
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;

    #[tokio::test]
    async fn snapshots_bound_total_bytes_preserve_split_utf8_and_drop_the_source_on_truncation() {
        let result = snapshot(Box::pin(futures_util::stream::iter([
            Ok(b"h\xc3".to_vec()),
            Ok(b"\xa9llo\n".to_vec()),
        ])))
        .await
        .unwrap();
        assert_eq!(result.lines, ["héllo\n"]);
        assert!(!result.truncated);
        let result = snapshot(Box::pin(futures_util::stream::iter([
            Ok(vec![b'x'; MAX_LOG_FRAME]),
            Ok(vec![b'y']),
        ])))
        .await
        .unwrap();
        assert!(result.truncated);
        assert_eq!(
            result.lines.iter().map(String::len).sum::<usize>(),
            MAX_LOG_FRAME
        );
        assert!(checked_snapshot(vec!["x".repeat(MAX_LOG_FRAME + 1)], false).is_err());
        assert!(validate_tail(0).is_err());
        assert!(validate_tail(201).is_err());
    }

    fn frame(channel: u8, data: &[u8]) -> Vec<u8> {
        let mut bytes = vec![channel, 0, 0, 0];
        bytes.extend_from_slice(&(data.len() as u32).to_be_bytes());
        bytes.extend_from_slice(data);
        bytes
    }
    fn stream(chunks: Vec<Vec<u8>>, tty: bool) -> RuntimeLogStream {
        decode_frames(
            Box::pin(futures_util::stream::iter(
                chunks.into_iter().map(|b| Ok(Bytes::from(b))),
            )),
            tty,
            CancellationToken::new(),
        )
    }
    #[tokio::test]
    async fn multiplexed_logs_handle_every_header_and_payload_split_without_emitting_headers() {
        let mut data = frame(1, "hello é\n".as_bytes());
        data.extend(frame(2, b"error\n"));
        data.extend(frame(1, b""));
        for split in 0..=data.len() {
            let decoded: Vec<_> =
                stream(vec![data[..split].to_vec(), data[split..].to_vec()], false)
                    .collect()
                    .await;
            let bytes: Vec<_> = decoded.into_iter().flat_map(Result::unwrap).collect();
            assert_eq!(bytes, "hello é\nerror\n".as_bytes(), "split {split}");
        }
    }
    #[tokio::test]
    async fn large_frames_can_be_released_without_losing_following_frames() {
        let payload = vec![b'x'; MAX_LOG_FRAME];
        let mut data = frame(1, &payload);
        data.extend(frame(2, b"next\n"));
        let mut decoded = stream(vec![data], false);
        assert_eq!(decoded.next().await.unwrap().unwrap(), payload);
        assert_eq!(decoded.next().await.unwrap().unwrap(), b"next\n");
        assert!(decoded.next().await.is_none());
    }

    #[tokio::test]
    async fn tty_logs_are_raw_and_output_chunks_are_bounded() {
        let data = vec![b'x'; 50_000];
        let output: Vec<_> = stream(vec![data.clone()], true).collect().await;
        assert!(
            output
                .iter()
                .all(|v| v.as_ref().unwrap().len() <= 16 * 1024)
        );
        assert_eq!(
            output
                .into_iter()
                .flat_map(Result::unwrap)
                .collect::<Vec<_>>(),
            data
        );
    }
    #[tokio::test]
    async fn consumed_input_is_released_before_yielding_the_last_output() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
        };

        struct TrackedChunk(Vec<u8>, Arc<AtomicBool>);
        impl AsRef<[u8]> for TrackedChunk {
            fn as_ref(&self) -> &[u8] {
                &self.0
            }
        }
        impl Drop for TrackedChunk {
            fn drop(&mut self) {
                self.1.store(true, Ordering::SeqCst);
            }
        }

        for tty in [false, true] {
            let payload = vec![b'x'; 32 * 1024];
            let data = if tty {
                payload.clone()
            } else {
                frame(1, &payload)
            };
            let released = Arc::new(AtomicBool::new(false));
            let chunk = Bytes::from_owner(TrackedChunk(data, released.clone()));
            let input =
                futures_util::stream::iter([Ok(chunk)]).chain(futures_util::stream::pending());
            let mut decoded = decode_frames(Box::pin(input), tty, CancellationToken::new());
            let mut output = decoded.next().await.unwrap().unwrap();
            if tty {
                assert!(!released.load(Ordering::SeqCst));
                output.extend(decoded.next().await.unwrap().unwrap());
            }
            assert_eq!(output, payload);
            // Do not poll again: a slow consumer can leave the stream suspended here.
            assert!(released.load(Ordering::SeqCst), "tty={tty}");
        }
    }
    #[tokio::test]
    async fn truncated_invalid_and_oversized_frames_fail() {
        let mut oversized = vec![1, 0, 0, 0];
        oversized.extend_from_slice(&((MAX_LOG_FRAME + 1) as u32).to_be_bytes());
        oversized.push(1);
        for bytes in [
            vec![1, 0],
            frame(3, b"bad"),
            vec![1, 0, 0, 0, 0, 0, 0, 2, 1],
            oversized,
        ] {
            assert!(stream(vec![bytes], false).next().await.unwrap().is_err());
        }
    }
    #[tokio::test]
    async fn cancel_interrupts_an_idle_log_stream() {
        let cancel = CancellationToken::new();
        let mut logs = decode_frames(
            Box::pin(futures_util::stream::pending()),
            false,
            cancel.clone(),
        );
        cancel.cancel();
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(1), logs.next())
                .await
                .unwrap()
                .is_none()
        );
    }
}
