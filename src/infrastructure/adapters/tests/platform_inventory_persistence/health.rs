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
    let health = citadel_adapters::connectors::docker::container_health::observation(
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
            .container_health_history(container)
            .await
            .unwrap()
            .len(),
        200
    );
    sqlx::query("UPDATE containerhealthtransitions SET recordedat=now()-interval '8 days' WHERE containerid=$1").bind(container).execute(&pool).await.unwrap();
    assert!(
        reader
            .container_health_history(container)
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
    let mut health = citadel_adapters::connectors::docker::container_health::observation(
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
    let entries = reader.container_health_history(container).await.unwrap();
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
            .container_health_history(container)
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
    let entries = reader.container_health_history(container).await.unwrap();
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
