use citadel_swarm_services::{ServiceOperationClaim, SwarmServiceError};
use sqlx::PgPool;
use uuid::Uuid;

pub(crate) async fn record_target(
    pool: &PgPool,
    claim: &ServiceOperationClaim,
    hash: &str,
    force: Option<i64>,
) -> Result<(), SwarmServiceError> {
    let changed = sqlx::query("UPDATE swarmservices SET targetruntimehash=$3,expectedforceupdate=$4 WHERE id=$1 AND operationid=$2 AND operationstate='PendingAcceptance'")
            .bind(claim.id).bind(claim.operation_id).bind(hash)
            .bind(force)
            .execute(pool).await.map_err(storage)?.rows_affected();
    if changed != 1 {
        return Err(SwarmServiceError::Conflict(
            "The operation changed before dispatch.".into(),
        ));
    }
    Ok(())
}

pub(crate) type ExpectedOperation = (Option<String>, Option<i64>, Option<i64>);
pub(crate) async fn expected(
    pool: &PgPool,
    id: Uuid,
    operation: Uuid,
) -> Result<Option<ExpectedOperation>, SwarmServiceError> {
    sqlx::query_as("SELECT targetruntimehash,basedockerversion,expectedforceupdate FROM swarmservices WHERE id=$1 AND operationid=$2 AND operationstate IN ('PendingAcceptance','Accepted')")
                .bind(id).bind(operation).fetch_optional(pool).await.map_err(storage)
}
fn storage(error: sqlx::Error) -> SwarmServiceError {
    SwarmServiceError::Storage(error.to_string())
}
