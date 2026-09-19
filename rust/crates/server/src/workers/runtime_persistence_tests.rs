use super::*;
use citadel_adapters::container_stats_store::PostgresContainerStatsStore;
use sqlx::{Connection, PgConnection, PgPool};
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL with CREATE DATABASE permission"]
async fn stats_writes_cannot_delete_history_and_maintenance_keeps_pending_alerts() {
    let admin_url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    let database = format!("citadel_runtime_{}", Uuid::now_v7().simple());
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
    let id = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,'unix:///fixture','Local',2,0,1024,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)").bind(id).execute(&pool).await.unwrap();
    for pending in [true, false] {
        sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending) VALUES($1,$2,$3,1,1,0,0,$4)")
            .bind(Uuid::now_v7()).bind(id).bind(chrono::Utc::now().timestamp()-700_000-i64::from(pending)).bind(pending).execute(&pool).await.unwrap();
    }
    sqlx::raw_sql("CREATE FUNCTION reject_hot_retention() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'stats writes must not delete history'; END $$").execute(&pool).await.unwrap();
    for table in ["containerstats", "platformstats", "swarmservicestats"] {
        sqlx::query(sqlx::AssertSqlSafe(format!("CREATE TRIGGER reject_hot_retention BEFORE DELETE ON {table} FOR EACH STATEMENT EXECUTE FUNCTION reject_hot_retention()"))).execute(&pool).await.unwrap();
    }
    let store = PostgresContainerStatsStore::new(pool.clone());
    store
        .persist_with_platform_stats(
            id,
            &[],
            Some(&citadel_platforms::RuntimePlatformStats {
                mem_total: 1024,
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    for table in ["containerstats", "platformstats", "swarmservicestats"] {
        sqlx::query(sqlx::AssertSqlSafe(format!(
            "DROP TRIGGER reject_hot_retention ON {table}"
        )))
        .execute(&pool)
        .await
        .unwrap();
    }
    assert_eq!(
        citadel_adapters::maintenance_store::cleanup(&pool, None)
            .await
            .unwrap(),
        1
    );
    let old_pending: i64 = sqlx::query_scalar("SELECT count(*) FROM platformstats WHERE platformid=$1 AND created<EXTRACT(EPOCH FROM CURRENT_TIMESTAMP)-604800 AND alertpending").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!(old_pending, 1);
    // Unchanged slow metadata must not rewrite the Platform tuple each sample.
    let before: String = sqlx::query_scalar("SELECT xmin::text FROM platforms WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    store
        .persist_with_platform_stats(
            id,
            &[],
            Some(&citadel_platforms::RuntimePlatformStats {
                mem_total: 1024,
                ..Default::default()
            }),
        )
        .await
        .unwrap();
    let after: String = sqlx::query_scalar("SELECT xmin::text FROM platforms WHERE id=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
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
async fn alert_evaluation_does_not_lock_samples_or_acknowledge_concurrent_upserts() {
    use citadel_alerts::{AlertEventSink, AlertObservation};
    use std::{sync::Arc, time::Duration};
    struct Blocked {
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
        first: std::sync::atomic::AtomicBool,
    }
    impl AlertEventSink for Blocked {
        fn observe<'a>(
            &'a self,
            _: &'a AlertObservation,
        ) -> futures_util::future::BoxFuture<
            'a,
            Result<Option<citadel_alerts::AlertEventView>, citadel_alerts::AlertError>,
        > {
            Box::pin(async move {
                if !self.first.swap(true, std::sync::atomic::Ordering::SeqCst) {
                    self.entered.notify_one();
                    self.release.notified().await;
                }
                Ok(None)
            })
        }
    }
    let url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let id = Uuid::now_v7();
    let sample = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'Local',2,0,1024,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)").bind(id).execute(&pool).await.unwrap();
    // Earliest possible timestamp ensures this fixture owns the first bounded batch.
    sqlx::query("INSERT INTO platformstats(id,platformid,created,cpuusage,memoryusage,rxbytes,txbytes,alertpending) VALUES($1,$2,-9223372036854775808,10,20,0,0,true)").bind(sample).bind(id).execute(&pool).await.unwrap();
    let alerts = Arc::new(Blocked {
        entered: Default::default(),
        release: Default::default(),
        first: Default::default(),
    });
    let (worker_pool, worker_alerts) = (pool.clone(), alerts.clone());
    let flush = tokio::spawn(async move {
        stats_alerts::flush_pending(&worker_pool, worker_alerts.as_ref(), 1).await
    });
    tokio::time::timeout(Duration::from_secs(5), alerts.entered.notified())
        .await
        .unwrap();
    assert_eq!(
        stats_alerts::flush_pending(&pool, alerts.as_ref(), 1)
            .await
            .unwrap(),
        0,
        "a second flusher cannot evaluate an active claim"
    );
    tokio::time::timeout(
        Duration::from_secs(1),
        sqlx::query("UPDATE platformstats SET cpuusage=90,alertpending=true WHERE id=$1")
            .bind(sample)
            .execute(&pool),
    )
    .await
    .expect("alert I/O must not hold a sample row lock")
    .unwrap();
    alerts.release.notify_one();
    flush.await.unwrap().unwrap();
    let pending: bool = sqlx::query_scalar("SELECT alertpending FROM platformstats WHERE id=$1")
        .bind(sample)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(pending, "the concurrent revision must remain pending");
    stats_alerts::flush_pending(&pool, alerts.as_ref(), 1)
        .await
        .unwrap();
    assert!(
        !sqlx::query_scalar::<_, bool>("SELECT alertpending FROM platformstats WHERE id=$1")
            .bind(sample)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_TEST_DATABASE_URL"]
async fn targeted_container_discovery_preserves_siblings_identity_fencing_and_notifications() {
    use std::time::Duration;
    let url = std::env::var("CITADEL_TEST_DATABASE_URL").unwrap();
    citadel_database::MigrationRunner::migrate(&url)
        .await
        .unwrap();
    let pool = PgPool::connect(&url).await.unwrap();
    let platform = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,address,connectortype,cpucount,imagecount,memtotal,name,networkcount,platformdescriptor,status,volumecount) VALUES($1,$1::text,'Local',1,0,1024,$1::text,0,'{\"$type\":\"Docker\"}','Online',0)").bind(platform).execute(&pool).await.unwrap();
    let mut notifications = super::listener(&pool, "citadel_container_created")
        .await
        .unwrap();
    let mut container = citadel_platforms::RuntimeContainerSummary {
        id: "observed-new".into(),
        name: "first".into(),
        image: "image".into(),
        image_id: String::new(),
        created: 1,
        state: "created".into(),
        status: "Created".into(),
        labels: Default::default(),
        ports: serde_json::json!([]),
        stack: None,
        is_system: false,
        system_role: None,
        has_citadel_ownership_labels: false,
        is_swarm_task: false,
    };
    assert!(
        citadel_adapters::resource_status_store::container_observation(
            &pool, platform, None, &container, 10
        )
        .await
        .unwrap()
    );
    let notification = tokio::time::timeout(Duration::from_secs(2), notifications.recv())
        .await
        .unwrap()
        .unwrap();
    let payload: serde_json::Value = serde_json::from_str(notification.payload()).unwrap();
    assert_eq!(payload["container"], "observed-new");
    let image: String = sqlx::query_scalar("SELECT dockerimageid FROM containers WHERE platformid=$1 AND dockercontainerid='observed-new'")
        .bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(
        image, "image",
        "a partial observation preserves the image-name fallback"
    );
    let persisted: Uuid = sqlx::query_scalar(
        "SELECT id FROM containers WHERE platformid=$1 AND dockercontainerid='observed-new'",
    )
    .bind(platform)
    .fetch_one(&pool)
    .await
    .unwrap();
    let mut sibling = container.clone();
    sibling.id = "sibling".into();
    sibling.name = "sibling".into();
    assert!(
        citadel_adapters::resource_status_store::container_observation(
            &pool, platform, None, &sibling, 11
        )
        .await
        .unwrap()
    );
    container.name = "renamed".into();
    container.state = "exited".into();
    assert!(
        citadel_adapters::resource_status_store::container_observation(
            &pool, platform, None, &container, 20
        )
        .await
        .unwrap()
    );
    container.name = "stale".into();
    assert!(
        !citadel_adapters::resource_status_store::container_observation(
            &pool, platform, None, &container, 5
        )
        .await
        .unwrap()
    );
    let current: (Uuid, String, String) = sqlx::query_as("SELECT id,name,state FROM containers WHERE platformid=$1 AND dockercontainerid='observed-new'").bind(platform).fetch_one(&pool).await.unwrap();
    assert_eq!(current, (persisted, "renamed".into(), "Exited".into()));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap(),
        2
    );
    assert!(
        citadel_adapters::resource_status_store::container_event(
            &pool,
            platform,
            None,
            "observed-new",
            None,
            None,
            21
        )
        .await
        .unwrap()
    );
    let survivor: String =
        sqlx::query_scalar("SELECT dockercontainerid FROM containers WHERE platformid=$1")
            .bind(platform)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(survivor, "sibling");
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    drop(notifications);
    pool.close().await;
}
