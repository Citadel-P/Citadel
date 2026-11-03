use citadel_adapters::persistence::postgres::updates::read_update_database_state;
use citadel_database::MigrationRunner;
use citadel_updates::UpdateError;
use sqlx::postgres::PgPoolOptions;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[tokio::test]
#[ignore = "requires a fresh disposable CITADEL_UPDATE_TEST_DATABASE_URL named citadel_update_test_*"]
async fn preflight_reads_exact_catalog_without_repairing_history_or_ownership() {
    let url = std::env::var("CITADEL_UPDATE_TEST_DATABASE_URL").unwrap();
    assert!(
        url::Url::parse(&url)
            .unwrap()
            .path()
            .starts_with("/citadel_update_test_")
    );
    let pool = PgPoolOptions::new()
        .max_connections(3)
        .connect(&url)
        .await
        .unwrap();
    assert!(sqlx::query_scalar::<_, bool>("SELECT to_regclass('public.citadel_schema_migrations') IS NULL AND to_regclass('public.users') IS NULL")
        .fetch_one(&pool).await.unwrap(), "fresh disposable database required");
    let token = CancellationToken::new();
    assert!(read_update_database_state(&pool, &token).await.is_err());
    assert!(
        sqlx::query_scalar::<_, bool>(
            "SELECT to_regclass('public.citadel_schema_migrations') IS NULL"
        )
        .fetch_one(&pool)
        .await
        .unwrap()
    );
    MigrationRunner::migrate(&url).await.unwrap();
    assert!(read_update_database_state(&pool, &token).await.is_err());
    let instance = Uuid::now_v7();
    sqlx::query(
        "INSERT INTO citadelinstanceidentity(id, instanceid, createdat) VALUES(1,$1,now())",
    )
    .bind(instance)
    .execute(&pool)
    .await
    .unwrap();
    assert!(read_update_database_state(&pool, &token).await.is_err());
    sqlx::query("UPDATE instancesetupstates SET initializedat=now(), initialadministratoractorid=$1 WHERE id=1")
        .bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    let state = read_update_database_state(&pool, &token).await.unwrap();
    assert_eq!(state.instance_id, instance);
    assert_eq!(
        state.catalog_sha256,
        MigrationRunner::catalog_sha256().unwrap()
    );
    assert!(state.database_bytes > 0);
    let original: serde_json::Value =
        sqlx::query_scalar("SELECT json_agg(t ORDER BY id) FROM citadel_schema_migrations t")
            .fetch_one(&pool)
            .await
            .unwrap();
    // Deliberately corrupt only this disposable fixture, then restore fixture rows
    // after asserting the production reader preserves the exact failure evidence.
    for mutation in [
        "DELETE FROM citadel_schema_migrations WHERE id='0002'",
        "UPDATE citadel_schema_migrations SET checksum='altered' WHERE id='0002'",
        "UPDATE citadel_schema_migrations SET completed_at=NULL WHERE id='0002'",
        "UPDATE citadel_schema_migrations SET failure='private-test-detail' WHERE id='0002'",
        "UPDATE citadel_schema_migrations SET id='unknown' WHERE id='0002'",
    ] {
        sqlx::raw_sql(mutation).execute(&pool).await.unwrap();
        let before: serde_json::Value =
            sqlx::query_scalar("SELECT json_agg(t ORDER BY id) FROM citadel_schema_migrations t")
                .fetch_one(&pool)
                .await
                .unwrap();
        let error = read_update_database_state(&pool, &token).await.unwrap_err();
        assert!(!error.to_string().contains("private-test-detail"));
        let after: serde_json::Value =
            sqlx::query_scalar("SELECT json_agg(t ORDER BY id) FROM citadel_schema_migrations t")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(before, after);
        sqlx::query("DELETE FROM citadel_schema_migrations")
            .execute(&pool)
            .await
            .unwrap();
        sqlx::query("INSERT INTO citadel_schema_migrations SELECT * FROM jsonb_populate_recordset(NULL::citadel_schema_migrations,$1)")
            .bind(&original).execute(&pool).await.unwrap();
    }
    let operation = Uuid::now_v7();
    sqlx::query("INSERT INTO instanceupdates(id,instanceid,actorid,idempotencykey,revision,operation) VALUES($1,$2,$3,$1,0,'\"corrupt operation\"'::jsonb)")
        .bind(operation).bind(instance).bind(citadel_identity::SYSTEM_ACTOR_ID).execute(&pool).await.unwrap();
    assert!(matches!(
        read_update_database_state(&pool, &token).await,
        Err(UpdateError::Conflict(_))
    ));
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT ownsinstallation FROM instanceupdates WHERE id=$1")
            .bind(operation)
            .fetch_one(&pool)
            .await
            .unwrap()
    );
    sqlx::query("UPDATE instanceupdates SET ownsinstallation=false WHERE id=$1")
        .bind(operation)
        .execute(&pool)
        .await
        .unwrap();
    assert!(read_update_database_state(&pool, &token).await.is_ok());
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(matches!(
        read_update_database_state(&pool, &cancelled).await,
        Err(UpdateError::Cancelled)
    ));
    let mut lock = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE citadel_schema_migrations IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *lock)
        .await
        .unwrap();
    assert!(
        tokio::time::timeout(
            std::time::Duration::from_secs(8),
            read_update_database_state(&pool, &token)
        )
        .await
        .unwrap()
        .is_err()
    );
    let interrupted = CancellationToken::new();
    let (result, ()) = tokio::join!(read_update_database_state(&pool, &interrupted), async {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        interrupted.cancel();
    });
    assert!(matches!(result, Err(UpdateError::Cancelled)));
    lock.rollback().await.unwrap();
    assert!(read_update_database_state(&pool, &token).await.is_ok());
    pool.close().await;
}
