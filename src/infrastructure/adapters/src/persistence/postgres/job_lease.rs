pub struct CoreJobLease(sqlx::PgConnection);
impl CoreJobLease {
    /// Immediate abandoned-run recovery assumes one Core dispatching against a DB.
    /// Keep an advisory lease for the entire lifecycle so another Core cannot
    /// classify live executions as abandoned during its startup.
    pub async fn acquire(database_url: &str) -> Result<Self, sqlx::Error> {
        use sqlx::Connection;
        let mut connection = sqlx::PgConnection::connect(database_url).await?;
        let acquired: bool = sqlx::query_scalar("SELECT pg_try_advisory_lock(4848495441444547)")
            .fetch_one(&mut connection)
            .await?;
        if !acquired {
            return Err(sqlx::Error::Protocol("Another Citadel Core owns the background jobs for this database. Stop that instance before starting another.".into()));
        }
        Ok(Self(connection))
    }

    pub async fn watch(
        mut self,
        token: tokio_util::sync::CancellationToken,
    ) -> Result<Self, sqlx::Error> {
        loop {
            tokio::select! {
                ()=token.cancelled()=>return Ok(self),
                _=tokio::time::sleep(std::time::Duration::from_secs(5))=>{}
            }
            tokio::select! {
                ()=token.cancelled()=>return Ok(self),
                result=tokio::time::timeout(std::time::Duration::from_secs(2),sqlx::query("SELECT 1").execute(&mut self.0))=>{
                    result.map_err(|_|sqlx::Error::Protocol("Core job lease connection timed out".into()))??;
                }
            }
        }
    }
}
