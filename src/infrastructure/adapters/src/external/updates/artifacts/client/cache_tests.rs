use super::*;
use crate::external::updates::artifacts::verification::AGENT_VERIFICATION_TTL;
use citadel_execution::{ProcessError, ProcessOutput, ProcessRequest};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

#[derive(Default)]
struct Runner {
    calls: AtomicUsize,
    fail: AtomicBool,
    pause: AtomicBool,
    started: tokio::sync::Notify,
    finish: tokio::sync::Notify,
}
impl ProcessRunner for Runner {
    fn run<'a>(
        &'a self,
        request: ProcessRequest,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<ProcessOutput, ProcessError>> {
        Box::pin(async move {
            self.calls.fetch_add(1, Ordering::SeqCst);
            self.started.notify_one();
            if self.pause.load(Ordering::SeqCst) {
                tokio::select! {
                    biased;
                    () = cancel.cancelled() => return Err(ProcessError::Cancelled),
                    () = self.finish.notified() => {},
                }
            }
            let image = request.arguments.last().unwrap().to_str().unwrap();
            let digest = image.split_once('@').unwrap().1;
            Ok(ProcessOutput {
                exit_code: Some(if self.fail.load(Ordering::SeqCst) {
                    1
                } else {
                    0
                }),
                stdout: serde_json::to_vec(
                    &serde_json::json!([{"critical":{"image":{"docker-manifest-digest":digest}}}]),
                )
                .unwrap(),
                stderr: vec![],
                stdout_truncated: false,
                stderr_truncated: false,
            })
        })
    }
}
fn image(digit: char) -> String {
    format!(
        "ghcr.io/citadel-p/citadel.agent@sha256:{}",
        digit.to_string().repeat(64)
    )
}

#[tokio::test(start_paused = true)]
async fn agent_cache_is_bounded_keyed_by_version_and_image_and_expires() {
    let runner = Arc::new(Runner::default());
    let source = GithubReleaseArtifacts::new(runner.clone()).unwrap();
    let cancel = CancellationToken::new();
    let a = image('a');
    let b = image('b');
    for _ in 0..3 {
        source
            .verify_agent_image("1.2.3", &a, &cancel)
            .await
            .unwrap();
    }
    assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
    tokio::time::advance(AGENT_VERIFICATION_TTL).await;
    source
        .verify_agent_image("1.2.3", &a, &cancel)
        .await
        .unwrap();
    assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
    source
        .verify_agent_image("1.2.3", &b, &cancel)
        .await
        .unwrap();
    source
        .verify_agent_image("1.2.4", &b, &cancel)
        .await
        .unwrap();
    source
        .verify_agent_image("1.2.3", &a, &cancel)
        .await
        .unwrap();
    assert_eq!(
        runner.calls.load(Ordering::SeqCst),
        5,
        "only the latest success is retained"
    );
    // Core review/approval's verifier always executes, even with a matching cached key.
    source.verify_image("1.2.3", &a, &cancel).await.unwrap();
    source.verify_image("1.2.3", &a, &cancel).await.unwrap();
    assert_eq!(runner.calls.load(Ordering::SeqCst), 7);
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        source.verify_agent_image("1.2.3", &a, &cancelled).await,
        Err(UpdateError::Cancelled)
    ));
    assert_eq!(runner.calls.load(Ordering::SeqCst), 7);
}

#[tokio::test(start_paused = true)]
async fn failed_refresh_does_not_serve_old_evidence_or_cache_failures() {
    let runner = Arc::new(Runner::default());
    let source = GithubReleaseArtifacts::new(runner.clone()).unwrap();
    let cancel = CancellationToken::new();
    let image = image('a');
    source
        .verify_agent_image("1.2.3", &image, &cancel)
        .await
        .unwrap();
    tokio::time::advance(AGENT_VERIFICATION_TTL).await;
    runner.fail.store(true, Ordering::SeqCst);
    for _ in 0..2 {
        assert!(
            source
                .verify_agent_image("1.2.3", &image, &cancel)
                .await
                .is_err()
        );
    }
    runner.fail.store(false, Ordering::SeqCst);
    source
        .verify_agent_image("1.2.3", &image, &cancel)
        .await
        .unwrap();
    source
        .verify_agent_image("1.2.3", &image, &cancel)
        .await
        .unwrap();
    assert_eq!(runner.calls.load(Ordering::SeqCst), 4);
}

#[tokio::test]
async fn concurrent_checks_share_verification_and_lock_waits_are_cancellable() {
    let runner = Arc::new(Runner::default());
    runner.pause.store(true, Ordering::SeqCst);
    let source = GithubReleaseArtifacts::new(runner.clone()).unwrap();
    let cancel = CancellationToken::new();
    let image = image('a');
    let first = source.verify_agent_image("1.2.3", &image, &cancel);
    tokio::pin!(first);
    tokio::select! { _ = runner.started.notified() => {}, _ = &mut first => panic!("must wait") }
    let waiting_cancel = CancellationToken::new();
    let waiting = source.verify_agent_image("1.2.3", &image, &waiting_cancel);
    tokio::pin!(waiting);
    tokio::select! {
        biased;
        _ = &mut waiting => panic!("must wait for the verifier"),
        () = std::future::ready(()) => {},
    }
    waiting_cancel.cancel();
    assert!(matches!(waiting.await, Err(UpdateError::Cancelled)));
    let second = source.verify_agent_image("1.2.3", &image, &cancel);
    runner.finish.notify_one();
    let (a, b) = tokio::join!(first, second);
    a.unwrap();
    b.unwrap();
    assert_eq!(runner.calls.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn cancelled_verification_is_not_cached_and_a_retry_can_run() {
    let runner = Arc::new(Runner::default());
    runner.pause.store(true, Ordering::SeqCst);
    let source = GithubReleaseArtifacts::new(runner.clone()).unwrap();
    let cancel = CancellationToken::new();
    let image = image('a');
    let attempt = source.verify_agent_image("1.2.3", &image, &cancel);
    tokio::pin!(attempt);
    tokio::select! { _ = runner.started.notified() => {}, _ = &mut attempt => panic!("must wait") }
    cancel.cancel();
    assert!(matches!(attempt.await, Err(UpdateError::Cancelled)));
    runner.pause.store(false, Ordering::SeqCst);
    source
        .verify_agent_image("1.2.3", &image, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(runner.calls.load(Ordering::SeqCst), 2);
}
