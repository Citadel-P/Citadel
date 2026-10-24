use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind, logs::*};
use futures_util::{StreamExt, future::BoxFuture};
use tokio_util::sync::CancellationToken;

impl ContainerLogPort for crate::client::DockerClient {
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
                    result.map_err(|_|failure("Opening container logs timed out."))?.map_err(normalize_container_log_error),
            }
        })
    }
}

/// Recognize only Docker's specific container-removal conflict. Agent callers
/// retain their existing authentication and transport mappings for other errors.
pub fn is_container_log_removal_conflict(error: &crate::client::DockerError) -> bool {
    matches!(error, crate::client::DockerError::Api { status, message }
        if *status == reqwest::StatusCode::CONFLICT
            && serde_json::from_str::<serde_json::Value>(message).ok()
                .is_some_and(|value| value["message"].as_str()
                    == Some("can not get logs from container which is dead or marked for removal")))
}

fn normalize_container_log_error(error: crate::client::DockerError) -> RuntimeCapabilityError {
    if is_container_log_removal_conflict(&error) {
        return RuntimeCapabilityError::new(
            RuntimeErrorKind::NotFound,
            "The container was removed or is being removed.",
            false,
        );
    }
    crate::client::runtime::normalize_docker_error(error)
}

impl LogReadPort for crate::client::DockerClient {
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
                    let stream = self.open_resource_logs(resource, false, tail, cancel).await.map_err(crate::client::runtime::normalize_docker_error)?;
                    snapshot(stream).await
                }) => result.map_err(|_|failure("Reading logs timed out."))?,
            }
        })
    }
}

pub fn validate_tail(tail: u16) -> Result<(), RuntimeCapabilityError> {
    if !(1..=200).contains(&tail) {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::InvalidRequest,
            "Tail must be between 1 and 200.",
            false,
        ));
    }
    Ok(())
}

pub fn checked_snapshot(
    lines: Vec<String>,
    truncated: bool,
) -> Result<LogSnapshot, RuntimeCapabilityError> {
    if lines.len() > MAX_LOG_FRAME || lines.iter().map(String::len).sum::<usize>() > MAX_LOG_FRAME {
        return Err(failure("Agent log response exceeds the limit."));
    }
    Ok(LogSnapshot { lines, truncated })
}

pub async fn snapshot(mut stream: RuntimeLogStream) -> Result<LogSnapshot, RuntimeCapabilityError> {
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
pub fn failure(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, message, false)
}

pub fn cancelled() -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(
        RuntimeErrorKind::Cancelled,
        "Log subscription canceled.",
        false,
    )
}

// Docker's non-TTY logs have an eight-byte multiplex header. TTY output is raw.
// Keep only one bounded partial frame; reject invalid/truncated frames visibly.
pub fn decode_frames(
    mut input: futures_util::stream::BoxStream<
        'static,
        Result<bytes::Bytes, crate::client::DockerError>,
    >,
    tty: bool,
    cancel: CancellationToken,
) -> RuntimeLogStream {
    Box::pin(async_stream::try_stream! {
        let mut pending=Vec::new();
        loop {
            let next=tokio::select!{biased;()=cancel.cancelled()=>break,next=input.next()=>next};
            let Some(next)=next else {if !pending.is_empty(){Err(failure("Docker log stream ended in a partial frame."))?;}break;};
            let mut chunk=next.map_err(crate::client::runtime::normalize_docker_error)?;
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
    #[test]
    fn removal_conflict_is_missing_without_hiding_other_failures() {
        for (status, message, expected) in [
            (
                reqwest::StatusCode::NOT_FOUND,
                "No such container: old",
                RuntimeErrorKind::NotFound,
            ),
            (
                reqwest::StatusCode::CONFLICT,
                "can not get logs from container which is dead or marked for removal",
                RuntimeErrorKind::NotFound,
            ),
            (
                reqwest::StatusCode::CONFLICT,
                "another conflict",
                RuntimeErrorKind::Conflict,
            ),
            (
                reqwest::StatusCode::BAD_REQUEST,
                "configured logging driver does not support reading",
                RuntimeErrorKind::InvalidRequest,
            ),
            (
                reqwest::StatusCode::INTERNAL_SERVER_ERROR,
                "daemon unavailable",
                RuntimeErrorKind::Unavailable,
            ),
        ] {
            let error = normalize_container_log_error(crate::client::DockerError::Api {
                status,
                message: serde_json::json!({"message":message}).to_string(),
            });
            assert_eq!(error.kind, expected);
        }
    }
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
