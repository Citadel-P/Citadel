use super::*;
use citadel_platforms::{PlatformReader, StatisticsReader, StatsWindow};
use std::{sync::Arc, time::Duration};

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn unchanged_swarm_refresh_writes_each_projection_once_and_removes_absent_resources() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let mut inventory = snapshot(platform, true);
    let service = &mut inventory.swarm.as_mut().unwrap().services[0];
    service.labels.clear();
    *service = service.clone().normalize_ownership();
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    let schema = format!("overhead_{}", platform.simple());
    let tables = [
        "swarmnodeprojections",
        "swarmserviceprojections",
        "swarmtaskprojections",
        "swarmconfigprojections",
        "swarmsecretprojections",
        "swarmnetworkprojections",
    ];
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE SCHEMA {schema}; CREATE TABLE {schema}.writes(resource text); CREATE FUNCTION {schema}.record() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN IF NEW.platformid='{platform}'::uuid THEN INSERT INTO {schema}.writes VALUES(TG_TABLE_NAME); END IF; RETURN NEW; END; $$;")))
        .execute(&pool).await.unwrap();
    for table in tables {
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE TRIGGER {schema} AFTER UPDATE ON {table} FOR EACH ROW EXECUTE FUNCTION {schema}.record()")))
            .execute(&pool).await.unwrap();
    }
    // Column-specific triggers distinguish full payload rewrites from timestamp-only writes.
    for table in tables {
        let column = if table == "swarmtaskprojections" {
            "ports"
        } else {
            "labels"
        };
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("CREATE TRIGGER {schema}_payload AFTER UPDATE OF {column} ON {table} FOR EACH ROW EXECUTE FUNCTION {schema}.record()")))
            .execute(&pool).await.unwrap();
    }
    inventory.observed_at += chrono::Duration::seconds(1);
    store.persist(&inventory).await.unwrap();
    let counts: Vec<(String, i64)> = sqlx::query_as(sqlx::AssertSqlSafe(format!(
        "SELECT resource,count(*) FROM {schema}.writes GROUP BY resource ORDER BY resource"
    )))
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(counts.len(), 6);
    assert!(
        counts.iter().all(|(_, count)| *count == 1),
        "unchanged inventory should only advance freshness: {counts:?}"
    );
    for table in tables {
        let observed: chrono::DateTime<Utc> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
            "SELECT observedat FROM {table} WHERE platformid=$1"
        )))
        .bind(platform)
        .fetch_one(&pool)
        .await
        .unwrap();
        // PostgreSQL timestamps retain microseconds; Rust observations can include nanoseconds.
        assert_eq!(
            observed.timestamp_micros(),
            inventory.observed_at.timestamp_micros()
        );
    }
    let older = inventory.clone();
    inventory.observed_at += chrono::Duration::seconds(1);
    inventory.swarm.as_mut().unwrap().nodes[0]
        .labels
        .insert("changed".into(), "yes".into());
    store.persist(&inventory).await.unwrap();
    store.persist(&older).await.unwrap();
    store
        .mark_swarm_stale(platform, older.observed_at)
        .await
        .unwrap();
    let (labels, stale): (serde_json::Value, bool) =
        sqlx::query_as("SELECT labels,isstale FROM swarmnodeprojections WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(labels["changed"], "yes");
    assert!(!stale);
    inventory.observed_at += chrono::Duration::seconds(1);
    inventory.swarm.as_mut().unwrap().tasks.clear();
    inventory.swarm.as_mut().unwrap().services.clear();
    store.persist(&inventory).await.unwrap();
    let counts: (i32, i32) = sqlx::query_as(
        "SELECT runningtaskcount,desiredtaskcount FROM swarmnodeprojections WHERE platformid=$1",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (0, 0));
    let names: serde_json::Value =
        sqlx::query_scalar("SELECT servicenames FROM swarmconfigprojections WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(names, json!([]));
    let tasks: i64 =
        sqlx::query_scalar("SELECT count(*) FROM swarmtaskprojections WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(tasks, 0);
    sqlx::raw_sql(sqlx::AssertSqlSafe(format!("DROP SCHEMA {schema} CASCADE")))
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn health_and_statistics_share_admission_without_blocking_ordinary_reads() {
    use citadel_adapters::persistence::postgres::platforms::{
        history_admission::Admission, statistics::reader::PostgresStatisticsReader,
    };
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let tasks = citadel_runtime::DynamicTasks::new(tokio_util::sync::CancellationToken::new());
    let admission = Arc::new(Admission::new(&pool, tasks.clone()));
    let reader = Arc::new(PostgresPlatformReader::with_admission(
        pool.clone(),
        admission.clone(),
    ));
    let stats = PostgresStatisticsReader::with_admission(pool.clone(), admission);
    let mut blocker = pool.begin().await.unwrap();
    let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
        .fetch_one(&mut *blocker)
        .await
        .unwrap();
    sqlx::query("LOCK TABLE containerhealthtransitions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let mut reads = Vec::new();
    for _ in 0..2 {
        let reader = reader.clone();
        reads.push(tokio::spawn(async move {
            reader.container_health_history(Uuid::now_v7()).await
        }));
    }
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM pg_stat_activity WHERE $1=ANY(pg_blocking_pids(pid))",
            )
            .bind(pid)
            .fetch_one(&pool)
            .await
            .unwrap();
            if count == 2 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let error = stats
        .platform(
            platform,
            StatsWindow::new(1).unwrap(),
            Utc::now().timestamp(),
        )
        .await
        .unwrap_err();
    assert_eq!(
        error.kind,
        citadel_platforms::RuntimeErrorKind::ResourceExhausted
    );
    assert_eq!(
        sqlx::query_scalar::<_, i32>("SELECT 1")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    for read in &reads {
        read.abort();
    }
    for read in reads {
        let _ = read.await;
    }
    blocker.rollback().await.unwrap();
    tokio::time::timeout(Duration::from_secs(6), async {
        while tasks.active() != 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    assert!(
        reader
            .container_health_history(Uuid::now_v7())
            .await
            .unwrap()
            .is_empty()
    );
    tasks.drain(Duration::from_secs(6)).await.unwrap();
    pool.close().await;
}
