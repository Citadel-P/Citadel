use super::*;
use citadel_adapters::persistence::postgres::connection_events::{CHANNEL, ConnectionEvent};
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
    let task = tokio::spawn(hub.run(Box::new(listener), token.clone()));
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
    let worker = tokio::spawn(hub.run(Box::new(listener), token.clone()));
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
    let worker = tokio::spawn(hub.run(Box::new(listener), token.clone()));
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
