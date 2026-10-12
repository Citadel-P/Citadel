use super::*;
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
