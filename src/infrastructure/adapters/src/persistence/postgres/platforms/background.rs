use citadel_platforms::jobs::background::{ContainerMaintenanceReader, UnmanagedContainer};
use citadel_platforms::{RuntimeCapabilityError, RuntimeErrorKind};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct PostgresContainerMaintenanceReader(pub PgPool);
impl ContainerMaintenanceReader for PostgresContainerMaintenanceReader {
    fn pruning_candidates(&self) -> BoxFuture<'_, Result<Vec<Uuid>, RuntimeCapabilityError>> {
        async move {
            sqlx::query_scalar("SELECT c.id FROM containers c JOIN platforms p ON p.id=c.platformid WHERE p.prunehistoricalswarmtaskcontainers AND lower(p.platformdescriptor->>'$type')='dockerswarm' AND p.status='Online' AND c.isswarmtask AND NOT c.issystem AND lower(c.state) IN('exited','dead') AND c.controlstate='Idle' AND c.projectionstalesince IS NULL ORDER BY c.updated,c.id LIMIT 100")
                .fetch_all(&self.0).await.map_err(storage)
        }.boxed()
    }
    fn unmanaged_container<'a>(
        &'a self,
        platform: Uuid,
        container: &'a str,
        node: Option<&'a str>,
    ) -> BoxFuture<'a, Result<Option<UnmanagedContainer>, RuntimeCapabilityError>> {
        async move {
            let row = sqlx::query("SELECT c.id,c.name,p.name platformname,p.address FROM containers c JOIN platforms p ON p.id=c.platformid WHERE c.platformid=$1 AND lower(c.dockercontainerid)=$2 AND c.dockernodeid IS NOT DISTINCT FROM $3 AND NOT c.issystem AND NOT c.isswarmtask AND c.deploymentid IS NULL AND c.stackid IS NULL")
                .bind(platform).bind(container).bind(node).fetch_optional(&self.0).await.map_err(storage)?;
            row.map(|r| Ok(UnmanagedContainer { id: r.try_get("id")?, name: r.try_get("name")?, platform_name: r.try_get("platformname")?, platform_address: r.try_get("address")? })).transpose().map_err(storage)
        }.boxed()
    }
}
fn storage(error: sqlx::Error) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}
