use super::*;
use citadel_platforms::jobs::observations::RuntimeProjectionStore;
use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite, SnapshotGeneration};

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn health_transitions_are_bounded_deduplicated_and_fenced() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')").bind(platform).execute(&pool).await.unwrap();
    let health = citadel_docker::client::container_health::observation(
        &json!({"Config":{"Healthcheck":{"Test":["CMD","probe"]}},"State":{"Health":{"Status":"healthy"}}}),
    );
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,health,projectionobservedat) VALUES($1,$2,'full-id','image','health',1,$4,'Running','[]',$3,$4)").bind(container).bind(platform).bind(serde_json::to_value(&health).unwrap()).bind(health.observed_at / 1000).execute(&pool).await.unwrap();
    for i in 0..5 {
        let mut next = health.clone();
        next.observed_at += i;
        sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
            .bind(container)
            .bind(serde_json::to_value(next).unwrap())
            .execute(&pool)
            .await
            .unwrap();
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1")
            .bind(container)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1, "freshness changes are not health transitions");
    let stamp = SnapshotGeneration::capture(platform, None, ProjectionKind::Containers).await;
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    assert_eq!(
        store
            .invalidate_container_health(platform, None)
            .await
            .unwrap(),
        vec!["full-id"]
    );
    assert!(
        !stamp.matches(&ProjectionWrite::begin(platform, None, ProjectionKind::Containers).await)
    );
    let value: Option<serde_json::Value> =
        sqlx::query_scalar("SELECT health FROM containers WHERE id=$1")
            .bind(container)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(value.is_none(), "stream gaps invalidate current health");
    let mut replay = health.clone();
    replay.observed_at = 1000;
    let change = store
        .container_observation(
            platform,
            None,
            &RuntimeContainerSummary {
                id: "full-id".into(),
                image_id: "image".into(),
                name: "health".into(),
                state: "running".into(),
                health: Some(replay),
                ..Default::default()
            },
            Utc::now().timestamp() + 2,
        )
        .await
        .unwrap();
    assert!(
        !change.changed(),
        "late delivery must not refresh pre-gap inspection evidence"
    );
    for i in 0..205 {
        sqlx::query("UPDATE containers SET state=$2 WHERE id=$1")
            .bind(container)
            .bind(if i % 2 == 0 { "Exited" } else { "Running" })
            .execute(&pool)
            .await
            .unwrap();
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1")
            .bind(container)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 200);
    let reader = PostgresPlatformReader::new(pool.clone());
    assert_eq!(
        reader
            .container_health_history(
                container,
                citadel_platforms::StatsWindow::new(168).unwrap(),
                chrono::Utc::now().timestamp()
            )
            .await
            .unwrap()
            .len(),
        200
    );
    sqlx::query("UPDATE containerhealthtransitions SET recordedat=now()-interval '8 days' WHERE containerid=$1").bind(container).execute(&pool).await.unwrap();
    assert!(
        reader
            .container_health_history(
                container,
                citadel_platforms::StatsWindow::new(168).unwrap(),
                chrono::Utc::now().timestamp()
            )
            .await
            .unwrap()
            .is_empty()
    );
    citadel_adapters::persistence::postgres::maintenance::cleanup(&pool, None)
        .await
        .unwrap();
    let retained: i64 =
        sqlx::query_scalar("SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1")
            .bind(container)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(
        retained, 0,
        "scheduled retention also purges quiet/offline history"
    );
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn periodic_health_samples_preserve_failures_and_use_actual_inspection_times() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    // Align away from minute boundaries without waiting on wall time.
    let now = Utc::now().timestamp_millis() / 60000 * 60000;
    let mut health = citadel_docker::client::container_health::observation(
        &json!({"Config":{"Healthcheck":{"Test":["CMD","probe"]}},"State":{"Health":{"Status":"healthy"}}}),
    );
    health.observed_at = now - 120000;
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')").bind(platform).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO containers(id,platformid,dockercontainerid,dockerimageid,name,created,updated,state,ports,health,projectionobservedat) VALUES($1,$2,'sample-id','image','health',1,$4,'Running','[]',$3,$4)").bind(container).bind(platform).bind(serde_json::to_value(&health).unwrap()).bind(now/1000-120).execute(&pool).await.unwrap();
    health.observed_at = now - 60000;
    for _ in 0..3 {
        sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
            .bind(container)
            .bind(serde_json::to_value(&health).unwrap())
            .execute(&pool)
            .await
            .unwrap();
    }
    let reader = PostgresPlatformReader::new(pool.clone());
    let entries = reader
        .container_health_history(
            container,
            citadel_platforms::StatsWindow::new(168).unwrap(),
            chrono::Utc::now().timestamp(),
        )
        .await
        .unwrap();
    assert_eq!(
        entries.len(),
        2,
        "unchanged health adds a sample, repeated delivery does not"
    );
    assert_eq!(entries[0].origin, "Sample");
    assert_eq!(
        entries[0].observed_at,
        now / 1000 - 60,
        "sample time is the inspection time, not the earlier projection time"
    );
    assert_eq!(entries[0].sample_count, 1);
    for stamp in [now + 600000, now - 600000] {
        let mut tx = pool.begin().await.unwrap();
        let mut invalid = health.clone();
        invalid.observed_at = stamp;
        sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
            .bind(container)
            .bind(serde_json::to_value(invalid).unwrap())
            .execute(&mut *tx)
            .await
            .unwrap();
        let count: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM containerhealthtransitions WHERE containerid=$1",
        )
        .bind(container)
        .fetch_one(&mut *tx)
        .await
        .unwrap();
        assert_eq!(
            count, 2,
            "future/stale evidence must not manufacture samples"
        );
        tx.rollback().await.unwrap();
    }
    for status in [
        citadel_platforms::container_health::HealthStatus::Unhealthy,
        citadel_platforms::container_health::HealthStatus::Healthy,
    ] {
        health.result = Some(status);
        sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
            .bind(container)
            .bind(serde_json::to_value(&health).unwrap())
            .execute(&pool)
            .await
            .unwrap();
    }
    assert_eq!(
        reader
            .container_health_history(
                container,
                citadel_platforms::StatsWindow::new(168).unwrap(),
                chrono::Utc::now().timestamp()
            )
            .await
            .unwrap()
            .len(),
        4,
        "rapid failure and recovery stay exact"
    );
    let bucket = (now / 1000 - 7200) / 300 * 300;
    // Seed retained older samples to test read aggregation without simulating a clock or runtime.
    for (offset, status) in [
        (0, "Healthy"),
        (60, "Healthy"),
        (120, "Unhealthy"),
        (180, "Healthy"),
    ] {
        let mut evidence = serde_json::to_value(&health).unwrap();
        evidence["result"] = json!(status);
        evidence["observedAt"] = json!((bucket + offset) * 1000);
        sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,health,origin) VALUES($1,$2,'sample-id',$3,'Running',$4,'Sample')").bind(container).bind(platform).bind(bucket+offset).bind(evidence).execute(&pool).await.unwrap();
    }
    let entries = reader
        .container_health_history(
            container,
            citadel_platforms::StatsWindow::new(168).unwrap(),
            chrono::Utc::now().timestamp(),
        )
        .await
        .unwrap();
    assert_eq!(entries.len(), 6);
    let group = entries
        .iter()
        .find(|entry| entry.sample_count == 3)
        .unwrap();
    assert_eq!(
        (group.first_observed_at, group.observed_at),
        (bucket, bucket + 180)
    );
    assert!(entries.iter().any(|entry| entry.observed_at == bucket + 120
        && entry.sample_count == 1
        && entry.health.as_ref().unwrap().result
            == Some(citadel_platforms::container_health::HealthStatus::Unhealthy)));
    // Sampling has its own per-container budget; it cannot consume the 200 transition slots.
    sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,health,origin) SELECT $1,$2,'sample-id',$3-i*60,'Running',$4,'Sample' FROM generate_series(1,10085) i").bind(container).bind(platform).bind(now/1000).bind(serde_json::to_value(&health).unwrap()).execute(&pool).await.unwrap();
    health.observed_at = now;
    sqlx::query("UPDATE containers SET health=$2 WHERE id=$1")
        .bind(container)
        .bind(serde_json::to_value(&health).unwrap())
        .execute(&pool)
        .await
        .unwrap();
    let counts: (i64,i64) = sqlx::query_as("SELECT count(*) FILTER(WHERE origin='Sample'),count(*) FILTER(WHERE origin<>'Sample') FROM containerhealthtransitions WHERE containerid=$1").bind(container).fetch_one(&pool).await.unwrap();
    assert_eq!(counts, (10080, 3));
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
    pool.close().await;
}

