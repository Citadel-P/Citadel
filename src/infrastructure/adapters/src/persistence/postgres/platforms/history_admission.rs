//! Admission survives cancellation until rollback completes or its I/O deadline expires.
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use citadel_runtime::{DynamicTaskReservation, DynamicTasks};
use sqlx::{PgPool, Postgres, pool::PoolConnection};
use std::{sync::Arc, time::Duration};

use tokio::sync::{OwnedSemaphorePermit, Semaphore};

// Allow the five-second server timeout to finish normally before discarding I/O.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(6);

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
        let result = rollback(self.connection.as_mut().unwrap()).await;
        if let Some(connection) = self.connection.take() {
            release(connection, result.is_ok());
        }
        self.permit.take();
        self.reservation.take();
        result
    }
}
impl Drop for Read {
    fn drop(&mut self) {
        if let Some(mut connection) = self.connection.take() {
            let permit = self.permit.take();
            self.reservation.take().unwrap().spawn(async move {
                // Retain local admission through healthy rollback, but never
                // let a silent transport stall hold the history slots forever.
                let result = rollback(&mut connection).await;
                if let Err(error) = &result {
                    tracing::warn!(%error, "History cleanup failed; discarding connection");
                }
                release(connection, result.is_ok());
                drop(permit);
            });
        }
    }
}
async fn rollback(connection: &mut PoolConnection<Postgres>) -> Result<(), RuntimeCapabilityError> {
    tokio::time::timeout(
        CLEANUP_TIMEOUT,
        sqlx::query("ROLLBACK").execute(&mut **connection),
    )
    .await
    .map_err(|_| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "History cleanup I/O deadline exceeded.",
            true,
        )
    })?
    .map(|_| ())
    .map_err(storage)
}

fn release(connection: PoolConnection<Postgres>, reusable: bool) {
    if reusable {
        drop(connection);
    } else {
        // SQLx 0.9 detach permanently removes this connection from the pool.
        // Dropping the owned PgConnection closes its local transport without an
        // awaited TLS shutdown/Terminate write. Remote backend cleanup can lag.
        drop(connection.detach());
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
    #[tokio::test]
    #[ignore = "requires disposable CITADEL_STATISTICS_DATABASE_URL"]
    async fn silent_transport_releases_slots_and_discards_connections_after_deadline() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        use tokio::net::{TcpListener, TcpStream};
        let mut url =
            url::Url::parse(&std::env::var("CITADEL_STATISTICS_DATABASE_URL").unwrap()).unwrap();
        let upstream = format!("{}:{}", url.host_str().unwrap(), url.port().unwrap_or(5432));
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        url.set_host(Some("127.0.0.1")).unwrap();
        url.set_port(Some(listener.local_addr().unwrap().port()))
            .unwrap();
        url.query_pairs_mut().append_pair("sslmode", "disable");
        let (gate, blocked) = tokio::sync::watch::channel(false);
        let observed = Arc::new(tokio::sync::Notify::new());
        let signal = observed.clone();
        let stop = tokio_util::sync::CancellationToken::new();
        let cancelled = stop.clone();
        let proxy = tokio::spawn(async move {
            let mut connections = tokio::task::JoinSet::new();
            loop {
                tokio::select! {
                    () = cancelled.cancelled() => break,
                    accepted = listener.accept() => {
                        let (client, _) = accepted.unwrap();
                        let upstream = upstream.clone();
                        let mut blocked = blocked.clone();
                        let signal = signal.clone();
                        connections.spawn(async move {
                            let server = TcpStream::connect(upstream).await.unwrap();
                            let (mut cr, mut cw) = client.into_split();
                            let (mut sr, mut sw) = server.into_split();
                            let responses = async {
                                let mut buffer = [0u8; 8192];
                                loop {
                                    let n = sr.read(&mut buffer).await?;
                                    if n == 0 { return Ok::<(), std::io::Error>(()); }
                                    if *blocked.borrow() { signal.notify_one(); }
                                    blocked.wait_for(|blocked| !*blocked).await.unwrap();
                                    cw.write_all(&buffer[..n]).await?;
                                }
                            };
                            tokio::select! {
                                _ = tokio::io::copy(&mut cr, &mut sw) => {},
                                _ = responses => {},
                            }
                        });
                    }
                }
            }
            connections.abort_all();
            while connections.join_next().await.is_some() {}
        });
        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .test_before_acquire(false)
            .connect(url.as_str())
            .await
            .unwrap();
        let tasks = DynamicTasks::new(tokio_util::sync::CancellationToken::new());
        let admission = Arc::new(Admission::new(&pool, tasks.clone()));
        let mut previous_pid = None;
        // Exercise both Drop cleanup and explicit finish, then cancellation during BEGIN.
        for mode in 0..3 {
            let mut read = admission.begin(&pool).await.unwrap();
            let pid: i32 = sqlx::query_scalar("SELECT pg_backend_pid()")
                .fetch_one(read.connection())
                .await
                .unwrap();
            assert_ne!(
                previous_pid,
                Some(pid),
                "uncertain connection must never be reused"
            );
            previous_pid = Some(pid);
            if mode == 2 {
                read.finish().await.unwrap();
                tokio::time::timeout(Duration::from_secs(2), async {
                    while pool.num_idle() == 0 {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
            } else {
                gate.send(true).unwrap();
                if mode == 0 {
                    drop(read);
                } else {
                    let error = read.finish().await.unwrap_err();
                    assert_eq!(error.kind, RuntimeErrorKind::Timeout);
                }
            }
            if mode == 2 {
                gate.send(true).unwrap();
                let admission = admission.clone();
                let pool = pool.clone();
                let setup = tokio::spawn(async move { admission.begin(&pool).await });
                tokio::time::timeout(Duration::from_secs(2), observed.notified())
                    .await
                    .unwrap();
                setup.abort();
                let _ = setup.await;
            }
            tokio::time::timeout(CLEANUP_TIMEOUT + Duration::from_secs(2), async {
                while tasks.active() != 0 || admission.permits.available_permits() != 1 {
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            })
            .await
            .unwrap();
            gate.send(false).unwrap();
            // Consume the proxy's observation before testing the next fault.
            let _ = tokio::time::timeout(Duration::from_millis(10), observed.notified()).await;
        }
        let read = admission.begin(&pool).await.unwrap();
        read.finish().await.unwrap();
        tasks.drain(Duration::from_secs(1)).await.unwrap();
        pool.close().await;
        stop.cancel();
        proxy.await.unwrap();
    }
}
