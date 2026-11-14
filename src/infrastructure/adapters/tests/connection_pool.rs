use std::time::Duration;

use citadel_adapters::persistence::postgres::connection::connect_pool;

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL; waits for real pool idle retirement"]
async fn idle_sessions_retire_without_interrupting_transactions_or_preventing_bursts() {
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    let pool = connect_pool(&url, 3, Duration::from_secs(5)).await.unwrap();
    let mut transaction = pool.begin().await.unwrap();
    let held_pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    let mut idle_pids = Vec::new();
    let mut burst = Vec::new();
    for _ in 0..2 {
        let mut connection = pool.acquire().await.unwrap();
        idle_pids.push(
            sqlx::query_scalar::<_, i32>("SELECT pg_backend_pid()")
                .fetch_one(&mut *connection)
                .await
                .unwrap(),
        );
        burst.push(connection);
    }
    assert_eq!(pool.size(), 3);
    drop(burst);

    // Observe pool accounting without acquiring connections and resetting idleness.
    // Allow two maintenance sweeps plus scheduling headroom.
    tokio::time::timeout(Duration::from_secs(150), async {
        while pool.size() != 1 {
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    })
    .await
    .expect("surplus idle sessions should retire");

    let still_held: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *transaction)
        .await
        .unwrap();
    assert_eq!(still_held, held_pid);
    let mut next_burst = Vec::new();
    for _ in 0..2 {
        let mut connection = pool.acquire().await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(&mut *connection)
            .await
            .unwrap();
        assert!(!idle_pids.contains(&pid), "idle backend must be replaced");
        next_burst.push(connection);
    }
    assert_eq!(pool.size(), 3);
    transaction.commit().await.unwrap();
    drop(next_burst);
    pool.close().await;
}

#[tokio::test]
async fn database_snapshot_preserves_pool_information_when_server_is_unavailable() {
    use citadel_instance::InstanceRepository;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(7)
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    pool.close().await;
    let store =
        citadel_adapters::persistence::postgres::instance::PostgresInstanceRepository::new(pool);
    let snapshot = store.database_connections().await.unwrap();
    assert_eq!(snapshot.maximum, 7);
    assert_eq!(snapshot.open, 0);
    assert_eq!(snapshot.idle, 0);
    assert_eq!(snapshot.borrowed, 0);
    assert_eq!(snapshot.shared_buffers_bytes, None);
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_EXECUTION_DATABASE_URL"]
async fn database_snapshot_is_bounded_when_the_only_connection_is_borrowed() {
    use citadel_instance::InstanceRepository;
    let url = std::env::var("CITADEL_EXECUTION_DATABASE_URL").unwrap();
    let pool = connect_pool(&url, 1, Duration::from_secs(5)).await.unwrap();
    let mut transaction = pool.begin().await.unwrap();
    let store = citadel_adapters::persistence::postgres::instance::PostgresInstanceRepository::new(
        pool.clone(),
    );
    let snapshot = tokio::time::timeout(Duration::from_secs(3), store.database_connections())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(snapshot.maximum, 1);
    assert_eq!(snapshot.open, 1);
    assert_eq!(snapshot.borrowed, 1);
    assert_eq!(snapshot.idle, 0);
    assert_eq!(snapshot.shared_buffers_bytes, None);
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&mut *transaction)
            .await
            .unwrap(),
        1
    );
    transaction.commit().await.unwrap();
    let snapshot = store.database_connections().await.unwrap();
    assert!(snapshot.shared_buffers_bytes.is_some_and(|value| value > 0));
    pool.close().await;
}
