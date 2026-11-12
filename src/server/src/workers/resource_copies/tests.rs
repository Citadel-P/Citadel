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
async fn active_copy_notifications_do_not_bypass_backoff_or_cancellation() {
    let (sender, mut wake) = watch::channel(());
    let cancel = CancellationToken::new();
    let start = tokio::time::Instant::now();
    sender.send_modify(|_| {});
    wait_for_work(&mut wake, &cancel, true).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(2));
    cancel.cancel();
    wait_for_work(&mut wake, &cancel, true).await;
    assert_eq!(tokio::time::Instant::now() - start, Duration::from_secs(2));
}
