use super::platforms::observe_platform_metrics;
use citadel_adapters::{
    container_stats_store::PostgresContainerStatsStore, postgres::alerts::PostgresAlertRepository,
};
use citadel_platforms::{HostDiskUsage, StatisticsReader};
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL on disposable PostgreSQL with CREATE DATABASE permission"]
async fn persisted_disk_samples_drive_builtin_alert_and_history_without_containers() {
    let admin_url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    let database = format!("citadel_disk_{}", Uuid::now_v7().simple());
    let mut admin = PgConnection::connect(&admin_url).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&mut admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&admin_url).unwrap();
    url.set_path(&database);
    citadel_database::MigrationRunner::migrate(url.as_str())
        .await
        .unwrap();
    let pool = PgPool::connect(url.as_str()).await.unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture','Local',2,0,1024,'Disk fixture',0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).execute(&pool).await.unwrap();
    let store = PostgresContainerStatsStore::new(pool.clone());
    let alerts = PostgresAlertRepository::new(pool.clone()); // Built-in rules work without paid entitlements.
    let rule = Uuid::parse_str("019d0000-0001-7000-8001-000000000024").unwrap();
    let configuration: (String, f64, i32, i32) = sqlx::query_as(
        "SELECT status,threshold,requiredmatches,cooldownseconds FROM alertrules WHERE id=$1",
    )
    .bind(rule)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(configuration, ("Enabled".into(), 90.0, 3, 300));

    for sample in 1..=4 {
        store
            .persist_with_disk(platform, &[], HostDiskUsage::new(95, 100, 95.0))
            .await
            .unwrap();
        observe_platform_metrics(&pool, &alerts, platform).await;
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM alertevents WHERE resourceid=$1 AND alertruleid=$2",
        )
        .bind(platform)
        .bind(rule)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(
            count,
            if sample < 3 { 0 } else { 1 },
            "three matches trigger one alert; cooldown prevents duplicates"
        );
    }
    let info: serde_json::Value = sqlx::query_scalar(
        "SELECT info::jsonb FROM alertevents WHERE resourceid=$1 AND alertruleid=$2",
    )
    .bind(platform)
    .bind(rule)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(info["DiskUsagePercent"], 95.0);
    assert_eq!(info["DiskUsedBytes"], 95);
    assert_eq!(info["DiskTotalBytes"], 100);

    // An unavailable sample is NULL, not a false recovery to zero.
    store.persist_with_disk(platform, &[], None).await.unwrap();
    observe_platform_metrics(&pool, &alerts, platform).await;
    let unavailable: Option<f64> = sqlx::query_scalar(
        "SELECT diskusage FROM platformstats WHERE platformid=$1 ORDER BY created DESC LIMIT 1",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(unavailable, None);
    let still_open: bool = sqlx::query_scalar(
        "SELECT resolvedat IS NULL FROM alertevents WHERE resourceid=$1 AND alertruleid=$2",
    )
    .bind(platform)
    .bind(rule)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(still_open);

    store
        .persist_with_disk(platform, &[], HostDiskUsage::new(50, 100, 50.0))
        .await
        .unwrap();
    observe_platform_metrics(&pool, &alerts, platform).await;
    let resolved: bool = sqlx::query_scalar(
        "SELECT resolvedat IS NOT NULL FROM alertevents WHERE resourceid=$1 AND alertruleid=$2",
    )
    .bind(platform)
    .bind(rule)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(resolved);
    // The same persisted fields feed history and dashboard projections.
    let history =
        citadel_adapters::statistics_read_store::PostgresStatisticsReader::new(pool.clone())
            .platform(
                platform,
                citadel_platforms::StatsWindow::new(24).unwrap(),
                chrono::Utc::now().timestamp(),
            )
            .await
            .unwrap();
    assert!(!history.is_empty());
    assert!(
        history
            .last()
            .unwrap()
            .disk_usage
            .is_some_and(|usage| (50.0..=95.0).contains(&usage))
    );
    assert_eq!(history.last().unwrap().disk_total_bytes, Some(100));

    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!(
        "DROP DATABASE {database} WITH (FORCE)"
    )))
    .execute(&mut admin)
    .await
    .unwrap();
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL"]
async fn threshold_flush_smooths_spikes_uses_latest_disk_and_flushes_idle_input() {
    use citadel_alerts::{AlertEventSink, AlertObservation};
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;
    #[derive(Default)]
    struct Capture(Mutex<Vec<AlertObservation>>);
    impl AlertEventSink for Capture {
        fn observe<'a>(
            &'a self,
            observation: &'a AlertObservation,
        ) -> futures_util::future::BoxFuture<
            'a,
            Result<Option<citadel_alerts::AlertEvent>, citadel_alerts::AlertError>,
        > {
            self.0.lock().unwrap().push(observation.clone());
            Box::pin(async { Ok(None) })
        }
    }
    let url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture','Local',2,0,1024,$2,0,'{\"$type\":\"Docker\"}','Offline',0)").bind(platform).bind(format!("median-{platform}")).execute(&pool).await.unwrap();
    let since = chrono::Utc::now().timestamp() - 10;
    for (offset, cpu, disk) in [
        (0, 10.0, Some(95.0)),
        (1, 99.0, Some(99.0)),
        (2, 20.0, None),
    ] {
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,diskusage,diskusedbytes,disktotalbytes,alertpending) VALUES($1,$2,$3,$4,30,0,0,$5,95,100,true)")
            .bind(Uuid::now_v7()).bind(platform).bind(since+offset).bind(cpu).bind(disk).execute(&pool).await.unwrap();
    }
    struct Unavailable;
    impl AlertEventSink for Unavailable {
        fn observe<'a>(
            &'a self,
            _: &'a AlertObservation,
        ) -> futures_util::future::BoxFuture<
            'a,
            Result<Option<citadel_alerts::AlertEvent>, citadel_alerts::AlertError>,
        > {
            Box::pin(async {
                Err(citadel_alerts::AlertError::Storage(
                    "injected failure".into(),
                ))
            })
        }
    }
    assert!(
        super::stats_alerts::flush_pending(&pool, &Unavailable, 500)
            .await
            .is_err()
    );
    let pending: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM platformstats WHERE platformid=$1 AND alertpending",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        pending, 3,
        "failed alert evaluation must not acknowledge samples"
    );
    let capture = Arc::new(Capture::default());
    super::stats_alerts::flush_pending(&pool, capture.as_ref(), 500)
        .await
        .unwrap();
    {
        let observations = capture.0.lock().unwrap();
        let cpu = observations
            .iter()
            .find(|o| o.resource_id == platform && o.alert_type == "PlatformCpuHigh")
            .unwrap();
        assert_eq!(
            cpu.value,
            Some(20.0),
            "one spike is smoothed using .NET median semantics"
        );
        assert!(
            !observations
                .iter()
                .any(|o| o.resource_id == platform && o.alert_type == "PlatformDiskHigh"),
            "newest unavailable disk does not reuse earlier high value"
        );
    }
    capture.0.lock().unwrap().clear();
    let cancel = CancellationToken::new();
    let worker = tokio::spawn(super::stats_alerts::run(
        cancel.clone(),
        pool.clone(),
        capture.clone(),
        Duration::from_millis(100),
        200,
    ));
    tokio::task::yield_now().await;
    // Delayed delivery must still be evaluated after its timestamp window.
    let now = chrono::Utc::now().timestamp() - 3600;
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending) VALUES($1,$2,$3,45,30,0,0,true)").bind(Uuid::now_v7()).bind(platform).bind(now).execute(&pool).await.unwrap();
    tokio::time::timeout(Duration::from_secs(4), async {
        loop {
            if capture
                .0
                .lock()
                .unwrap()
                .iter()
                .any(|o| o.resource_id == platform && o.alert_type == "PlatformCpuHigh")
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("delayed partial batch flushes without another input sample");
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let pending: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM platformstats WHERE platformid=$1 AND alertpending",
            )
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
            if pending == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("successfully evaluated samples are acknowledged");

    cancel.cancel();
    worker.await.unwrap().unwrap();
    let status: String = sqlx::query_scalar("SELECT status FROM platforms WHERE id=$1")
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(status, "Offline", "statistics never mark platform online");
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
async fn statistics_failure_is_propagated_and_retry_obeys_shutdown() {
    use std::time::Duration;
    use tokio_util::sync::CancellationToken;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    pool.close().await;
    let store = PostgresContainerStatsStore::new(pool);
    let platform = Uuid::now_v7();
    assert!(
        store.persist_with_disk(platform, &[], None).await.is_err(),
        "a failed persistence attempt must propagate to the writer"
    );
    let cancel = CancellationToken::new();
    let retry = super::platforms::persist_stats_retry(&store, platform, &[], None, &cancel);
    let stop = async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        cancel.cancel();
    };
    let (result, ()) = tokio::time::timeout(Duration::from_millis(200), async {
        tokio::join!(retry, stop)
    })
    .await
    .unwrap();
    assert_eq!(
        result.unwrap_err().kind,
        citadel_platforms::RuntimeErrorKind::Cancelled
    );
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL on disposable PostgreSQL with CREATE DATABASE permission"]
async fn platform_statistics_update_storage_and_counts_without_changing_connectivity() {
    let admin_url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    let database = format!("citadel_stats_metadata_{}", Uuid::now_v7().simple());
    let mut admin = PgConnection::connect(&admin_url).await.unwrap();
    sqlx::query(sqlx::AssertSqlSafe(format!("CREATE DATABASE {database}")))
        .execute(&mut admin)
        .await
        .unwrap();
    let mut url = url::Url::parse(&admin_url).unwrap();
    url.set_path(&database);
    citadel_database::MigrationRunner::migrate(url.as_str())
        .await
        .unwrap();
    let pool = PgPool::connect(url.as_str()).await.unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture','Local',2,0,1024,'Storage fixture',0,'{\"$type\":\"DockerSwarm\",\"nodeID\":\"manager-1\"}','Offline',0)").bind(platform).execute(&pool).await.unwrap();
    let store = PostgresContainerStatsStore::new(pool.clone());
    let mut sample = citadel_platforms::RuntimePlatformStats {
        image_used_bytes: Some(2048),
        volume_used_bytes: Some(4096),
        image_count: 5,
        volume_count: 3,
        network_count: 4,
        mem_total: 8192,
        container_count: 6,
        containers_running: 3,
        containers_paused: 1,
        containers_stopped: 2,
        ..Default::default()
    };
    // No container input is required to flush the platform observation.
    store
        .persist_with_platform_stats(platform, &[], Some(&sample))
        .await
        .unwrap();
    let row: (String, i32, i32, i32, i64, serde_json::Value) = sqlx::query_as("SELECT status,imagecount,volumecount,networkcount,memtotal,platformdescriptor::jsonb FROM platforms WHERE id=$1").bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(
        (&row.0, row.1, row.2, row.3, row.4),
        (&"Offline".to_string(), 5, 3, 4, 8192)
    );
    for (key, value) in [
        ("imageUsedBytes", 2048),
        ("volumeUsedBytes", 4096),
        ("containerCount", 6),
        ("containersRunning", 3),
        ("containersPaused", 1),
        ("containersStopped", 2),
    ] {
        assert_eq!(row.5[key], value);
    }
    assert_eq!(row.5["$type"], "DockerSwarm");
    assert_eq!(row.5["nodeID"], "manager-1");
    sample.image_used_bytes = None;
    sample.volume_used_bytes = None;
    store
        .persist_with_platform_stats(platform, &[], Some(&sample))
        .await
        .unwrap();
    let descriptor: serde_json::Value =
        sqlx::query_scalar("SELECT platformdescriptor::jsonb FROM platforms WHERE id=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(descriptor["imageUsedBytes"].is_null());
    assert!(descriptor["volumeUsedBytes"].is_null());
    pool.close().await;
    sqlx::query(sqlx::AssertSqlSafe(format!("DROP DATABASE {database}")))
        .execute(&mut admin)
        .await
        .unwrap();
}
