use super::*;
use citadel_runtime::maintenance::MaintenanceStore;

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn oversized_minute_stays_unsealed_and_does_not_starve_next_source() {
    let (pool, database) = storage_fixture().await;
    let dense = platform(&pool).await;
    let small = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86400) / 60 * 60;
    for scope in [&dense, &small] {
        sqlx::query("INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'')")
            .bind(scope.platform_id)
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
        SELECT gen_random_uuid(),$1,$2+n*10,1,1,0,0,
        CASE WHEN n<2 THEN (SELECT jsonb_agg(jsonb_build_object('containerId',c::text,'rxBytes',0,'txBytes',0)) FROM generate_series(1,50001)c) ELSE '[]'::jsonb END
        FROM unnest(ARRAY[0,1,6])n")
        .bind(dense.platform_id).bind(start).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes) SELECT gen_random_uuid(),$1,$2+n,1,1,0,0 FROM unnest(ARRAY[0,10,60])n")
        .bind(small.platform_id).bind(start).execute(&pool).await.unwrap();
    let store = citadel_adapters::persistence::postgres::maintenance::PostgresMaintenanceStore::new(
        pool.clone(),
    );
    assert!(
        store.compact().await.unwrap().stalled_source.is_some(),
        "oversized input is a source-local stall, not a global failure"
    );
    let sealed: i64 =
        sqlx::query_scalar("SELECT compacteduntil FROM platformhistorysources WHERE platformid=$1")
            .bind(dense.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sealed, 0, "never seal part of an oversized minute");
    assert!(
        store.compact().await.unwrap().processed > 0,
        "cursor must admit another source"
    );
    let sealed: i64 =
        sqlx::query_scalar("SELECT compacteduntil FROM platformhistorysources WHERE platformid=$1")
            .bind(small.platform_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(sealed, start + 60);
    sqlx::query("UPDATE platformstats SET containerio='[]' WHERE platformid=$1 AND created=$2")
        .bind(dense.platform_id)
        .bind(start)
        .execute(&pool)
        .await
        .unwrap();
    assert!(
        store.compact().await.unwrap().processed > 0,
        "a stalled source is retried after correction"
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM platformstats WHERE platformid=$1")
        .bind(dense.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 3,
        "sparse/cost gate must still preserve the raw minute"
    );
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn compaction_seals_only_a_prefix_of_complete_minutes_within_input_budget() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86400) / 60 * 60;
    sqlx::query("INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'')")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,containerio)
        SELECT gen_random_uuid(),$1,$2+n,1,1,0,0,
        CASE WHEN n<120 THEN (SELECT jsonb_agg(jsonb_build_object('containerId',c::text,'rxBytes',0,'txBytes',0)) FROM generate_series(1,30000)c) ELSE '[]'::jsonb END
        FROM unnest(ARRAY[0,10,60,70,120])n")
        .bind(scope.platform_id).bind(start).execute(&pool).await.unwrap();
    let store = citadel_adapters::persistence::postgres::maintenance::PostgresMaintenanceStore::new(
        pool.clone(),
    );
    for expected in [start + 60, start + 120] {
        assert!(store.compact().await.unwrap().processed > 0);
        let sealed: i64 = sqlx::query_scalar(
            "SELECT compacteduntil FROM platformhistorysources WHERE platformid=$1",
        )
        .bind(scope.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(sealed, expected);
    }
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn session_closed_while_waiting_for_platform_lock_cannot_persist() {
    let (pool, database) = storage_fixture().await;
    let mut scope = platform(&pool).await;
    let closed = tokio_util::sync::CancellationToken::new();
    scope.closed = Some(closed.clone());
    let mut blocker = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(scope.platform_id)
        .execute(&mut *blocker)
        .await
        .unwrap();
    let worker_pool = pool.clone();
    let sample = PlatformStatsSample {
        scope: scope.clone(),
        created: chrono::Utc::now().timestamp(),
        memory_active: 10.,
        cpu_usage: 1.,
        rx_bytes: 0.,
        tx_bytes: 0.,
        container_io: vec![],
        metadata: None,
    };
    let worker = tokio::spawn(async move {
        PostgresStatsBatchStore::new(worker_pool)
            .persist_batch(&[sample])
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid)))",
            )
            .bind(pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    closed.cancel();
    blocker.commit().await.unwrap();
    let result = worker.await.unwrap().unwrap();
    assert_eq!((result.persisted, result.stale), (0, 1));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM platformstats WHERE platformid=$1")
        .bind(scope.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn shared_history_admission_leaves_default_pool_capacity_for_ingestion() {
    use citadel_platforms::{ServiceStatIdentity, StatisticsReader, StatsWindow};
    use std::sync::Arc;
    let (pool, database) = storage_fixture().await;
    pool.close().await;
    let mut url =
        url::Url::parse(&std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap()).unwrap();
    url.set_path(&database);
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(url.as_str())
        .await
        .unwrap();

    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "history-admission").await;
    let reader = Arc::new(citadel_adapters::persistence::postgres::platforms::statistics::reader::PostgresStatisticsReader::new(pool.clone()));
    let now = chrono::Utc::now().timestamp();
    let mut blocker = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE statisticsrollups IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let first_reader = reader.clone();
    let first = tokio::spawn(async move {
        first_reader
            .containers(&[id], StatsWindow::new(1).unwrap(), now)
            .await
    });
    let second_reader = reader.clone();
    let platform_id = scope.platform_id;
    let second = tokio::spawn(async move {
        second_reader
            .service(
                ServiceStatIdentity {
                    platform_id,
                    docker_service_id: "fixture",
                    managed_service_id: None,
                    stack_id: None,
                    service_name: "fixture",
                },
                StatsWindow::new(1).unwrap(),
                now,
            )
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let waiting: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))",
            )
            .bind(pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if waiting >= 2 {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let error = reader
        .containers(&[id], StatsWindow::new(1).unwrap(), now)
        .await
        .unwrap_err();
    assert_eq!(
        error.kind,
        citadel_platforms::RuntimeErrorKind::ResourceExhausted
    );
    let sample = PlatformStatsSample {
        scope,
        created: now,
        memory_active: 10.,
        cpu_usage: 1.,
        rx_bytes: 0.,
        tx_bytes: 0.,
        container_io: vec![],
        metadata: None,
    };
    let written = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        let ordinary: i32 = sqlx::query_scalar("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(ordinary, 1);
        PostgresStatsBatchStore::new(pool.clone())
            .persist_batch(&[sample])
            .await
    })
    .await
    .unwrap()
    .unwrap();
    assert_eq!(written.persisted, 1);
    blocker.commit().await.unwrap();
    first.await.unwrap().unwrap();
    second.await.unwrap().unwrap();
    close_storage_fixture(pool, database).await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn slow_round_trips_commit_a_complete_minute_prefix_within_worker_budget() {
    use std::time::{Duration, Instant};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let start = (chrono::Utc::now().timestamp() - 2 * 86400) / 60 * 60;
    sqlx::query("INSERT INTO platformhistorysources(platformid,nodeid) VALUES($1,'')")
        .bind(scope.platform_id)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes) SELECT gen_random_uuid(),$1,$2+n*60,1,1,0,0 FROM generate_series(0,60)n")
        .bind(scope.platform_id).bind(start).execute(&pool).await.unwrap();
    let mut url =
        url::Url::parse(&std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap()).unwrap();
    url.set_path(&database);
    let upstream = format!("{}:{}", url.host_str().unwrap(), url.port().unwrap_or(5432));
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    url.set_host(Some("127.0.0.1")).unwrap();
    url.set_port(Some(listener.local_addr().unwrap().port()))
        .unwrap();
    url.query_pairs_mut().append_pair("sslmode", "disable");
    let proxy = tokio::spawn(async move {
        let mut connections = tokio::task::JoinSet::new();
        loop {
            let (client, _) = listener.accept().await.unwrap();
            let upstream = upstream.clone();
            connections.spawn(async move {
                let server = TcpStream::connect(upstream).await.unwrap();
                let (mut cr, mut cw) = client.into_split();
                let (mut sr, mut sw) = server.into_split();
                let responses = async {
                    let mut buffer = [0u8; 8192];
                    loop {
                        let n = sr.read(&mut buffer).await?;
                        if n == 0 {
                            return Ok::<(), std::io::Error>(());
                        }
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        cw.write_all(&buffer[..n]).await?;
                    }
                };
                tokio::select! {
                    _ = tokio::io::copy(&mut cr, &mut sw) => {},
                    _ = responses => {},
                }
            });
        }
    });
    let slow_pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(1)
        .connect(url.as_str())
        .await
        .unwrap();
    let store = citadel_adapters::persistence::postgres::maintenance::PostgresMaintenanceStore::new(
        slow_pool.clone(),
    );
    let mut previous = start;
    for _ in 0..2 {
        let elapsed = Instant::now();
        let progress = tokio::time::timeout(Duration::from_secs(5), store.compact())
            .await
            .expect("sizing must leave time for a committed prefix")
            .unwrap();
        assert!(progress.processed > 0);
        let sealed: i64 = sqlx::query_scalar(
            "SELECT compacteduntil FROM platformhistorysources WHERE platformid=$1",
        )
        .bind(scope.platform_id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert!(sealed > previous && sealed < start + 3600);
        assert_eq!(sealed % 60, 0);
        eprintln!(
            "100ms response delay: committed {} minutes in {:?}",
            (sealed - previous) / 60,
            elapsed.elapsed()
        );
        previous = sealed;
    }
    slow_pool.close().await;
    proxy.abort();
    let _ = proxy.await;
    close_storage_fixture(pool, database).await;
}
