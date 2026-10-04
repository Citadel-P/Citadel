//! One LISTEN connection fans out worker wakeups and committed connection events.
use super::connection_events::{ConnectionEventHub, ConnectionNotification};
use citadel_adapters::persistence::postgres::connection_events::{CHANNEL, ConnectionEvent};
use citadel_runtime::RuntimeSignal;
use std::{sync::Arc, time::Duration};
use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub(super) struct DatabaseNotificationHub {
    signals: Arc<[watch::Sender<()>; RuntimeSignal::ALL.len()]>,
    pub connections: ConnectionEventHub,
}
impl DatabaseNotificationHub {
    pub fn new() -> Self {
        Self {
            signals: Arc::new(std::array::from_fn(|_| watch::channel(()).0)),
            connections: ConnectionEventHub::new(256),
        }
    }
    pub fn subscribe(&self, signal: RuntimeSignal) -> watch::Receiver<()> {
        self.signals[signal as usize].subscribe()
    }
    pub async fn run(
        self,
        mut listener: sqlx::postgres::PgListener,
        token: CancellationToken,
    ) -> Result<(), std::convert::Infallible> {
        loop {
            let result = tokio::select! { ()=token.cancelled()=>return Ok(()), result=listener.try_recv()=>result };
            match result {
                Ok(Some(notification)) => {
                    if notification.channel() == CHANNEL {
                        match serde_json::from_str::<ConnectionEvent>(notification.payload()) {
                            Ok(event) => self
                                .connections
                                .publish(ConnectionNotification::Changed(event)),
                            Err(error) => {
                                tracing::warn!(%error, "Invalid connection event; refreshing subscribers");
                                self.connections.publish(ConnectionNotification::Resync);
                            }
                        }
                    }
                    if let Some(signal) = RuntimeSignal::ALL
                        .into_iter()
                        .find(|signal| signal.channel() == notification.channel())
                    {
                        self.signals[signal as usize].send_modify(|_| {});
                    }
                }
                Ok(None) => {
                    self.connections.publish(ConnectionNotification::Resync);
                    citadel_runtime::runtime_metrics::RuntimeWork::NotificationReconnect.units(1);
                    // SQLx has re-established LISTEN before returning None. Recover lost
                    // signals immediately; timers are still the final safety net.
                    for signal in self.signals.iter() {
                        signal.send_modify(|_| {});
                    }
                }
                Err(error) => {
                    citadel_runtime::runtime_metrics::RuntimeWork::NotificationReconnect.units(1);
                    tracing::warn!(%error, "Database notification hub reconnecting; fallback timers remain active");
                    tokio::select! { ()=token.cancelled()=>return Ok(()), _=tokio::time::sleep(Duration::from_secs(2))=>{} }
                }
            }
        }
    }
}

/// A signal arriving during execution stays pending until this wait. A missed
/// notification can delay persisted work by at most the fallback duration.
pub(super) async fn wait(
    receiver: &mut watch::Receiver<()>,
    token: &CancellationToken,
    fallback: Duration,
) {
    tokio::select! {
        () = token.cancelled() => {},
        _ = tokio::time::sleep(fallback) => {},
        changed = receiver.changed() => if changed.is_err() {
            tokio::select! { ()=token.cancelled()=>{}, _=tokio::time::sleep(fallback)=>{} }
        },
    }
}

#[cfg(test)]
mod tests {
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
}

