use super::runtime::BuildRuntimeError as BuildFailure;
use citadel_builds::BuildLogSink;
use citadel_execution::SecretRedactor as Redactor;
use citadel_execution::{
    OutputRetention, ProcessChunk, ProcessError, ProcessOutput, ProcessRequest,
};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

const OUTPUT_DRAIN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);

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
            citadel_processes::run(
                request
                    .output(sender)
                    .output_retention(OutputRetention::StreamOnly),
                &cancellation,
            )
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
    let producer_failed = CancellationToken::new();
    let consume = async {
        // Keep the same append future alive across producer termination. Dropping
        // and replaying it could duplicate a batch whose commit was acknowledged late.
        tokio::pin!(consume);
        let result = tokio::select! {
            biased;
            result = &mut consume => result,
            () = async {
                tokio::select! {
                    () = producer_failed.cancelled() => {},
                    () = cancellation.cancelled() => {},
                }
            } => {
                match tokio::time::timeout(OUTPUT_DRAIN_TIMEOUT, &mut consume).await {
                    Ok(Ok(())) => Err(BuildFailure::Process(ProcessError::Cancelled)),
                    Ok(Err(error)) => Err(error),
                    Err(_) => {
                        tracing::warn!("Build output drain timed out; final log delivery may be incomplete. Unacknowledged output will not be replayed.");
                        Err(BuildFailure::Process(ProcessError::Cancelled))
                    }
                }
            }
        };
        if result.is_err() {
            cancellation.cancel();
        }
        result
    };
    let execute = async {
        let result = execution.await;
        if result.is_err() {
            producer_failed.cancel();
        }
        result
    };
    // Always await executor cleanup, even when persistence fails or is cancelled.
    let (execution, consumed) = tokio::join!(execute, consume);
    match (execution, consumed) {
        // Keep producer terminal/cleanup evidence, including the original failure.
        (Err(error), _) if !matches!(error, BuildFailure::Process(ProcessError::Cancelled)) => {
            Err(error)
        }
        (_, Err(error)) => Err(error),
        (result, Ok(())) => result,
    }
}

