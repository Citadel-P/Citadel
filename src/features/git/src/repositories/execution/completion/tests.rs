use super::*;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

fn reference(status: crate::GitRepositoryRefStatus) -> GitRepositoryRef {
    GitRepositoryRef {
        id: Uuid::nil(),
        git_repository_id: Uuid::nil(),
        branch: "main".into(),
        resolved_commit_sha: Some("a".repeat(40)),
        status,
        last_error: None,
        last_synced_at: Utc::now(),
    }
}

#[tokio::test(start_paused = true)]
async fn five_second_sync_with_multiple_waiters_reads_only_initial_and_completion_state() {
    let (signal, _) = watch::channel(());
    let healthy = Arc::new(AtomicBool::new(false));
    let reads = Arc::new(AtomicUsize::new(0));
    let mut tasks = Vec::new();
    for _ in 0..8 {
        let receiver = signal.subscribe();
        let healthy = healthy.clone();
        let reads = reads.clone();
        tasks.push(tokio::spawn(async move {
            wait(receiver, || {
                reads.fetch_add(1, Ordering::SeqCst);
                let status = if healthy.load(Ordering::SeqCst) {
                    crate::GitRepositoryRefStatus::Healthy
                } else {
                    crate::GitRepositoryRefStatus::Syncing
                };
                std::future::ready(Ok(Some(reference(status))))
            })
            .await
        }));
    }
    tokio::task::yield_now().await;
    for _ in 0..20 {
        tokio::time::advance(Duration::from_millis(250)).await;
        tokio::task::yield_now().await;
    }
    assert_eq!(reads.load(Ordering::SeqCst), 8);
    healthy.store(true, Ordering::SeqCst);
    signal.send_modify(|_| {});
    for task in tasks {
        assert_eq!(task.await.unwrap().unwrap(), "a".repeat(40));
    }
    assert_eq!(reads.load(Ordering::SeqCst), 16);
}

#[tokio::test(start_paused = true)]
async fn missed_or_closed_signal_recovers_at_ten_seconds_without_spinning() {
    for close in [false, true] {
        let (signal, receiver) = watch::channel(());
        let retained = if close { None } else { Some(signal) };
        let reads = Arc::new(AtomicUsize::new(0));
        let observed = reads.clone();
        let start = tokio::time::Instant::now();
        let task = tokio::spawn(async move {
            wait(receiver, || {
                let n = observed.fetch_add(1, Ordering::SeqCst);
                std::future::ready(Ok(Some(reference(if n == 0 {
                    crate::GitRepositoryRefStatus::Pending
                } else {
                    crate::GitRepositoryRefStatus::Healthy
                }))))
            })
            .await
        });
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(5)).await;
        tokio::task::yield_now().await;
        assert_eq!(reads.load(Ordering::SeqCst), 1);
        assert_eq!(task.await.unwrap().unwrap(), "a".repeat(40));
        assert_eq!(tokio::time::Instant::now() - start, FALLBACK);
        assert_eq!(reads.load(Ordering::SeqCst), 2);
        drop(retained);
    }
}

#[tokio::test(start_paused = true)]
async fn completion_during_read_is_not_lost_and_hint_never_replaces_authoritative_state() {
    let (signal, receiver) = watch::channel(());
    let mut reads = 0;
    let start = tokio::time::Instant::now();
    let result = wait(receiver, || {
        reads += 1;
        if reads == 1 {
            signal.send_modify(|_| {});
        }
        std::future::ready(Ok(Some(reference(if reads < 3 {
            crate::GitRepositoryRefStatus::Pending
        } else {
            crate::GitRepositoryRefStatus::Healthy
        }))))
    })
    .await
    .unwrap();
    assert_eq!(result, "a".repeat(40));
    assert_eq!(reads, 3);
    assert_eq!(tokio::time::Instant::now() - start, FALLBACK);
}

#[tokio::test(start_paused = true)]
async fn cancellation_and_timeout_interrupt_even_a_hung_enqueue_or_read() {
    let token = CancellationToken::new();
    let other = token.clone();
    let task = tokio::spawn(async move { bounded::<()>(&other, std::future::pending()).await });
    tokio::task::yield_now().await;
    let start = tokio::time::Instant::now();
    token.cancel();
    assert!(matches!(
        task.await.unwrap(),
        Err(GitRepositoryExecutionError::Git(GitError::Process(
            citadel_execution::ProcessError::Cancelled
        )))
    ));
    assert_eq!(tokio::time::Instant::now(), start);
    let error = bounded::<()>(&CancellationToken::new(), std::future::pending()).await;
    assert!(matches!(
        error,
        Err(GitRepositoryExecutionError::Git(GitError::Process(
            citadel_execution::ProcessError::Timeout(_)
        )))
    ));
    assert_eq!(tokio::time::Instant::now() - start, TIMEOUT);
}

#[tokio::test]
async fn terminal_failures_missing_refs_and_invalid_commits_still_fail() {
    let (signal, _) = watch::channel(());
    for status in [
        crate::GitRepositoryRefStatus::Degraded,
        crate::GitRepositoryRefStatus::Healthy,
    ] {
        let mut row = reference(status);
        if status == crate::GitRepositoryRefStatus::Healthy {
            row.resolved_commit_sha = Some("invalid".into());
        }
        assert!(
            wait(signal.subscribe(), || std::future::ready(Ok(Some(
                row.clone()
            ))))
            .await
            .is_err()
        );
    }
    assert!(matches!(
        wait(signal.subscribe(), || std::future::ready(Ok(None))).await,
        Err(GitRepositoryExecutionError::NotFound)
    ));
}
