use citadel_instance::DatabaseConnections;
use sqlx::PgPool;
use std::time::Duration;

pub(super) async fn snapshot(pool: &PgPool) -> DatabaseConnections {
    // Read on demand, without privileged catalog access or a permanent statement
    // cache entry. External servers/proxies may refuse the setting; pool usage
    // remains available. The timeout includes waiting for a pool connection.
    let shared_buffers_bytes = tokio::time::timeout(
        Duration::from_secs(2),
        sqlx::query_scalar::<_, i64>("SELECT pg_size_bytes(current_setting('shared_buffers'))")
            .persistent(false)
            .fetch_one(pool),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .and_then(|value| u64::try_from(value).ok());

    // Pool counters are approximate under concurrent acquisitions/retirements.
    let open = pool.size();
    let idle = (pool.num_idle().min(open as usize)) as u32;
    DatabaseConnections {
        maximum: pool.options().get_max_connections(),
        open,
        idle,
        borrowed: open - idle,
        shared_buffers_bytes,
    }
}
