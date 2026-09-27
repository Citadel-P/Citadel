use super::*;
use citadel_platforms::jobs::ReconciliationScope;
use std::sync::{
    Mutex,
    atomic::{AtomicUsize, Ordering},
};

fn request(platform_id: uuid::Uuid, scope: ReconciliationScope) -> ScopedEventRequest {
    ScopedEventRequest {
        platform_id,
        refresh: EventRefresh::Resource(scope),
    }
}

#[tokio::test(start_paused = true)]
async fn failing_containers_do_not_delay_images_or_other_platforms_and_shutdown_cancels_reads() {
    let cancel = CancellationToken::new();
    let (sender, receivers) = event_refresh_channels(4);
    let failing = request(uuid::Uuid::now_v7(), ReconciliationScope::Containers);
    let healthy = request(uuid::Uuid::now_v7(), ReconciliationScope::Containers);
    let image = request(failing.platform_id, ReconciliationScope::Images);
    let committed = Arc::new(Mutex::new(Vec::new()));
    let calls = Arc::new(Mutex::new(Vec::new()));
    let worker_cancel = cancel.clone();
    let writes = committed.clone();
    let reads = calls.clone();
    let worker = tokio::spawn(async move {
        run_refresh_workers(
            &worker_cancel,
            receivers,
            Duration::from_secs(10),
            |key| {
                reads.lock().unwrap().push(key);
                async move {
                    if key == failing {
                        Err("containers unavailable")
                    } else {
                        Ok(Some(()))
                    }
                }
            },
            |key, ()| {
                writes.lock().unwrap().push(key);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    sender.send(failing, &cancel).await.unwrap();
    sender.send(image, &cancel).await.unwrap();
    sender.send(healthy, &cancel).await.unwrap();
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert!(committed.lock().unwrap().contains(&image));
    assert!(committed.lock().unwrap().contains(&healthy));
    assert!(!committed.lock().unwrap().contains(&failing));
    assert_eq!(
        calls
            .lock()
            .unwrap()
            .iter()
            .filter(|&&key| key == failing)
            .count(),
        1
    );
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn burst_during_collection_discards_old_read_and_commits_one_follow_up() {
    let cancel = CancellationToken::new();
    let (sender, receivers) = event_refresh_channels(1);
    let key = request(uuid::Uuid::now_v7(), ReconciliationScope::Images);
    let calls = Arc::new(AtomicUsize::new(0));
    let commits = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(tokio::sync::Notify::new());
    let (reads, writes, wake, token) = (
        calls.clone(),
        commits.clone(),
        release.clone(),
        cancel.clone(),
    );
    let worker = tokio::spawn(async move {
        run_refresh_workers(
            &token,
            receivers,
            Duration::from_secs(2),
            |_| {
                let pass = reads.fetch_add(1, Ordering::SeqCst);
                let wake = wake.clone();
                async move {
                    if pass == 0 {
                        wake.notified().await;
                    }
                    Ok::<_, &str>(Some(pass))
                }
            },
            |_, pass| {
                assert_eq!(pass, 1);
                writes.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    sender.send(key, &cancel).await.unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    for _ in 0..20 {
        sender.send(key, &cancel).await.unwrap();
    }
    tokio::task::yield_now().await;
    release.notify_one();
    for _ in 0..10 {
        tokio::task::yield_now().await;
    }
    assert_eq!(commits.load(Ordering::SeqCst), 0);
    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(commits.load(Ordering::SeqCst), 1);
    cancel.cancel();
    worker.await.unwrap();
}

#[tokio::test(start_paused = true)]
async fn stalled_container_read_keeps_image_lane_live_and_cancellation_drops_the_read() {
    let cancel = CancellationToken::new();
    let (sender, receivers) = event_refresh_channels(2);
    let container = request(uuid::Uuid::now_v7(), ReconciliationScope::Containers);
    let image = request(container.platform_id, ReconciliationScope::Images);
    let dropped = Arc::new(AtomicUsize::new(0));
    let writes = Arc::new(AtomicUsize::new(0));
    struct ReadGuard(Arc<AtomicUsize>);
    impl Drop for ReadGuard {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    let (drop_count, commits, token) = (dropped.clone(), writes.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_refresh_workers(
            &token,
            receivers,
            Duration::from_secs(10),
            |key| {
                let drop_count = drop_count.clone();
                async move {
                    if key == container {
                        let _guard = ReadGuard(drop_count);
                        std::future::pending::<()>().await;
                    }
                    Ok::<_, &str>(Some(()))
                }
            },
            |key, ()| {
                assert_eq!(key, image);
                commits.fetch_add(1, Ordering::SeqCst);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    sender.send(container, &cancel).await.unwrap();
    sender.send(image, &cancel).await.unwrap();
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(1)).await;
    for _ in 0..20 {
        tokio::task::yield_now().await;
    }
    assert_eq!(writes.load(Ordering::SeqCst), 1);
    assert_eq!(dropped.load(Ordering::SeqCst), 0);
    cancel.cancel();
    worker.await.unwrap();
    assert_eq!(dropped.load(Ordering::SeqCst), 1);
}

#[tokio::test(start_paused = true)]
async fn failing_volume_does_not_repeat_or_delay_other_recovery_scopes() {
    let cancel = CancellationToken::new();
    let (sender, receivers) = event_refresh_channels(8);
    let platform = uuid::Uuid::now_v7();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let commits = Arc::new(Mutex::new(Vec::new()));
    let (reads, writes, token) = (calls.clone(), commits.clone(), cancel.clone());
    let worker = tokio::spawn(async move {
        run_refresh_workers(
            &token,
            receivers,
            Duration::from_secs(10),
            |key| {
                reads.lock().unwrap().push(key.refresh);
                async move {
                    if key.refresh == EventRefresh::Resource(ReconciliationScope::Volumes) {
                        Err("volume failure")
                    } else {
                        Ok(Some(()))
                    }
                }
            },
            |key, ()| {
                writes.lock().unwrap().push(key.refresh);
                async { Ok::<_, &str>(true) }
            },
        )
        .await;
    });
    for refresh in std::iter::once(EventRefresh::Platform).chain(
        citadel_platforms::jobs::RuntimeRecoveryCoordinator::platform_committed(
            citadel_platforms::PlatformKind::DockerSwarm,
        ),
    ) {
        sender
            .send(
                ScopedEventRequest {
                    platform_id: platform,
                    refresh,
                },
                &cancel,
            )
            .await
            .unwrap();
    }
    tokio::task::yield_now().await;
    tokio::time::advance(Duration::from_secs(2)).await;
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
    assert_eq!(commits.lock().unwrap().len(), 5);
    assert_eq!(calls.lock().unwrap().len(), 6);
    tokio::time::advance(Duration::from_secs(11)).await;
    for _ in 0..30 {
        tokio::task::yield_now().await;
    }
    assert_eq!(
        commits.lock().unwrap().len(),
        5,
        "healthy scopes must remain idle"
    );
    assert_eq!(
        calls.lock().unwrap().len(),
        7,
        "only the failed Volume scope retries"
    );
    cancel.cancel();
    worker.await.unwrap();
}
