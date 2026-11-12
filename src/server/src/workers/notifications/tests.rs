use super::*;
use citadel_runtime::RuntimeSignal;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
#[tokio::test(start_paused = true)]
async fn coalesces_busy_worker_signals_and_falls_back_after_loss() {
    let hub = DatabaseNotificationHub::new();
    let mut signals = hub.subscribe(RuntimeSignal::Builds);
    for _ in 0..1000 {
        hub.signals[RuntimeSignal::Builds as usize].send_modify(|_| {});
    }
    let token = CancellationToken::new();
    let start = tokio::time::Instant::now();
    wait(&mut signals, &token, Duration::from_secs(30)).await;
    assert_eq!(start, tokio::time::Instant::now());
    wait(&mut signals, &token, Duration::from_secs(30)).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(30));
    token.cancel();
    wait(&mut signals, &token, Duration::from_secs(30)).await;
}

#[tokio::test]
async fn scoped_subscriber_lag_requests_resync_and_retains_later_events() {
    use citadel_runtime::notifications::RuntimeNotification;
    let hub = DatabaseNotificationHub::new();
    let mut slow = hub.scoped(ScopedTopic::Copies);
    let mut fast = hub.scoped(ScopedTopic::Copies);
    let id = uuid::Uuid::now_v7();
    for _ in 0..300 {
        hub.changes[ScopedTopic::Copies as usize]
            .send(RuntimeNotification::ResourceCopy(id))
            .unwrap();
        assert!(
            matches!(fast.recv().await.unwrap(), RuntimeNotification::ResourceCopy(received) if received == id)
        );
    }
    assert!(matches!(
        slow.recv().await.unwrap(),
        RuntimeNotification::Resync
    ));
    assert!(
        matches!(slow.recv().await.unwrap(), RuntimeNotification::ResourceCopy(received) if received == id)
    );
    drop(hub);
    assert!(
        tokio::time::timeout(Duration::from_millis(20), fast.recv())
            .await
            .is_err(),
        "closed fanout must not spin"
    );
}
