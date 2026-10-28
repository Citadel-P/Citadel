//! Forward bounded progress without retaining a second copy of the complete log.
use super::{cancelled_error, normalize_status};
use citadel_contracts::citadel::images::v1::{BuildAdmissionResult, ImageBuildResponse};
use citadel_execution::ProcessChunk;
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::{Stream, StreamExt};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tonic::Status;

#[derive(Debug, Default)]
pub(crate) struct BuildStreamSummary {
    pub not_started: bool,
    pub digest: Option<String>,
    pub log_bytes: usize,
}

impl BuildStreamSummary {
    pub(crate) fn merge(&mut self, next: Self) {
        self.log_bytes = self.log_bytes.saturating_add(next.log_bytes);
        // A pushed manifest digest takes precedence over a build's image ID.
        self.digest = next.digest.or(self.digest.take());
    }
}

// Keep only a possible digest token, including across arbitrary stream chunks.
// Noisy output never grows this buffer beyond a SHA-256 digest's 71 bytes.
#[derive(Default)]
struct DigestReader {
    token: Vec<u8>,
    overflow: bool,
    digest: Option<String>,
}

impl DigestReader {
    fn push(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            if byte.is_ascii_whitespace() || byte == b',' {
                self.finish_token();
            } else if self.token.len() < 71 && !self.overflow {
                self.token.push(byte);
            } else {
                self.overflow = true;
            }
        }
    }

    fn finish_token(&mut self) {
        if !self.overflow
            && self.token.len() == 71
            && self.token.starts_with(b"sha256:")
            && self.token[7..].iter().all(u8::is_ascii_hexdigit)
        {
            self.digest = String::from_utf8(self.token.clone()).ok();
        }
        self.token.clear();
        self.overflow = false;
    }
}

