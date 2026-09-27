use std::sync::Arc;
use std::time::Duration;

use citadel_alerts::{AlertDeliveryService, AlertError};
use tokio_util::sync::CancellationToken;

pub async fn deliveries(
    cancellation: CancellationToken,
    service: Arc<AlertDeliveryService>,
    mut wake: tokio::sync::watch::Receiver<()>,
) -> Result<(), AlertError> {
    loop {
        if cancellation.is_cancelled() {
            return Ok(());
        }
        match service.process_one(&cancellation).await {
            Ok(true) => continue,
            Ok(false) => {}
            Err(error) => tracing::error!(%error, "Alert delivery worker iteration failed"),
        }
        let fallback = if cancellation.is_cancelled() {
            return Ok(());
        } else {
            Duration::from_secs(60)
        };
        let sleep = match service.next_delivery_deadline().await {
            Ok(Some(deadline)) => deadline
                .signed_duration_since(chrono::Utc::now())
                .to_std()
                .unwrap_or(Duration::ZERO)
                .min(fallback),
            Ok(None) => fallback,
            Err(error) => {
                tracing::warn!(%error, "Could not read alert delivery deadline; using recovery fallback");
                Duration::from_secs(5)
            }
        };
        wait_for_work(&mut wake, &cancellation, sleep).await;
    }
}

async fn wait_for_work(
    wake: &mut tokio::sync::watch::Receiver<()>,
    cancellation: &CancellationToken,
    timeout: Duration,
) {
    tokio::select! {
        () = cancellation.cancelled() => {},
        _ = tokio::time::sleep(timeout) => {},
        changed = wake.changed() => if changed.is_err() {
            tokio::select! { ()=cancellation.cancelled()=>{}, _=tokio::time::sleep(timeout)=>{} }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn idle_delivery_waits_for_signal_or_deadline() {
        let (_sender, mut wake) = tokio::sync::watch::channel(());
        let cancellation = CancellationToken::new();
        let started = tokio::time::Instant::now();
        let waiter = tokio::spawn({
            let cancellation = cancellation.clone();
            async move { wait_for_work(&mut wake, &cancellation, Duration::from_secs(30)).await }
        });
        tokio::task::yield_now().await;
        assert!(
            !waiter.is_finished(),
            "empty queue must wait instead of polling"
        );
        tokio::time::advance(Duration::from_secs(29)).await;
        tokio::task::yield_now().await;
        assert!(!waiter.is_finished());
        tokio::time::advance(Duration::from_secs(1)).await;
        waiter.await.unwrap();
        assert_eq!(
            tokio::time::Instant::now() - started,
            Duration::from_secs(30)
        );

        let (sender, mut wake) = tokio::sync::watch::channel(());
        let waiter = tokio::spawn({
            let cancellation = cancellation.clone();
            async move { wait_for_work(&mut wake, &cancellation, Duration::from_secs(60)).await }
        });
        tokio::task::yield_now().await;
        sender.send_modify(|_| {});
        waiter.await.unwrap();
        assert_eq!(
            tokio::time::Instant::now() - started,
            Duration::from_secs(30)
        );
    }
}

/// Producers notify inside their state transaction; PostgreSQL delivers only after commit.
/// Carry the failure snapshot, since another operation may already have replaced the row.
pub(super) async fn observations(
    cancel: CancellationToken,
    alerts: Arc<dyn citadel_alerts::AlertEventSink>,
    mut listener: sqlx::postgres::PgListener,
) -> Result<(), std::convert::Infallible> {
    loop {
        let notification = tokio::select! { biased; ()=cancel.cancelled()=>return Ok(()), result=listener.recv()=>result };
        match notification {
            Ok(notification) => match serde_json::from_str::<citadel_alerts::AlertObservation>(
                notification.payload(),
            ) {
                Ok(observation) => {
                    if let Err(error) = alerts.observe(&observation).await {
                        tracing::warn!(%error,"Committed job alert evaluation failed");
                    }
                }
                Err(error) => tracing::warn!(%error,"Invalid job alert observation"),
            },
            Err(error) => {
                tracing::warn!(%error,"Job alert listener failed");
                tokio::select! { ()=cancel.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
            }
        }
    }
}
