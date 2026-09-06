use super::BuildFailure;
use citadel_builds::BuildLogSink;
use citadel_execution::{ProcessChunk, ProcessOutput, ProcessRequest};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub(super) async fn run(
    request: ProcessRequest,
    cancellation: &CancellationToken,
    progress: &dyn BuildLogSink,
    secrets: &[&str],
) -> Result<ProcessOutput, BuildFailure> {
    let cancellation = cancellation.child_token();
    let (sender, receiver) = mpsc::channel(8);
    capture(
        async {
            citadel_execution::run(request.output(sender), &cancellation)
                .await
                .map_err(BuildFailure::Process)
        },
        receiver,
        progress,
        secrets,
        &cancellation,
    )
    .await
}

pub(super) async fn capture<T>(
    execution: impl Future<Output = Result<T, BuildFailure>>,
    mut receiver: mpsc::Receiver<ProcessChunk>,
    progress: &dyn BuildLogSink,
    secrets: &[&str],
    cancellation: &CancellationToken,
) -> Result<T, BuildFailure> {
    let consume = async {
        let mut stdout = Redactor::new(secrets);
        let mut stderr = Redactor::new(secrets);
        while let Some(chunk) = receiver.recv().await {
            let redactor = if chunk.stream == "stderr" {
                &mut stderr
            } else {
                &mut stdout
            };
            let message = redactor.push(&chunk.bytes, false);
            if !message.is_empty() {
                progress.append(chunk.stream, &message).await.map_err(|_| {
                    BuildFailure::Validation("Build log persistence failed.".into())
                })?;
            }
        }
        for (stream, redactor) in [("stdout", &mut stdout), ("stderr", &mut stderr)] {
            let message = redactor.push(&[], true);
            if !message.is_empty() {
                progress.append(stream, &message).await.map_err(|_| {
                    BuildFailure::Validation("Build log persistence failed.".into())
                })?;
            }
        }
        Ok::<_, BuildFailure>(())
    };
    let consume = async {
        let result = consume.await;
        if result.is_err() {
            cancellation.cancel();
        }
        result
    };
    let (execution, consumed) = tokio::join!(execution, consume);
    consumed?;
    execution
}

/// Keeps only a possible secret prefix and an incomplete UTF-8 character
/// between chunks. Never publish a prefix that could complete in the next read.
struct Redactor<'a> {
    secrets: &'a [&'a str],
    pending: Vec<u8>,
}
impl<'a> Redactor<'a> {
    fn new(secrets: &'a [&'a str]) -> Self {
        Self {
            secrets,
            pending: Vec::new(),
        }
    }
    fn push(&mut self, input: &[u8], finish: bool) -> String {
        self.pending.extend_from_slice(input);
        let limit = if !finish {
            match std::str::from_utf8(&self.pending) {
                Err(error) if error.error_len().is_none() => error.valid_up_to(),
                _ => self.pending.len(),
            }
        } else {
            self.pending.len()
        };
        let mut output = Vec::new();
        let mut offset = 0;
        while offset < limit {
            let rest = &self.pending[offset..];
            if self.secrets.iter().any(|secret| {
                !secret.is_empty()
                    && secret.len() > rest.len()
                    && secret.as_bytes().starts_with(rest)
            }) {
                if finish {
                    output.extend_from_slice(b"[redacted]");
                    offset = self.pending.len();
                }
                break;
            }
            if let Some(length) = self
                .secrets
                .iter()
                .filter(|secret| !secret.is_empty() && rest.starts_with(secret.as_bytes()))
                .map(|secret| secret.len())
                .max()
            {
                output.extend_from_slice(b"[redacted]");
                offset += length;
            } else {
                output.push(self.pending[offset]);
                offset += 1;
            }
        }
        self.pending.drain(..offset);
        String::from_utf8_lossy(&output).into_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn redacts_secrets_across_every_chunk_boundary_and_preserves_utf8() {
        let input = "hello 🔐 token-long-value and multi\nline value".as_bytes();
        let secrets = ["token", "token-long-value", "multi\nline"];
        for split in 0..=input.len() {
            let mut redactor = Redactor::new(&secrets);
            let mut output = redactor.push(&input[..split], false);
            output.push_str(&redactor.push(&input[split..], false));
            output.push_str(&redactor.push(&[], true));
            assert_eq!(
                output, "hello 🔐 [redacted] and [redacted] value",
                "split {split}"
            );
        }
    }
    #[test]
    fn retains_only_a_possible_secret_prefix() {
        let secrets = ["secret-value"];
        let mut redactor = Redactor::new(&secrets);
        assert_eq!(
            redactor.push(b"ordinary output sec", false),
            "ordinary output "
        );
        assert_eq!(redactor.pending, b"sec");
        assert_eq!(redactor.push(b"ret-value!", true), "[redacted]!");
        assert!(redactor.pending.is_empty());
    }

    struct RejectLogs;
    impl BuildLogSink for RejectLogs {
        fn append<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> futures_util::future::BoxFuture<'a, Result<(), citadel_builds::BuildError>> {
            Box::pin(async {
                Err(citadel_builds::BuildError::Storage(
                    "fixture failure".into(),
                ))
            })
        }
    }

    #[tokio::test]
    async fn persistence_failure_cancels_the_producer_instead_of_hanging() {
        let (sender, receiver) = mpsc::channel(1);
        let cancellation = CancellationToken::new();
        let execution = async {
            sender
                .send(ProcessChunk {
                    stream: "stdout",
                    bytes: b"progress".to_vec(),
                })
                .await
                .unwrap();
            cancellation.cancelled().await;
            drop(sender);
            Ok(())
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            capture(execution, receiver, &RejectLogs, &[], &cancellation),
        )
        .await
        .unwrap();
        assert!(result.is_err());
        assert!(cancellation.is_cancelled());
    }
}
