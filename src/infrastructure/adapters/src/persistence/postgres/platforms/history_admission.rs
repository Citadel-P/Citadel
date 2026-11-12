//! Admission survives a cancelled request until PostgreSQL has finished rollback.
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use citadel_runtime::{DynamicTaskReservation, DynamicTasks};
use sqlx::{PgPool, Postgres, pool::PoolConnection};
use std::sync::Arc;
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

pub struct Admission {
    permits: Arc<Semaphore>,
    tasks: DynamicTasks,
}
impl Admission {
    pub fn new(pool: &PgPool, tasks: DynamicTasks) -> Self {
        Self {
            permits: Arc::new(Semaphore::new(
                (pool.options().get_max_connections() as usize).min(2),
            )),
            tasks,
        }
    }
    pub(crate) async fn begin(&self, pool: &PgPool) -> Result<Read, RuntimeCapabilityError> {
        let permit = self
            .permits
            .clone()
            .try_acquire_owned()
            .map_err(|_| capacity("History is busy; retry shortly."))?;
        let reservation = self
            .tasks
            .reserve()
            .ok_or_else(|| capacity("History is shutting down."))?;
        let connection = pool.acquire().await.map_err(storage)?;
        let mut read = Read {
            connection: Some(connection),
            permit: Some(permit),
            reservation: Some(reservation),
        };
        sqlx::raw_sql("BEGIN READ ONLY; SET LOCAL statement_timeout='5s'; SET LOCAL jit=off;")
            .execute(read.connection())
            .await
            .map_err(storage)?;
        Ok(read)
    }
}
pub(crate) struct Read {
    connection: Option<PoolConnection<Postgres>>,
    permit: Option<OwnedSemaphorePermit>,
    reservation: Option<DynamicTaskReservation>,
}
impl Read {
    pub fn connection(&mut self) -> &mut sqlx::PgConnection {
        self.connection.as_mut().unwrap()
    }
    pub async fn finish(mut self) -> Result<(), RuntimeCapabilityError> {
        sqlx::query("ROLLBACK")
            .execute(self.connection())
            .await
            .map_err(storage)?;
        self.connection.take();
        self.permit.take();
        self.reservation.take();
        Ok(())
    }
}
impl Drop for Read {
    fn drop(&mut self) {
        if let Some(mut connection) = self.connection.take() {
            let permit = self.permit.take();
            self.reservation.take().unwrap().spawn(async move {
                // SQLx drains pending responses before issuing ROLLBACK. The
                // server timeout bounds a dropped query; retain admission until
                // ReadyForQuery, not merely until its Rust future disappears.
                if let Err(error) = sqlx::query("ROLLBACK").execute(&mut *connection).await {
                    tracing::warn!(%error, "History cleanup failed; closing connection");
                    let _ = connection.close().await;
                }
                drop(permit);
            });
        }
    }
}
fn capacity(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::ResourceExhausted, message, true)
}
pub(crate) fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    let kind = if error
        .as_database_error()
        .is_some_and(|e| e.code().as_deref() == Some("57014"))
    {
        RuntimeErrorKind::Timeout
    } else {
        RuntimeErrorKind::Remote
    };
    RuntimeCapabilityError::new(kind, error.to_string(), kind == RuntimeErrorKind::Timeout)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    #[tokio::test]
    #[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
    async fn cancellation_holds_admission_until_backend_finishes_and_settings_are_reset() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let observer = PgPool::connect(&url).await.unwrap();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect(&url)
            .await
            .unwrap();
        let tasks = DynamicTasks::new(tokio_util::sync::CancellationToken::new());
        let admission = Admission::new(&pool, tasks.clone());
        let mut read = admission.begin(&pool).await.unwrap();
        let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
            .fetch_one(read.connection())
            .await
            .unwrap();
        sqlx::query("SET LOCAL statement_timeout='1s'")
            .execute(read.connection())
            .await
            .unwrap();
        let query = tokio::spawn(async move {
            sqlx::query("SELECT pg_sleep(30) /* history_cancel_fixture */")
                .execute(read.connection())
                .await
        });
        tokio::time::timeout(Duration::from_secs(5), async {
            loop {
                let active: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND wait_event='PgSleep')").bind(pid).fetch_one(&observer).await.unwrap();
                if active { break; }
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }).await.unwrap();
        query.abort();
        let _ = query.await;
        assert!(
            admission.begin(&pool).await.is_err(),
            "cancelled SQL still owns admission"
        );
        assert_eq!(tasks.active(), 1);
        tasks.drain(Duration::from_secs(5)).await.unwrap();
        let active: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE pid=$1 AND state='active')",
        )
        .bind(pid)
        .fetch_one(&observer)
        .await
        .unwrap();
        assert!(!active);
        assert_eq!(admission.permits.available_permits(), 1);
        let (timeout, jit): (String, String) =
            sqlx::query_as("SELECT current_setting('statement_timeout'),current_setting('jit')")
                .fetch_one(&pool)
                .await
                .unwrap();
        assert_eq!(timeout, "0");
        assert_eq!(jit, "on");
        pool.close().await;
        observer.close().await;
    }

    #[tokio::test]
    #[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
    async fn timeout_is_typed_and_success_and_error_release_admission() {
        let url = std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap();
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await
            .unwrap();
        let tasks = DynamicTasks::new(tokio_util::sync::CancellationToken::new());
        let admission = Admission::new(&pool, tasks.clone());
        let mut first = admission.begin(&pool).await.unwrap();
        let second = admission.begin(&pool).await.unwrap();
        assert!(admission.begin(&pool).await.is_err());
        second.finish().await.unwrap();
        sqlx::query("SET LOCAL statement_timeout='50ms'")
            .execute(first.connection())
            .await
            .unwrap();
        let error = sqlx::query("SELECT pg_sleep(5)")
            .execute(first.connection())
            .await
            .unwrap_err();
        assert_eq!(storage(error).kind, RuntimeErrorKind::Timeout);
        drop(first);
        tasks.drain(Duration::from_secs(5)).await.unwrap();
        assert_eq!(admission.permits.available_permits(), 2);
        pool.close().await;
    }
}
