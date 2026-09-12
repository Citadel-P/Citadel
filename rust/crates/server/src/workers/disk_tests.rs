use super::platforms::observe_platform_metrics;
use citadel_adapters::{
    alert_store::PostgresAlertStore, container_stats_store::PostgresContainerStatsStore,
};
use citadel_platforms::{HostDiskUsage, StatisticsReadStore};
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
    let alerts = PostgresAlertStore::new(pool.clone()); // Built-in rules work without paid entitlements.
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
        citadel_adapters::statistics_read_store::PostgresStatisticsReadStore::new(pool.clone())
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
