use super::runtime::wait_for_work;
use std::time::Duration;
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[tokio::test(start_paused = true)]
async fn idle_copy_lanes_coalesce_wakeups_and_recover_after_loss() {
    let (sender, mut execution) = watch::channel(());
    let mut reconciliation = execution.clone();
    let cancel = CancellationToken::new();
    let start = tokio::time::Instant::now();
    for _ in 0..1000 {
        sender.send_modify(|_| {});
    }
    for wake in [&mut execution, &mut reconciliation] {
        wait_for_work(wake, &cancel, false).await;
        assert_eq!(tokio::time::Instant::now(), start);
    }
    wait_for_work(&mut execution, &cancel, false).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(30));
    drop(sender);
    wait_for_work(&mut reconciliation, &cancel, false).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(60));
    cancel.cancel();
    wait_for_work(&mut execution, &cancel, false).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(60));
}
#[tokio::test(start_paused = true)]
async fn failed_copy_checks_back_off_despite_notifications_and_cancel_promptly() {
    let (sender, mut wake) = watch::channel(());
    let cancel = CancellationToken::new();
    let start = tokio::time::Instant::now();
    sender.send_modify(|_| {});
    wait_for_work(&mut wake, &cancel, true).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(5));
    cancel.cancel();
    wait_for_work(&mut wake, &cancel, true).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(5));
}

#[tokio::test(start_paused = true)]
async fn copy_listener_reports_idle_retry_and_recovery_without_counting_idle_time() {
    use citadel_resource_copies::{CopyChangeSource, CopyError};
    use citadel_runtime::jobs::{JobCatalog, JobOutcome, JobOwnership, JobState};
    struct Changes(tokio::sync::mpsc::UnboundedReceiver<Result<Option<uuid::Uuid>, CopyError>>);
    impl CopyChangeSource for Changes {
        fn next(
            &mut self,
        ) -> futures_util::future::BoxFuture<'_, Result<Option<uuid::Uuid>, CopyError>> {
            Box::pin(async { self.0.recv().await.unwrap() })
        }
    }
    async fn settle() {
        for _ in 0..10 {
            tokio::task::yield_now().await;
        }
    }
    let catalog = JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register(
        "resource-copy-realtime",
        citadel_primitives::JobCategory::ResourceCopy,
        JobOwnership::ProcessLocal,
    );
    let cancel = CancellationToken::new();
    let token = cancel.clone();
    let (send, recv) = tokio::sync::mpsc::unbounded_channel();
    let worker = tokio::spawn(async move {
        job.scope(super::runtime::realtime_updates(
            Box::new(Changes(recv)),
            None,
            token,
        ))
        .await
    });
    settle().await;
    tokio::time::advance(Duration::from_secs(60)).await;
    let idle = catalog.snapshot().jobs.remove(0);
    assert_eq!(idle.state, JobState::Waiting);
    assert_eq!(idle.iterations, 0);
    assert_eq!(idle.last_success_at, None);
    send.send(Err(CopyError::Storage("private database detail".into())))
        .unwrap();
    settle().await;
    let failed = catalog.snapshot().jobs.remove(0);
    assert_eq!(failed.state, JobState::BackingOff);
    assert_eq!(failed.failures, 1);
    assert!(failed.next_check.is_some());
    send.send(Ok(Some(uuid::Uuid::now_v7()))).unwrap();
    tokio::time::advance(Duration::from_secs(2)).await;
    settle().await;
    let recovered = catalog.snapshot().jobs.remove(0);
    assert_eq!(recovered.state, JobState::Waiting);
    assert_eq!(recovered.last_outcome, Some(JobOutcome::Completed));
    assert_eq!(recovered.iterations, 2);
    assert_eq!(recovered.total_duration_ms, 0);
    assert!(
        !recovered
            .last_error
            .unwrap()
            .contains("private database detail")
    );
    cancel.cancel();
    worker.await.unwrap().unwrap();
    assert_eq!(catalog.snapshot().jobs[0].iterations, 2);
}
