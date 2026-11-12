use super::runtime::BuildRuntimeError as BuildFailure;
use citadel_builds::BuildLogSink;
use citadel_execution::SecretRedactor as Redactor;
use citadel_execution::{ProcessChunk, ProcessOutput, ProcessRequest};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub async fn run(
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

pub async fn capture<T>(
    execution: impl Future<Output = Result<T, BuildFailure>>,
    mut receiver: mpsc::Receiver<ProcessChunk>,
    progress: &dyn BuildLogSink,
    secrets: &[&str],
    cancellation: &CancellationToken,
) -> Result<T, BuildFailure> {
    let consume = async {
        let mut stdout = Redactor::new(secrets);
        let mut stderr = Redactor::new(secrets);
        let mut batch = Vec::new();
        let mut bytes = 0usize;
        let mut deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
        loop {
            let chunk = tokio::select! {
                chunk = receiver.recv() => chunk,
                () = tokio::time::sleep_until(deadline), if !batch.is_empty() => {
                    flush(progress, &mut batch).await?; bytes = 0;
                    continue;
                }
            };
            let Some(chunk) = chunk else {
                break;
            };
            let redactor = if chunk.stream == "stderr" {
                &mut stderr
            } else {
                &mut stdout
            };
            let message = redactor.push(&chunk.bytes, false);
            enqueue(
                progress,
                &mut batch,
                &mut bytes,
                &mut deadline,
                chunk.stream,
                &message,
            )
            .await?;
        }
        for (stream, redactor) in [("stdout", &mut stdout), ("stderr", &mut stderr)] {
            let message = redactor.push(&[], true);
            enqueue(
                progress,
                &mut batch,
                &mut bytes,
                &mut deadline,
                stream,
                &message,
            )
            .await?;
        }
        flush(progress, &mut batch).await?;
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

async fn flush(
    progress: &dyn BuildLogSink,
    batch: &mut Vec<citadel_builds::BuildLog>,
) -> Result<(), BuildFailure> {
    if !batch.is_empty() {
        progress
            .append_batch(batch)
            .await
            .map_err(|_| BuildFailure::Validation("Build log persistence failed.".into()))?;
        batch.clear();
    }
    Ok(())
}
async fn enqueue(
    progress: &dyn BuildLogSink,
    batch: &mut Vec<citadel_builds::BuildLog>,
    bytes: &mut usize,
    deadline: &mut tokio::time::Instant,
    stream: &str,
    mut message: &str,
) -> Result<(), BuildFailure> {
    while !message.is_empty() {
        let end = message.floor_char_boundary(message.len().min(8192));
        let cost = end + 128;
        if *bytes + cost > 64 * 1024 || batch.len() >= 128 {
            flush(progress, batch).await?;
            *bytes = 0;
        }
        if batch.is_empty() {
            *deadline = tokio::time::Instant::now() + std::time::Duration::from_millis(250);
        }
        batch.push(citadel_builds::BuildLog {
            stream: stream.into(),
            message: message[..end].into(),
        });
        *bytes += cost;
        message = &message[end..];
    }
    Ok(())
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

#[cfg(test)]
mod batching_tests {
    use super::*;
    use std::sync::Mutex;
    #[derive(Default)]
    struct Sink(Mutex<Vec<Vec<citadel_builds::BuildLog>>>);
    impl BuildLogSink for Sink {
        fn append<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> futures_util::future::BoxFuture<'a, Result<(), citadel_builds::BuildError>> {
            Box::pin(async { panic!("capture must use the batch sink") })
        }
        fn append_batch<'a>(
            &'a self,
            logs: &'a [citadel_builds::BuildLog],
        ) -> futures_util::future::BoxFuture<'a, Result<(), citadel_builds::BuildError>> {
            Box::pin(async move {
                self.0.lock().unwrap().push(logs.to_vec());
                Ok(())
            })
        }
    }
    #[tokio::test]
    async fn burst_batches_preserve_order_redaction_and_flush_a_quiet_stream() {
        let sink = Sink::default();
        let (sender, receiver) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let execution = async {
            for bytes in [b"first secret".as_slice(), b"-value\n", b"last\n"] {
                sender
                    .send(ProcessChunk {
                        stream: "stdout",
                        bytes: bytes.to_vec(),
                    })
                    .await
                    .unwrap();
            }
            tokio::time::sleep(std::time::Duration::from_millis(400)).await;
            assert!(
                !sink.0.lock().unwrap().is_empty(),
                "quiet output must flush before EOF"
            );
            drop(sender);
            Ok(())
        };
        capture(execution, receiver, &sink, &["secret-value"], &cancel)
            .await
            .unwrap();
        let batches = sink.0.lock().unwrap();
        let output: String = batches
            .iter()
            .flatten()
            .map(|log| log.message.as_str())
            .collect();
        assert!(output.starts_with("first ") && output.ends_with("last\n"));
        assert!(!output.contains("secret-value"));
        assert!(batches.len() < 3, "the burst must coalesce");
        assert!(batches.iter().all(|batch| {
            batch.len() <= 128
                && batch
                    .iter()
                    .map(|log| log.message.len() + 128)
                    .sum::<usize>()
                    <= 65536
        }));
    }
    #[tokio::test]
    async fn large_unicode_output_is_split_on_char_boundaries_and_byte_budget() {
        let sink = Sink::default();
        let mut batch = Vec::new();
        let mut bytes = 0;
        let mut deadline = tokio::time::Instant::now();
        let message = "é".repeat(100_000);
        enqueue(
            &sink,
            &mut batch,
            &mut bytes,
            &mut deadline,
            "stderr",
            &message,
        )
        .await
        .unwrap();
        flush(&sink, &mut batch).await.unwrap();
        let batches = sink.0.lock().unwrap();
        assert!(batches.len() > 1);
        assert!(batches.iter().all(|batch| {
            batch
                .iter()
                .map(|log| log.message.len() + 128)
                .sum::<usize>()
                <= 65536
        }));
        assert_eq!(
            batches
                .iter()
                .flatten()
                .map(|log| log.message.as_str())
                .collect::<String>(),
            message
        );
    }
}