#[cfg(test)]
mod postgres_tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires CITADEL_TEST_DATABASE_URL"]
    async fn connection_events_reach_independent_subscribers_after_commit() {
        let pool = sqlx::PgPool::connect(&std::env::var("CITADEL_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let listener = super::super::listener(&pool, CHANNEL).await.unwrap();
        let hub = DatabaseNotificationHub::new();
        let mut first = hub.connections.subscribe();
        let mut second = hub.connections.subscribe();
        let token = CancellationToken::new();
        let task = tokio::spawn(hub.run(listener, token.clone()));
        let event = ConnectionEvent {
            resource: citadel_adapters::persistence::postgres::connection_events::ConnectionResource::BuildAgentPool,
            resource_id: uuid::Uuid::now_v7(),
            state: citadel_adapters::persistence::postgres::connection_events::ConnectionState::Offline,
        };
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(CHANNEL)
            .bind(serde_json::to_string(&event).unwrap())
            .execute(&mut *tx)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(100), first.recv())
                .await
                .is_err()
        );
        tx.commit().await.unwrap();
        for receiver in [&mut first, &mut second] {
            let notification = tokio::time::timeout(Duration::from_secs(2), receiver.recv())
                .await
                .unwrap()
                .unwrap();
            assert!(
                matches!(notification, ConnectionNotification::Changed(received) if received == event)
            );
        }
        token.cancel();
        task.await.unwrap().unwrap();
    }

    #[tokio::test]
    #[ignore = "requires CITADEL_TEST_DATABASE_URL"]
    async fn every_fixed_topic_obeys_commit_and_rollback() {
        let pool = sqlx::PgPool::connect(&std::env::var("CITADEL_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let mut listener = super::super::listener(&pool, "citadel_notify_transaction_test")
            .await
            .unwrap();
        let hub = DatabaseNotificationHub::new();
        let mut receivers = Vec::new();
        for signal in RuntimeSignal::ALL {
            listener.listen(signal.channel()).await.unwrap();
            receivers.push(hub.subscribe(signal));
        }
        let token = CancellationToken::new();
        let worker = tokio::spawn(hub.run(listener, token.clone()));
        for (signal, receiver) in RuntimeSignal::ALL.into_iter().zip(&mut receivers) {
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SELECT pg_notify($1, '')")
                .bind(signal.channel())
                .execute(&mut *tx)
                .await
                .unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(20), receiver.changed())
                    .await
                    .is_err()
            );
            tx.rollback().await.unwrap();
            assert!(
                tokio::time::timeout(Duration::from_millis(20), receiver.changed())
                    .await
                    .is_err()
            );
            let mut tx = pool.begin().await.unwrap();
            sqlx::query("SELECT pg_notify($1, '')")
                .bind(signal.channel())
                .execute(&mut *tx)
                .await
                .unwrap();
            tx.commit().await.unwrap();
            tokio::time::timeout(Duration::from_secs(2), receiver.changed())
                .await
                .unwrap()
                .unwrap();
        }
        token.cancel();
        worker.await.unwrap().unwrap();
        pool.close().await;
    }

    #[tokio::test]
    #[ignore = "requires CITADEL_TEST_DATABASE_URL"]
    async fn notifications_follow_commit_and_recover_after_listener_disconnect() {
        let pool = sqlx::PgPool::connect(&std::env::var("CITADEL_TEST_DATABASE_URL").unwrap())
            .await
            .unwrap();
        let mut listener = super::super::listener(&pool, "citadel_notify_reconnect_test")
            .await
            .unwrap();
        listener
            .listen(RuntimeSignal::Builds.channel())
            .await
            .unwrap();
        let hub = DatabaseNotificationHub::new();
        let mut wake = hub.subscribe(RuntimeSignal::Builds);
        let token = CancellationToken::new();
        let worker = tokio::spawn(hub.run(listener, token.clone()));
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_notify('citadel_build_work','')")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(50), wake.changed())
                .await
                .is_err()
        );
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_notify('citadel_build_work','')")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.commit().await.unwrap();
        tokio::time::timeout(Duration::from_secs(3), wake.changed())
            .await
            .unwrap()
            .unwrap();
        let disconnected: bool = sqlx::query_scalar("SELECT pg_terminate_backend(pid) FROM pg_stat_activity WHERE datname=current_database() AND application_name='citadel_notify_reconnect_test'").fetch_one(&pool).await.unwrap();
        assert!(disconnected);
        tokio::time::timeout(Duration::from_secs(5), wake.changed())
            .await
            .unwrap()
            .unwrap();
        sqlx::query("SELECT pg_notify('citadel_build_work','')")
            .execute(&pool)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), wake.changed())
            .await
            .unwrap()
            .unwrap();
        token.cancel();
        worker.await.unwrap().unwrap();
        pool.close().await;
    }
}