pub(crate) async fn consume_image_build_stream<S>(
    mut stream: S,
    maximum_bytes: usize,
    progress: Option<&mpsc::Sender<ProcessChunk>>,
    cancellation: &CancellationToken,
) -> Result<BuildStreamSummary, RuntimeCapabilityError>
where
    S: Stream<Item = Result<ImageBuildResponse, Status>> + Unpin,
{
    let mut summary = BuildStreamSummary::default();
    let mut text_digest = DigestReader::default();
    let mut received_frame = false;
    let mut completed = false;
    loop {
        let next = tokio::select! {
            biased;
            () = cancellation.cancelled() => return Err(cancelled_error()),
            item = stream.next() => item,
        };
        let Some(item) = next else { break };
        let item = item.map_err(normalize_status)?;
        if summary.not_started {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                "Unexpected output after build admission refusal.",
                false,
            ));
        }
        if matches!(
            BuildAdmissionResult::try_from(item.admission),
            Ok(BuildAdmissionResult::BuildAdmissionCapacityBusyNotStarted
                | BuildAdmissionResult::BuildAdmissionFencedNotStarted)
        ) {
            if received_frame
                || item.stream.is_some()
                || item.id.is_some()
                || item.status.is_some()
                || item.error_message.is_some()
                || item.error.is_some()
                || item.progress.is_some()
                || item.progress_message.is_some()
            {
                return Err(RuntimeCapabilityError::new(
                    RuntimeErrorKind::Remote,
                    "Invalid late build admission refusal; execution will not be replayed.",
                    false,
                ));
            }
            summary.not_started = true;
            continue;
        }
        received_frame = true;
        completed |= item.status.as_deref() == Some("completed");

        let error = item
            .error_message
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| {
                item.error
                    .as_ref()
                    .and_then(|error| error.message.as_deref())
                    .filter(|value| !value.trim().is_empty())
            });
        if let Some(error) = error {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Remote,
                error.to_owned(),
                false,
            ));
        }
        if let Some(id) = item.id {
            let mut structured = DigestReader::default();
            structured.push(id.as_bytes());
            structured.finish_token();
            summary.digest = structured.digest.or(summary.digest);
        }
        for (index, value) in [item.stream, item.status, item.progress_message]
            .into_iter()
            .enumerate()
        {
            let Some(value) = value else { continue };
            if index != 0 {
                text_digest.finish_token();
            }
            text_digest.push(value.as_bytes());
            if index != 0 {
                text_digest.finish_token();
            }
            let retained = value
                .len()
                .min(maximum_bytes.saturating_sub(summary.log_bytes));
            if let Some(progress) = progress {
                for bytes in value.as_bytes()[..retained].chunks(8192) {
                    tokio::select! {
                        biased;
                        () = cancellation.cancelled() => return Err(cancelled_error()),
                        sent = progress.send(ProcessChunk { stream: "stdout", bytes: bytes.to_vec() }) => {
                            sent.map_err(|_| RuntimeCapabilityError::new(RuntimeErrorKind::Remote, "Build output consumer closed.", false))?;
                        }
                    }
                }
            }
            summary.log_bytes += retained;
        }
    }
    if !summary.not_started && !completed {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Remote,
            "Build stream closed without a completion receipt; execution will not be replayed.",
            false,
        ));
    }
    text_digest.finish_token();
    summary.digest = summary.digest.or(text_digest.digest);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn only_a_complete_pre_start_busy_response_is_replayable() {
        let busy = ImageBuildResponse {
            admission: BuildAdmissionResult::BuildAdmissionCapacityBusyNotStarted as i32,
            ..Default::default()
        };
        let accepted = ImageBuildResponse {
            admission: BuildAdmissionResult::BuildAdmissionAccepted as i32,
            ..Default::default()
        };
        let result = consume_image_build_stream(
            futures_util::stream::iter([Ok(busy.clone())]),
            0,
            None,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert!(result.not_started);
        for frames in [
            vec![],
            vec![accepted.clone()],
            vec![accepted.clone(), busy.clone()],
            vec![busy, accepted],
        ] {
            assert!(
                consume_image_build_stream(
                    futures_util::stream::iter(frames.into_iter().map(Ok)),
                    0,
                    None,
                    &CancellationToken::new()
                )
                .await
                .is_err()
            );
        }
        // A generic transport refusal carries no evidence about execution.
        assert!(
            consume_image_build_stream(
                futures_util::stream::iter([Err(Status::resource_exhausted("busy"))]),
                0,
                None,
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn admission_fence_is_replayable_only_before_any_execution_output() {
        let fenced = ImageBuildResponse {
            admission: BuildAdmissionResult::BuildAdmissionFencedNotStarted as i32,
            ..Default::default()
        };
        let accepted = ImageBuildResponse {
            admission: BuildAdmissionResult::BuildAdmissionAccepted as i32,
            ..Default::default()
        };
        for (frames, allowed) in [
            (vec![fenced.clone()], true),
            (vec![accepted.clone(), fenced.clone()], false),
            (vec![fenced, accepted], false),
        ] {
            let result = consume_image_build_stream(
                futures_util::stream::iter(frames.into_iter().map(Ok)),
                0,
                None,
                &CancellationToken::new(),
            )
            .await;
            assert_eq!(result.is_ok_and(|value| value.not_started), allowed);
        }
    }

    #[tokio::test]
    async fn noisy_output_is_bounded_and_digest_survives_log_truncation() {
        let digest = format!("sha256:{}", "a".repeat(64));
        let (progress, mut receiver) = mpsc::channel(1);
        let responses = vec![
            Ok(ImageBuildResponse {
                stream: Some("x".repeat(1024 * 1024)),
                ..Default::default()
            }),
            Ok(ImageBuildResponse {
                stream: Some(format!("\n{}", &digest[..30])),
                ..Default::default()
            }),
            Ok(ImageBuildResponse {
                stream: Some(digest[30..].into()),
                ..Default::default()
            }),
            Ok(ImageBuildResponse {
                status: Some("completed".into()),
                ..Default::default()
            }),
        ];
        let summary = consume_image_build_stream(
            futures_util::stream::iter(responses),
            32,
            Some(&progress),
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(summary.digest, Some(digest));
        assert_eq!(summary.log_bytes, 32);
        assert_eq!(receiver.recv().await.unwrap().bytes.len(), 32);
        assert!(receiver.try_recv().is_err());
    }

    #[tokio::test]
    async fn structured_digest_wins_and_late_failure_is_not_success() {
        let digest = format!("sha256:{}", "b".repeat(64));
        let response = ImageBuildResponse {
            status: Some("completed".into()),
            id: Some(digest.clone()),
            stream: Some(format!("sha256:{}", "a".repeat(64))),
            ..Default::default()
        };
        let summary = consume_image_build_stream(
            futures_util::stream::iter([Ok(response.clone())]),
            0,
            None,
            &CancellationToken::new(),
        )
        .await
        .unwrap();
        assert_eq!(summary.digest, Some(digest));
        let failure = ImageBuildResponse {
            error_message: Some("push failed".into()),
            ..Default::default()
        };
        assert!(
            consume_image_build_stream(
                futures_util::stream::iter([Ok(response), Ok(failure)]),
                0,
                None,
                &CancellationToken::new()
            )
            .await
            .is_err()
        );
    }

    #[tokio::test]
    async fn slow_log_consumer_can_be_cancelled() {
        let (progress, _receiver) = mpsc::channel(1);
        let token = CancellationToken::new();
        let response = ImageBuildResponse {
            stream: Some("x".repeat(32 * 1024)),
            ..Default::default()
        };
        let pending = consume_image_build_stream(
            futures_util::stream::iter([Ok(response)]),
            32 * 1024,
            Some(&progress),
            &token,
        );
        tokio::pin!(pending);
        assert!(
            tokio::time::timeout(std::time::Duration::from_millis(10), &mut pending)
                .await
                .is_err()
        );
        token.cancel();
        assert!(pending.await.is_err());
    }
}
