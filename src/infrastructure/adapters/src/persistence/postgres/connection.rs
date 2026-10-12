use std::time::Duration;

use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres, Transaction};

pub async fn connect_pool(
    database_url: &str,
    max_connections: u32,
    acquire_timeout: Duration,
) -> Result<PgPool, sqlx::Error> {
    assert!(
        max_connections > 0,
        "PostgreSQL pool size must be bounded above zero"
    );
    PgPoolOptions::new()
        .min_connections(0)
        .max_connections(max_connections)
        .acquire_timeout(acquire_timeout)
        .connect(database_url)
        .await
}

pub async fn begin_transaction(pool: &PgPool) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
    pool.begin().await
}

pub struct PostgresDatabaseHealth(pub PgPool);
impl citadel_runtime::database_health::DatabaseHealth for PostgresDatabaseHealth {
    fn healthy(&self) -> futures_util::future::BoxFuture<'_, bool> {
        Box::pin(async move {
            sqlx::query_scalar::<_, i32>("SELECT 1")
                .fetch_one(&self.0)
                .await
                .is_ok()
        })
    }
    fn pool_usage(&self) -> citadel_runtime::database_health::DatabasePoolUsage {
        citadel_runtime::database_health::DatabasePoolUsage {
            connections: self.0.size(),
            idle: self.0.num_idle(),
        }
    }
}
