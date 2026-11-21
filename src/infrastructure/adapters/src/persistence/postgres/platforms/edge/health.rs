use citadel_platforms::jobs::background::{EdgeHealthObservation, EdgeHealthReader};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::{FutureExt, future::BoxFuture};
use uuid::Uuid;
impl EdgeHealthReader for super::store::PostgresEdgeStore {
    fn health<'a>(
        &'a self,
        platforms: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<EdgeHealthObservation>, RuntimeCapabilityError>> {
        async move {
            sqlx::query_as::<_, (Uuid, Uuid, bool)>("SELECT platformid,id,COALESCE(connectionstatus='Connected' AND lastheartbeatatutc>CURRENT_TIMESTAMP-INTERVAL '90 seconds',false) FROM edgeagentbindings WHERE platformid=ANY($1) AND resourcetype='Platform' AND dockernodeid IS NULL AND revokedatutc IS NULL AND connectionstatus<>'Revoked'")
                .bind(platforms).fetch_all(&self.pool).await.map(|rows| rows.into_iter().map(|(platform, binding, healthy)| EdgeHealthObservation {platform, binding, healthy}).collect()).map_err(storage)
        }.boxed()
    }
}

fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}