async fn flush(
    progress: &dyn BuildLogSink,
    batch: &mut Vec<citadel_builds::BuildLog>,
) -> Result<(), BuildFailure> {
    if !batch.is_empty() {
        progress
            .append_batch(batch)
            .await
            .map_err(|_| {
                tracing::warn!("Build log persistence failed; final log delivery may be incomplete. Unacknowledged output will not be replayed.");
                BuildFailure::Validation("Build log persistence failed.".into())
            })?;
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

    struct BlockedLogs(tokio::sync::Notify);
    impl BuildLogSink for BlockedLogs {
        fn append<'a>(
            &'a self,
            _: &'a str,
            _: &'a str,
        ) -> futures_util::future::BoxFuture<'a, Result<(), citadel_builds::BuildError>> {
            Box::pin(async {
                self.0.notify_one();
                std::future::pending().await
            })
        }
    }

    #[tokio::test(start_paused = true)]
    async fn cancellation_or_producer_failure_releases_a_blocked_log_sink() {
        for producer_failed in [false, true] {
            let sink = BlockedLogs(tokio::sync::Notify::new());
            let cancellation = CancellationToken::new();
            let (sender, receiver) = mpsc::channel(1);
            let cleaned = std::sync::atomic::AtomicBool::new(false);
            let execution = async {
                sender
                    .send(ProcessChunk {
                        stream: "stdout",
                        bytes: b"pending log".to_vec(),
                    })
                    .await
                    .unwrap();
                sink.0.notified().await;
                if !producer_failed {
                    cancellation.cancel();
                }
                drop(sender);
                cleaned.store(true, std::sync::atomic::Ordering::SeqCst);
                if producer_failed {
                    Err(BuildFailure::Process(ProcessError::Timeout(
                        std::time::Duration::from_secs(1),
                    )))
                } else {
                    Ok(())
                }
            };
            let result = tokio::time::timeout(
                OUTPUT_DRAIN_TIMEOUT + std::time::Duration::from_secs(1),
                capture(execution, receiver, &sink, &[], &cancellation),
            )
            .await
            .expect("a blocked sink must not retain a finished/cancelled executor");
            assert!(cleaned.load(std::sync::atomic::Ordering::SeqCst));
            if producer_failed {
                assert!(matches!(
                    result,
                    Err(BuildFailure::Process(ProcessError::Timeout(_)))
                ));
            } else {
                assert!(matches!(
                    result,
                    Err(BuildFailure::Process(ProcessError::Cancelled))
                ));
            }
        }
    }

    #[tokio::test(start_paused = true)]
    async fn committed_but_unacknowledged_batch_is_never_replayed() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        struct Unacknowledged {
            commits: AtomicUsize,
            committed: tokio::sync::Notify,
        }
        impl BuildLogSink for Unacknowledged {
            fn append<'a>(
                &'a self,
                _: &'a str,
                _: &'a str,
            ) -> futures_util::future::BoxFuture<'a, Result<(), citadel_builds::BuildError>>
            {
                Box::pin(async move {
                    self.commits.fetch_add(1, Ordering::SeqCst);
                    self.committed.notify_one();
                    std::future::pending().await
                })
            }
        }
        for failed in [false, true] {
            let sink = Unacknowledged {
                commits: AtomicUsize::new(0),
                committed: tokio::sync::Notify::new(),
            };
            let cancellation = CancellationToken::new();
            let (sender, receiver) = mpsc::channel(1);
            let execution = async {
                sender
                    .send(ProcessChunk {
                        stream: "stderr",
                        bytes: b"diagnostic".to_vec(),
                    })
                    .await
                    .unwrap();
                sink.committed.notified().await;
                drop(sender);
                if failed {
                    Err(BuildFailure::Command("terminal failure".into()))
                } else {
                    cancellation.cancel();
                    Ok(())
                }
            };
            let started = tokio::time::Instant::now();
            let result = tokio::time::timeout(
                OUTPUT_DRAIN_TIMEOUT + std::time::Duration::from_secs(1),
                capture(execution, receiver, &sink, &[], &cancellation),
            )
            .await
            .expect("unacknowledged append must have a bounded drain");
            assert!(started.elapsed() >= OUTPUT_DRAIN_TIMEOUT);
            assert!(
                result.is_err(),
                "unacknowledged output must not become success"
            );
            assert_eq!(sink.commits.load(Ordering::SeqCst), 1);
            if failed {
                assert!(
                    matches!(result, Err(BuildFailure::Command(message)) if message == "terminal failure")
                );
            } else {
                assert!(matches!(
                    result,
                    Err(BuildFailure::Process(ProcessError::Cancelled))
                ));
            }
        }
    }

    #[tokio::test]
    async fn persistence_failure_cancels_the_producer_instead_of_hanging() {
        let (sender, receiver) = mpsc::channel(1);
        let cancellation = CancellationToken::new();
        let cleaned = std::sync::atomic::AtomicBool::new(false);
        let execution = async {
            sender
                .send(ProcessChunk {
                    stream: "stdout",
                    bytes: b"progress".to_vec(),
                })
                .await
                .unwrap();
            cancellation.cancelled().await;
            tokio::task::yield_now().await;
            drop(sender);
            cleaned.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        };
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            capture(execution, receiver, &RejectLogs, &[], &cancellation),
        )
        .await
        .unwrap();
        assert!(
            matches!(result, Err(BuildFailure::Validation(message)) if message == "Build log persistence failed.")
        );
        assert!(cancellation.is_cancelled());
        assert!(cleaned.load(std::sync::atomic::Ordering::SeqCst));
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
                tokio::task::yield_now().await;
                self.0.lock().unwrap().push(logs.to_vec());
                Ok(())
            })
        }
    }
    #[tokio::test]
    async fn producer_failure_preserves_diagnostics_with_an_async_sink() {
        let sink = Sink::default();
        let (sender, receiver) = mpsc::channel(8);
        let execution = async move {
            sender
                .send(ProcessChunk {
                    stream: "stderr",
                    bytes: b"compiler diagnostic\n".to_vec(),
                })
                .await
                .unwrap();
            drop(sender);
            Err::<(), _>(BuildFailure::Command("terminal failure".into()))
        };
        let result = capture(execution, receiver, &sink, &[], &CancellationToken::new()).await;
        assert!(
            matches!(result, Err(BuildFailure::Command(message)) if message == "terminal failure")
        );
        let batches = sink.0.lock().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].len(), 1);
        assert_eq!(batches[0][0].message, "compiler diagnostic\n");
    }

    #[tokio::test]
    async fn cancellation_finishes_a_healthy_append_before_returning() {
        let sink = Sink::default();
        let cancellation = CancellationToken::new();
        let (sender, receiver) = mpsc::channel(1);
        let execution = async {
            sender
                .send(ProcessChunk {
                    stream: "stderr",
                    bytes: b"last diagnostic".to_vec(),
                })
                .await
                .unwrap();
            drop(sender);
            cancellation.cancel();
            Err::<(), _>(BuildFailure::Process(ProcessError::Cancelled))
        };
        let result = capture(execution, receiver, &sink, &[], &cancellation).await;
        assert!(matches!(
            result,
            Err(BuildFailure::Process(ProcessError::Cancelled))
        ));
        let batches = sink.0.lock().unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0][0].message, "last diagnostic");
    }

    #[tokio::test]
    async fn burst_batches_preserve_order_redaction_and_flush_a_quiet_stream() {
        let sink = Sink::default();
        let (sender, receiver) = mpsc::channel(8);
        let cancel = CancellationToken::new();
        let execution = async {
            for bytes in [b"first secret".as_slice(), b"-value\n\xc3", b"\xa9 last\n"] {
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
        assert!(output.contains("é last"));
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
    #[cfg(unix)]
    #[tokio::test]
    async fn build_streams_both_pipes_and_returns_only_completion_metadata() {
        let sink = Sink::default();
        let request = ProcessRequest::new("/bin/sh").args([
            "-c",
            "(head -c 2097152 /dev/zero | tr '\\0' x) & (head -c 2097152 /dev/zero | tr '\\0' y >&2) & wait; exit 7",
        ]).limits(super::super::runtime::limits(std::time::Duration::from_secs(10), 1024 * 1024));
        let output = run(request, &CancellationToken::new(), &sink, &[])
            .await
            .unwrap();
        assert_eq!(output.exit_code, Some(7));
        assert!(output.stdout_truncated && output.stderr_truncated);
        assert_eq!(output.stdout.capacity() + output.stderr.capacity(), 0);
        let batches = sink.0.lock().unwrap();
        for (stream, byte) in [("stdout", b'x'), ("stderr", b'y')] {
            let logs: Vec<_> = batches
                .iter()
                .flatten()
                .filter(|log| log.stream == stream)
                .collect();
            assert_eq!(
                logs.iter().map(|log| log.message.len()).sum::<usize>(),
                1024 * 1024
            );
            assert!(
                logs.iter()
                    .all(|log| log.message.bytes().all(|value| value == byte))
            );
        }
    }
}
