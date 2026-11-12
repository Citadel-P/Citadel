use super::*;
use crate::workers::connection_events::ConnectionNotification;
use citadel_adapters::persistence::postgres::connection_events::{CHANNEL, ConnectionEvent};
use citadel_runtime::RuntimeSignal;
use citadel_runtime::notifications::RuntimeNotification;
use std::time::Duration;
use tokio_util::sync::CancellationToken;
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
    for channel in [
        "citadel_resource_copies",
        "citadel_backup_coverage",
        "citadel_stack_drift",
    ] {
        listener.listen(channel).await.unwrap();
    }
    let mut scoped = [
        hub.scoped(ScopedTopic::Copies),
        hub.scoped(ScopedTopic::Coverage),
        hub.scoped(ScopedTopic::StackDrift),
    ];
    let mut copy_work = hub.copy_work();
    let mut wake = hub.subscribe(RuntimeSignal::Builds);
    let mut job_alerts = hub.subscribe(RuntimeSignal::JobAlerts);
    let mut pruning = hub.subscribe(RuntimeSignal::SwarmPrune);
    for signal in [RuntimeSignal::JobAlerts, RuntimeSignal::SwarmPrune] {
        listener.listen(signal.channel()).await.unwrap();
    }
    let connections: i64 = sqlx::query_scalar("SELECT count(*) FROM pg_stat_activity WHERE datname=current_database() AND application_name='citadel_notify_reconnect_test'")
        .fetch_one(&pool).await.unwrap();
    assert_eq!(connections, 1, "worker topics share one LISTEN backend");
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
    for receiver in [&mut wake, &mut job_alerts, &mut pruning, &mut copy_work] {
        tokio::time::timeout(Duration::from_secs(5), receiver.changed())
            .await
            .unwrap()
            .unwrap();
    }
    for receiver in &mut scoped {
        assert!(matches!(
            tokio::time::timeout(Duration::from_secs(3), receiver.recv())
                .await
                .unwrap()
                .unwrap(),
            RuntimeNotification::Resync
        ));
    }
    let id = uuid::Uuid::now_v7();
    for (channel, scoped) in [
        "citadel_resource_copies",
        "citadel_backup_coverage",
        "citadel_stack_drift",
    ]
    .into_iter()
    .zip(&mut scoped)
    {
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(channel)
            .bind(id.to_string())
            .execute(&pool)
            .await
            .unwrap();
        let event = tokio::time::timeout(Duration::from_secs(3), scoped.recv())
            .await
            .unwrap()
            .unwrap();
        assert!(
            matches!(event, RuntimeNotification::ResourceCopy(received) | RuntimeNotification::BackupCoverage(received) | RuntimeNotification::StackDrift(received) if received == id)
        );
    }
    tokio::time::timeout(Duration::from_secs(3), copy_work.changed())
        .await
        .unwrap()
        .unwrap();
    for (signal, receiver) in [
        (RuntimeSignal::JobAlerts, &mut job_alerts),
        (RuntimeSignal::SwarmPrune, &mut pruning),
    ] {
        sqlx::query("SELECT pg_notify($1, '')")
            .bind(signal.channel())
            .execute(&pool)
            .await
            .unwrap();
        tokio::time::timeout(Duration::from_secs(3), receiver.changed())
            .await
            .unwrap()
            .unwrap();
    }
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

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL"]
async fn scoped_events_are_commit_fenced_and_wake_both_copy_lanes() {
    let pool = sqlx::PgPool::connect(&std::env::var("CITADEL_TEST_DATABASE_URL").unwrap())
        .await
        .unwrap();
    let mut listener = super::super::listener(&pool, "citadel_scoped_transaction_test")
        .await
        .unwrap();
    let channels = [
        "citadel_resource_copies",
        "citadel_backup_coverage",
        "citadel_stack_drift",
    ];
    for channel in channels {
        listener.listen(channel).await.unwrap();
    }
    let hub = DatabaseNotificationHub::new();
    let mut scoped = [
        hub.scoped(ScopedTopic::Copies),
        hub.scoped(ScopedTopic::Coverage),
        hub.scoped(ScopedTopic::StackDrift),
    ];
    let mut execution = hub.copy_work();
    let mut reconciliation = hub.copy_work();
    let cancel = CancellationToken::new();
    let worker = tokio::spawn(hub.run(Box::new(listener), cancel.clone()));
    for (channel, scoped) in channels.into_iter().zip(&mut scoped) {
        let id = uuid::Uuid::now_v7();
        let mut tx = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(channel)
            .bind(id.to_string())
            .execute(&mut *tx)
            .await
            .unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), scoped.recv())
                .await
                .is_err()
        );
        tx.rollback().await.unwrap();
        assert!(
            tokio::time::timeout(Duration::from_millis(20), scoped.recv())
                .await
                .is_err()
        );
        assert!(!execution.has_changed().unwrap());
        sqlx::query("SELECT pg_notify($1, $2)")
            .bind(channel)
            .bind(id.to_string())
            .execute(&pool)
            .await
            .unwrap();
        let event = tokio::time::timeout(Duration::from_secs(3), scoped.recv())
            .await
            .unwrap()
            .unwrap();
        match channel {
            "citadel_resource_copies" => {
                assert!(
                    matches!(event, RuntimeNotification::ResourceCopy(received) if received == id)
                );
                for lane in [&mut execution, &mut reconciliation] {
                    tokio::time::timeout(Duration::from_secs(3), lane.changed())
                        .await
                        .unwrap()
                        .unwrap();
                }
            }
            "citadel_backup_coverage" => assert!(
                matches!(event, RuntimeNotification::BackupCoverage(received) if received == id)
            ),
            _ => assert!(
                matches!(event, RuntimeNotification::StackDrift(received) if received == id)
            ),
        }
    }
    cancel.cancel();
    worker.await.unwrap().unwrap();
    pool.close().await;
}
