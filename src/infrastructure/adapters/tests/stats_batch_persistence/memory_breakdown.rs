use super::*;

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn memory_breakdown_survives_capture_correction_compaction_and_missing_history() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let id = add_container(&pool, &scope, "memory").await;
    let deployment = Uuid::now_v7();
    sqlx::query("INSERT INTO deployments(id,name,platformid,spec,status,createdbyactorid) VALUES($1,'memory',$2,'{}','Healthy','00000000-0000-0000-0000-000000000001')")
        .bind(deployment).bind(scope.platform_id).execute(&pool).await.unwrap();
    sqlx::query("UPDATE containers SET deploymentid=$1 WHERE id=$2")
        .bind(deployment)
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let start = chrono::Utc::now().timestamp() / 60 * 60 - 120;
    let store = PostgresStatsBatchStore::new(pool.clone());
    let mut samples: Vec<_> = (0..12)
        .map(|i| {
            let mut s = container(&scope, "memory", start + i * 10, 1.);
            s.sample.memory_active_cache = Some(20. + i as f64);
            s
        })
        .collect();
    store.persist_batch(&samples).await.unwrap();
    samples[0].sample.memory_active_cache = Some(30.);
    store.persist_batch(&samples[..1]).await.unwrap();
    let source: i64 = sqlx::query_scalar("SELECT id FROM statisticssources WHERE containerid=$1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    async fn history(pool: &PgPool, id: Uuid, start: i64) -> Vec<Option<f64>> {
        sqlx::query(include_str!(
            "../../src/persistence/postgres/platforms/statistics/container_history.sql"
        ))
        .bind(vec![id])
        .bind(start)
        .bind(60i64)
        .bind(start + 119)
        .fetch_all(pool)
        .await
        .unwrap()
        .iter()
        .map(|r| r.get("memoryactivecache"))
        .collect()
    }
    let before = history(&pool, id, start).await;
    assert_eq!(before, vec![Some(145. / 6.), Some(28.5)]);
    sqlx::query(include_str!(
        "../../src/persistence/postgres/platforms/statistics/compact.sql"
    ))
    .bind(source)
    .bind(start)
    .bind(start + 120)
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(history(&pool, id, start).await, before);
    let rows = sqlx::query(include_str!(
        "../../src/persistence/postgres/platforms/statistics/workload_history.sql"
    ))
    .bind(deployment)
    .bind(true)
    .bind(start)
    .bind(start + 119)
    .bind(60i64)
    .fetch_all(&pool)
    .await
    .unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| r.get::<Option<f64>, _>("memoryactivecache"))
            .collect::<Vec<_>>(),
        before
    );
    // Missing coverage in a rollup cannot be averaged as zero or ignored.
    sqlx::query(
        "UPDATE statisticsrollups SET memoryactivecache=NULL WHERE sourceid=$1 AND created=$2",
    )
    .bind(source)
    .bind(start)
    .execute(&pool)
    .await
    .unwrap();
    let row = sqlx::query(include_str!(
        "../../src/persistence/postgres/platforms/statistics/container_history.sql"
    ))
    .bind(vec![id])
    .bind(start)
    .bind(3600i64)
    .bind(start + 119)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(row.get::<Option<f64>, _>("memoryactivecache"), None);
    assert_eq!(row.get::<f64, _>("memoryactive"), 100.);
    let mixed: Vec<_> = (0..6)
        .map(|i| {
            let mut sample = container(&scope, "memory", start + 120 + i * 10, 1.);
            sample.sample.memory_active_cache = if i == 0 { None } else { Some(25.) };
            sample
        })
        .collect();
    store.persist_batch(&mixed).await.unwrap();
    sqlx::query(include_str!(
        "../../src/persistence/postgres/platforms/statistics/compact.sql"
    ))
    .bind(source)
    .bind(start + 120)
    .bind(start + 180)
    .execute(&pool)
    .await
    .unwrap();
    let incomplete: Option<f64> = sqlx::query_scalar(
        "SELECT memoryactivecache FROM statisticsrollups WHERE sourceid=$1 AND created=$2",
    )
    .bind(source)
    .bind(start + 120)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(incomplete, None);
    close_storage_fixture(pool, database).await;
}
