pub struct CoreJobLease(sqlx::PgConnection);

const JOB_LEASE_KEY: i64 = 4848495441444547;

impl CoreJobLease {
    /// Immediate abandoned-run recovery assumes one Core dispatching against a DB.
    /// Keep an advisory lease for the entire lifecycle so another Core cannot
    /// classify live executions as abandoned during its startup.
    pub async fn acquire(database_url: &str) -> Result<Self, sqlx::Error> {
        use sqlx::Connection;
        let mut connection = sqlx::PgConnection::connect(database_url).await?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock($1)")
            .bind(JOB_LEASE_KEY)
            .fetch_one(&mut connection)
            .await?;
        if !acquired {
            return Err(sqlx::Error::Protocol("Another Citadel Core owns the background jobs for this database. Stop that instance before starting another.".into()));
        }
        Ok(Self(connection))
    }

    /// Check the original session's ownership, never reconnect or reacquire it.
    /// A successful connection probe alone would not detect an unlocked session.
    pub async fn verify(&mut self) -> Result<(), sqlx::Error> {
        let held = tokio::time::timeout(
            std::time::Duration::from_secs(2),
            sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM pg_locks WHERE locktype='advisory' AND pid=pg_backend_pid() AND classid::bigint=$1 AND objid::bigint=$2 AND objsubid=1 AND mode='ExclusiveLock' AND granted)",
            )
            .bind(JOB_LEASE_KEY >> 32)
            .bind(JOB_LEASE_KEY & 0xffff_ffff)
            .fetch_one(&mut self.0),
        )
        .await
        .map_err(|_| sqlx::Error::Protocol("Core job lease verification timed out".into()))??;
        if !held {
            return Err(sqlx::Error::Protocol(
                "Core no longer owns the background job lease".into(),
            ));
        }
        Ok(())
    }

    /// Borrowed monitoring also protects initialization, before a supervisor
    /// exists. Dropping this future leaves the lease with its lifecycle owner.
    pub async fn wait_for_loss(&mut self) -> sqlx::Error {
        loop {
            if let Err(error) = self.verify().await {
                return error;
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    pub async fn watch(
        mut self,
        token: tokio_util::sync::CancellationToken,
    ) -> Result<Self, sqlx::Error> {
        tokio::select! {
            () = token.cancelled() => Ok(self),
            error = self.wait_for_loss() => Err(error),
        }
    }
}
