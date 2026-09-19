pub async fn count(pool: &sqlx::PgPool) -> Option<i64> {
    let enabled: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM pg_extension WHERE extname='pg_stat_statements')",
    )
    .fetch_one(pool)
    .await
    .unwrap();
    assert!(enabled || std::env::var_os("CITADEL_REQUIRE_QUERY_COUNTS").is_none());
    if enabled {
        Some(sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query NOT LIKE '%pg_stat_statements%'")
            .fetch_one(pool).await.unwrap())
    } else {
        None
    }
}

pub async fn assert_single_query(pool: &sqlx::PgPool, before: Option<i64>) {
    if let Some(before) = before {
        // count()'s extension probe is excluded from this assertion.
        let after: i64 = sqlx::query_scalar("SELECT COALESCE(sum(calls),0)::bigint FROM pg_stat_statements WHERE dbid=(SELECT oid FROM pg_database WHERE datname=current_database()) AND query NOT LIKE '%pg_stat_statements%'")
            .fetch_one(pool).await.unwrap();
        assert_eq!(
            after - before,
            1,
            "workload projection must batch all enrichment and ACL metadata"
        );
    }
}