#[tokio::test]
#[ignore = "requires CITADEL_PLATFORM_DATABASE_URL"]
async fn compose_service_identity_uses_inventory_labels_and_clears_removed_labels() {
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(4)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    seed_platform(&pool, platform, Uuid::now_v7(), Uuid::now_v7()).await;
    let mut inventory = snapshot(platform, false);
    inventory.containers[0]
        .labels
        .insert("com.docker.compose.service".into(), "api".into());
    let store = PostgresInventoryProjectionStore::new(pool.clone());
    store.persist(&inventory).await.unwrap();
    let value: Option<String> = sqlx::query_scalar(
        "SELECT composeservicename FROM containers WHERE platformid=$1 AND dockercontainerid=$2",
    )
    .bind(platform)
    .bind(&inventory.containers[0].id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(value.as_deref(), Some("api"));
    inventory.observed_at += chrono::Duration::seconds(2);
    inventory.containers[0]
        .labels
        .remove("com.docker.compose.service");
    store.persist(&inventory).await.unwrap();
    let value: Option<String> = sqlx::query_scalar(
        "SELECT composeservicename FROM containers WHERE platformid=$1 AND dockercontainerid=$2",
    )
    .bind(platform)
    .bind(&inventory.containers[0].id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        value, None,
        "do not retain or guess a removed Compose identity"
    );
}

#[tokio::test]
#[ignore = "requires disposable CITADEL_PLATFORM_DATABASE_URL"]
async fn health_history_filters_before_grouping_without_temporary_writes() {
    use sqlx::Row;
    let url = std::env::var("CITADEL_PLATFORM_DATABASE_URL").unwrap();
    MigrationRunner::migrate(&url).await.unwrap();
    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let platform = Uuid::now_v7();
    let container = Uuid::now_v7();
    let now = Utc::now().timestamp() / 60 * 60;
    sqlx::query("INSERT INTO platforms(id,name,address,connectortype,cpucount,imagecount,memtotal,networkcount,volumecount,platformdescriptor,status) VALUES($1,$1::text,$1::text,'Local',1,0,1024,0,0,'{\"$type\":\"Docker\"}','Online')")
        .bind(platform).execute(&pool).await.unwrap();
    // One week for five ordinary containers. Batch fixtures to avoid repeatedly
    // updating one counter through a long chain in a single seeding transaction.
    for id in [
        container,
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
        Uuid::now_v7(),
    ] {
        for start in (0..10080).step_by(480) {
            sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,health,origin)
                SELECT $1,$2,'history-fixture',$3-n*60,'Running',jsonb_build_object(
                'configuration','Enabled','result',CASE WHEN n%101=0 THEN 'Unhealthy' ELSE 'Healthy' END,
                'observedAt',($3::bigint-n*60)*1000,'lastCheckAt','2026-01-01T00:00:00Z',
                'failingStreak',0,'exitCode',0,'oomKilled',false,'restartCount',0,
                'startedAt','2026-01-01T00:00:00Z','finishedAt',NULL),'Sample'
                FROM generate_series($4::integer,$4::integer+479)n")
                .bind(id).bind(platform).bind(now).bind(start).execute(&pool).await.unwrap();
        }
    }
    sqlx::raw_sql("ANALYZE containerhealthtransitions")
        .execute(&pool)
        .await
        .unwrap();
    let sql = include_str!(
        "../../src/persistence/postgres/platforms/reader/container_health_history.sql"
    );
    let reader = PostgresPlatformReader::new(pool.clone());
    for mode in ["force_custom_plan", "force_generic_plan"] {
        let mut tx = pool.begin().await.unwrap();
        sqlx::raw_sql(sqlx::AssertSqlSafe(format!("SET LOCAL work_mem='4MB'; SET LOCAL jit=off; SET LOCAL plan_cache_mode={mode}; PREPARE health_range(uuid,bigint,bigint,bigint) AS {sql}")))
            .execute(&mut *tx).await.unwrap();
        for minutes in [15, 60, 360, 1440, 10080] {
            let from = now - i64::from(minutes) * 60;
            let execute = format!("EXECUTE health_range('{container}',{from},{now},{now})");
            let plan: serde_json::Value = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
                "EXPLAIN(ANALYZE,BUFFERS,TIMING OFF,FORMAT JSON) {execute}"
            )))
            .fetch_one(&mut *tx)
            .await
            .unwrap();
            assert_eq!(
                plan[0]["Plan"]["Temp Written Blocks"], 0,
                "{mode}, {minutes}: {plan}"
            );
            let rows = sqlx::query(sqlx::AssertSqlSafe(execute))
                .fetch_all(&mut *tx)
                .await
                .unwrap();
            assert_eq!(
                rows.iter()
                    .map(|r| r.get::<i64, _>("samplecount"))
                    .sum::<i64>(),
                i64::from(minutes + 1).min(10080)
            );
            assert!(
                rows.iter()
                    .all(|r| r.get::<i64, _>("firstobservedat") >= from
                        && r.get::<i64, _>("observedat") <= now)
            );
        }
        sqlx::query("DEALLOCATE health_range")
            .execute(&mut *tx)
            .await
            .unwrap();
        tx.rollback().await.unwrap();
    }
    // A partial older five-minute bucket must not count evidence outside the range.
    let from = now - 2 * 3600 + 17;
    let until = from + 143;
    let rows = reader
        .container_health_history(
            container,
            citadel_platforms::StatsWindow::between(from, until, now).unwrap(),
            now,
        )
        .await
        .unwrap();
    assert_eq!(rows.iter().map(|r| r.sample_count).sum::<i64>(), 2);
    assert!(
        rows.iter()
            .all(|r| r.first_observed_at >= from && r.observed_at <= until)
    );
    // Equal timestamps choose the newest identity, while a later insertion with
    // an older observation must not replace the latest representative payload.
    for (offset, restarts) in [(1_i64, 11), (1, 22), (0, 33)] {
        sqlx::query("INSERT INTO containerhealthtransitions(containerid,platformid,dockercontainerid,observedat,state,health,origin) VALUES($1,$2,'history-fixture',$3,'Running',$4,'Sample')")
            .bind(container).bind(platform).bind(from + offset)
            .bind(json!({"configuration":"Enabled","result":"Healthy","observedAt":(from+offset)*1000,"restartCount":restarts}))
            .execute(&pool).await.unwrap();
    }
    let rows = reader
        .container_health_history(
            container,
            citadel_platforms::StatsWindow::between(from, from + 1, now).unwrap(),
            now,
        )
        .await
        .unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(
        (
            rows[0].first_observed_at,
            rows[0].observed_at,
            rows[0].sample_count
        ),
        (from, from + 1, 3)
    );
    assert_eq!(rows[0].health.as_ref().unwrap().restart_count, Some(22));
    sqlx::query("DELETE FROM platforms WHERE id=$1")
        .bind(platform)
        .execute(&pool)
        .await
        .unwrap();
}
