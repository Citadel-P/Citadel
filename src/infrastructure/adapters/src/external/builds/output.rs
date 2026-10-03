use super::runtime::BuildRuntimeError as BuildFailure;
use citadel_builds::BuildLogSink;
use citadel_execution::SecretRedactor as Redactor;
use citadel_execution::{ProcessChunk, ProcessOutput, ProcessRequest};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run(
    request: ProcessRequest,
    cancellation: &CancellationToken,
    progress: &dyn BuildLogSink,
    secrets: &[&str],
) -> Result<ProcessOutput, BuildFailure> {
    let cancellation = cancellation.child_token();
    let (sender, receiver) = mpsc::channel(8);
    capture(
        async {
            citadel_processes::run(request.output(sender), &cancellation)
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

pub(crate) async fn capture<T>(
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

#[cfg(test)]
mod tests {
    use super::*;
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
