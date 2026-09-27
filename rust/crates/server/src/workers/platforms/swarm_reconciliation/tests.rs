use super::super::scoped_reconciler::{event_refresh_channels, run_refresh_workers};
use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

async fn settle() {
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
}
async fn debounce() {
    settle().await;
    tokio::time::advance(DEBOUNCE).await;
    settle().await;
}

#[tokio::test(start_paused = true)]
async fn hundred_swarm_events_reconcile_once_without_standalone_work() {
    use citadel_platforms::jobs::{
        DeltaOutcome, ReconciliationDecision, RuntimeEventKind, SwarmResource, event_decision,
    };
    let cancel = CancellationToken::new();
    let (sender, receivers) = event_refresh_channels(1);
    let calls = Arc::new(Mutex::new(Vec::new()));
    let writes = Arc::new(AtomicUsize::new(0));
    let (reads, commits, token) = (calls.clone(), writes.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_refresh_workers(
            &token,
            receivers,
            Duration::from_secs(5),
            |key| {
                reads.lock().unwrap().push(key);
                async { Ok::<_, &str>(Some(())) }
            },
            |_, ()| {
                commits.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    let id = Uuid::now_v7();
    for i in 0..100 {
        let resource = [
            SwarmResource::Service,
            SwarmResource::Node,
            SwarmResource::Network,
            SwarmResource::Secret,
            SwarmResource::Config,
        ][i % 5];
        assert_eq!(
            event_decision(
                RuntimeEventKind::SwarmDirty(resource),
                true,
                true,
                DeltaOutcome::Pending
            ),
            ReconciliationDecision::SwarmDirty
        );
        sender
            .send(
                ScopedEventRequest {
                    platform_id: id,
                    refresh: EventRefresh::Swarm,
                },
                &cancel,
            )
            .await
            .unwrap();
    }
    settle().await;
    assert!(calls.lock().unwrap().is_empty());
    debounce().await;
    assert_eq!(
        *calls.lock().unwrap(),
        vec![ScopedEventRequest {
            platform_id: id,
            refresh: EventRefresh::Swarm
        }]
    );
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    tokio::time::advance(Duration::from_secs(30)).await;
    settle().await;
    assert_eq!(calls.lock().unwrap().len(), 1);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn in_flight_burst_discards_read_and_retains_one_follow_up() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = channel(1);
    let calls = Arc::new(AtomicUsize::new(0));
    let commits = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Notify::new());
    let (reads, writes, wake, token) = (
        calls.clone(),
        commits.clone(),
        release.clone(),
        cancel.clone(),
    );
    let worker = tokio::spawn(async move {
        run(
            &token,
            receiver,
            Duration::from_secs(5),
            &|_| {
                let pass = reads.fetch_add(1, Ordering::SeqCst);
                let wake = wake.clone();
                async move {
                    if pass == 0 {
                        wake.notified().await;
                    }
                    Ok::<_, &str>(Some(pass))
                }
            },
            &|_, pass| {
                assert_eq!(pass, 1);
                writes.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    let id = Uuid::now_v7();
    sender.mark(id, &cancel).await.unwrap();
    debounce().await;
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for _ in 0..100 {
        sender.mark(id, &cancel).await.unwrap();
    }
    release.notify_one();
    settle().await;
    assert_eq!(commits.load(Ordering::SeqCst), 0);
    debounce().await;
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(commits.load(Ordering::SeqCst), 1);
    assert!(sender.0.state.lock().unwrap().pending.is_empty());
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn new_platforms_wait_at_capacity_but_duplicates_never_wait() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = channel(1);
    let id = Uuid::now_v7();
    sender.mark(id, &cancel).await.unwrap();
    let (other, token) = (sender.clone(), cancel.clone());
    let blocked = tokio::spawn(async move { other.mark(Uuid::now_v7(), &token).await });
    settle().await;
    assert!(!blocked.is_finished());
    for _ in 0..100 {
        sender.mark(id, &cancel).await.unwrap();
    }
    assert_eq!(sender.0.state.lock().unwrap().pending.len(), 1);
    drop(receiver);
    assert_eq!(blocked.await.unwrap(), Err(()));
    assert!(sender.0.state.lock().unwrap().pending.is_empty());
}

#[tokio::test(start_paused = true)]
async fn bounded_parallelism_and_shutdown_drop_active_reads() {
    struct Guard(Arc<AtomicUsize>);
    impl Drop for Guard {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::SeqCst);
        }
    }
    let cancel = CancellationToken::new();
    let (sender, receiver) = channel(10);
    let active = Arc::new(AtomicUsize::new(0));
    let (reads, token) = (active.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run(
            &token,
            receiver,
            Duration::from_secs(5),
            &|_| {
                let reads = reads.clone();
                async move {
                    reads.fetch_add(1, Ordering::SeqCst);
                    let _guard = Guard(reads);
                    std::future::pending::<()>().await;
                    Ok::<_, &str>(Some(()))
                }
            },
            &|_, ()| async { Ok::<_, &str>(true) },
        )
        .await;
    });
    for _ in 0..10 {
        sender.mark(Uuid::now_v7(), &cancel).await.unwrap();
    }
    debounce().await;
    assert_eq!(active.load(Ordering::SeqCst), CONCURRENCY);
    cancel.cancel();
    worker.await.unwrap();
    assert_eq!(active.load(Ordering::SeqCst), 0);
    assert!(sender.0.state.lock().unwrap().pending.is_empty());
}

#[tokio::test(start_paused = true)]
async fn failure_retries_only_its_platform_and_deduplicates_retry_events() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = channel(2);
    let failing = Uuid::now_v7();
    let healthy = Uuid::now_v7();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let writes = Arc::new(Mutex::new(Vec::new()));
    let (reads, commits, token) = (calls.clone(), writes.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run(
            &token,
            receiver,
            Duration::from_secs(5),
            &|key| {
                let mut calls = reads.lock().unwrap();
                let fail = key.platform_id == failing && !calls.contains(&failing);
                calls.push(key.platform_id);
                async move {
                    if fail {
                        Err("unavailable")
                    } else {
                        Ok(Some(()))
                    }
                }
            },
            &|key, ()| {
                commits.lock().unwrap().push(key.platform_id);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    sender.mark(failing, &cancel).await.unwrap();
    sender.mark(healthy, &cancel).await.unwrap();
    debounce().await;
    assert_eq!(*writes.lock().unwrap(), vec![healthy]);
    for _ in 0..100 {
        sender.mark(failing, &cancel).await.unwrap();
    }
    tokio::time::advance(Duration::from_secs(4)).await;
    settle().await;
    assert_eq!(calls.lock().unwrap().len(), 2);
    tokio::time::advance(Duration::from_secs(1)).await;
    settle().await;
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|&&id| id == failing)
            .count(),
        2
    );
    assert_eq!(writes.lock().unwrap().len(), 2);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn safety_pass_is_delayed_and_uses_only_current_manager_targets() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = channel(2);
    let manager = Uuid::now_v7();
    let current = Arc::new(Mutex::new(vec![manager]));
    let (targets, token, signal) = (current.clone(), cancel.clone(), sender.clone());
    let safety = tokio::spawn(async move {
        safety_pass(&token, Duration::from_secs(60), signal, || {
            let ids = targets.lock().unwrap().clone();
            async move { ids }
        })
        .await;
    });
    settle().await;
    assert!(sender.0.state.lock().unwrap().pending.is_empty());
    tokio::time::advance(Duration::from_secs(60)).await;
    settle().await;
    assert_eq!(
        sender
            .0
            .state
            .lock()
            .unwrap()
            .pending
            .keys()
            .copied()
            .collect::<Vec<_>>(),
        vec![manager]
    );
    sender.0.state.lock().unwrap().pending.clear();
    current.lock().unwrap().clear();
    tokio::time::advance(Duration::from_secs(60)).await;
    settle().await;
    assert!(sender.0.state.lock().unwrap().pending.is_empty());
    cancel.cancel();
    safety.await.unwrap();
    drop(receiver);
}

#[tokio::test(start_paused = true)]
async fn dirty_during_commit_survives_completion() {
    let cancel = CancellationToken::new();
    let (sender, receiver) = channel(1);
    let writes = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(Notify::new());
    let (commits, wake, token) = (writes.clone(), release.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run(
            &token,
            receiver,
            Duration::from_secs(5),
            &|_| async { Ok::<_, &str>(Some(())) },
            &|_, ()| {
                let pass = commits.fetch_add(1, Ordering::SeqCst);
                let wake = wake.clone();
                async move {
                    if pass == 0 {
                        wake.notified().await;
                    }
                    Ok::<_, &str>(true)
                }
            },
        )
        .await;
    });
    let id = Uuid::now_v7();
    sender.mark(id, &cancel).await.unwrap();
    debounce().await;
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    sender.mark(id, &cancel).await.unwrap();
    release.notify_one();
    settle().await;
    assert_eq!(sender.0.state.lock().unwrap().pending.len(), 1);
    debounce().await;
    assert_eq!(writes.load(Ordering::SeqCst), 2);
    assert!(sender.0.state.lock().unwrap().pending.is_empty());
    cancel.cancel();
    worker.await.unwrap();
}
