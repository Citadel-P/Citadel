use citadel_application::{AuthorizedPlatformReader, AuthorizedReadError};
use citadel_domain::{ActorId, PlatformSummary};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Row};

const PLATFORM_RESOURCE_TYPE: i32 = 0;
const READ_PERMISSION_MASK: i32 = 1 | 2 | 4;

const LIST_AUTHORIZED_PLATFORMS: &str = r#"
    WITH ActorScope AS (
        SELECT principalActor.Id AS ActorId
        FROM Actors principalActor
        LEFT JOIN Users principalUser ON principalUser.ActorId = principalActor.Id
        WHERE (principalActor.Id = $1 OR principalUser.Id = $1)
          AND principalActor.IsEnabled

        UNION

        SELECT t.ActorId
        FROM Teams t
        JOIN ActorTeamMemberships membership ON membership.TeamId = t.Id
        JOIN Actors principalActor ON principalActor.Id = membership.MemberActorId
        LEFT JOIN Users principalUser ON principalUser.ActorId = principalActor.Id
        JOIN Actors teamActor ON teamActor.Id = t.ActorId
        WHERE (principalActor.Id = $1 OR principalUser.Id = $1)
          AND principalActor.IsEnabled
          AND teamActor.IsEnabled
    ),
    GlobalAccess AS (
        SELECT 1 AS HasAccess
        FROM ActorRoles ar
        JOIN Permissions p ON p.RoleId = ar.RoleId
        JOIN ActorScope actorScope ON actorScope.ActorId = ar.ActorId
        WHERE p.ResourceType = $2
          AND (p.PermissionLevel & $3) <> 0
          AND ($4 = 0 OR (p.SpecificPermissions & $4) = $4)
        LIMIT 1
    )
    SELECT p.Id, p.Name, p.Address, p.Status, p.ConnectorType
    FROM Platforms p
    WHERE EXISTS (SELECT 1 FROM GlobalAccess)
       OR EXISTS (
            SELECT 1
            FROM ResourceAccesses ra
            JOIN ActorScope actorScope ON actorScope.ActorId = ra.ActorId
            WHERE ra.ResourceType = $2
              AND (ra.PermissionLevel & $3) <> 0
              AND ($4 = 0 OR (ra.SpecificPermissions & $4) = $4)
              AND ra.ResourceId = p.Id
       )
    ORDER BY p.Name
"#;

#[derive(Clone)]
pub struct PostgresAuthorizedPlatformReader {
    pool: PgPool,
}

impl PostgresAuthorizedPlatformReader {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl AuthorizedPlatformReader for PostgresAuthorizedPlatformReader {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
    ) -> BoxFuture<'a, Result<Vec<PlatformSummary>, AuthorizedReadError>> {
        Box::pin(async move {
            let rows = sqlx::query(LIST_AUTHORIZED_PLATFORMS)
                .bind(actor_id.value())
                .bind(PLATFORM_RESOURCE_TYPE)
                .bind(READ_PERMISSION_MASK)
                .bind(0_i32)
                .fetch_all(&self.pool)
                .await
                .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?;

            rows.into_iter()
                .map(|row| {
                    Ok(PlatformSummary {
                        id: row
                            .try_get("id")
                            .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?,
                        name: row
                            .try_get("name")
                            .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?,
                        address: row
                            .try_get("address")
                            .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?,
                        status: row
                            .try_get("status")
                            .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?,
                        connector_type: row
                            .try_get("connectortype")
                            .map_err(|error| AuthorizedReadError::Storage(error.to_string()))?,
                    })
                })
                .collect()
        })
    }
}
