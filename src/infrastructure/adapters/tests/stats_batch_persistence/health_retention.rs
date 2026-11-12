use super::*;
use citadel_adapters::persistence::postgres::maintenance::cleanup;

#[tokio::test]
#[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
async fn empty_counter_cleanup_serializes_with_commit_rollback_and_cancellation() {
    let (pool, database) = storage_fixture().await;
    let scope = platform(&pool).await;
    let container = add_container(&pool, &scope, "retention-race").await;
    sqlx::query("UPDATE containers SET updated=$2 WHERE id=$1")
        .bind(container)
        .bind(chrono::Utc::now().timestamp())
        .execute(&pool)
        .await
        .unwrap();
    for commit in [true, false] {
        sqlx::query("DELETE FROM containerhealthtransitions WHERE containerid=$1")
            .bind(container)
            .execute(&pool)
            .await
            .unwrap();
        // Leave an empty counter, as age retention does. A writer owns admission
        // while the real cleanup reaches its independent lock statement.
        let mut writer = pool.begin().await.unwrap();
        sqlx::query("SELECT pg_advisory_xact_lock(680224102)")
            .execute(&mut *writer)
            .await
            .unwrap();
        sqlx::query("UPDATE containers SET state=CASE WHEN state='Running' THEN 'Exited' ELSE 'Running' END WHERE id=$1")
            .bind(container).execute(&mut *writer).await.unwrap();
        let cleaning = pool.clone();
        let task = tokio::spawn(async move { cleanup(&cleaning, None).await });
        wait_for_cleanup(&pool).await;
        if commit {
            writer.commit().await.unwrap();
        } else {
            writer.rollback().await.unwrap();
        }
        task.await.unwrap().unwrap();
        let counters: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM containerhealthhistorycounters WHERE containerid=$1",
        )
        .bind(container)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(counters, i64::from(commit));
        if commit {
            let ordinal: i64 = sqlx::query_scalar(
                "SELECT changes FROM containerhealthhistorycounters WHERE containerid=$1",
            )
            .bind(container)
            .fetch_one(&pool)
            .await
            .unwrap();
            assert_eq!(ordinal, 2, "cleanup must preserve the advanced ordinal");
        }
    }
    // Cancellation while waiting rolls back cleanup without touching the writer.
    let mut writer = pool.begin().await.unwrap();
    sqlx::query("UPDATE containers SET state='Paused' WHERE id=$1")
        .bind(container)
        .execute(&mut *writer)
        .await
        .unwrap();
    let cleaning = pool.clone();
    let task = tokio::spawn(async move { cleanup(&cleaning, None).await });
    wait_for_cleanup(&pool).await;
    task.abort();
    let _ = task.await;
    writer.commit().await.unwrap();
    // Exercise the actual insertion trigger's local cap after the race.
    for n in 0..205 {
        sqlx::query("UPDATE containers SET state=$2 WHERE id=$1")
            .bind(container)
            .bind(if n % 2 == 0 { "Running" } else { "Exited" })
            .execute(&pool)
            .await
            .unwrap();
    }
    let (count, distinct, maximum): (i64, i64, i64) = sqlx::query_as("SELECT count(*),count(DISTINCT retentionordinal),max(retentionordinal) FROM containerhealthtransitions WHERE containerid=$1")
        .bind(container).fetch_one(&pool).await.unwrap();
    assert_eq!((count, distinct, maximum), (200, 200, 206));
    sqlx::query("DELETE FROM containers WHERE id=$1")
        .bind(container)
        .execute(&pool)
        .await
        .unwrap();
    cleanup(&pool, None).await.unwrap();
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM containerhealthhistorycounters WHERE containerid=$1",
    )
    .bind(container)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(
        count, 1,
        "deleted containers retain counters while history exists"
    );
    close_storage_fixture(pool, database).await;
}

async fn wait_for_cleanup(pool: &PgPool) {
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let waiting: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND objid=680224102 AND NOT granted AND database=(SELECT oid FROM pg_database WHERE datname=current_database()))")
                    .fetch_one(pool).await.unwrap();
                if waiting { break; }
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        }).await.unwrap();
}
