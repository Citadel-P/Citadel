use crate::persistence::postgres::authorization::AUTHORIZED_CTE;
use citadel_discovery::LookupCaller;
use citadel_discovery::LookupError;
use citadel_discovery::LookupQuery;
use citadel_discovery::LookupReader;
use citadel_discovery::LookupResourceInfo;
use citadel_discovery::LookupResourceType as Kind;
use citadel_discovery::LookupResult;
use citadel_primitives::{ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use sqlx::{AssertSqlSafe, PgPool, Row};
use uuid::Uuid;

const ACCESS: &str = "($4 OR (SELECT allowed FROM global_access) OR EXISTS (SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid WHERE access.resourcetype=$2 AND access.resourceid=resource.id AND (access.permissionlevel & $3)<>0))";

pub struct PostgresLookupStore {
    pool: PgPool,
}
impl PostgresLookupStore {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    async fn rows(
        &self,
        caller: &LookupCaller,
        kind: Kind,
        condition: &str,
        source_id: Option<Uuid>,
    ) -> Result<Vec<LookupResourceInfo>, LookupError> {
        let (table, permission, name, active) = metadata(kind)?;
        let limit = if matches!(kind, Kind::User | Kind::Team | Kind::ServiceAccount) {
            " LIMIT 50"
        } else {
            ""
        };
        // Only closed, code-owned fragments are interpolated. IDs and actor data
        // are always bound, and only id/name are projected (never configuration).
        let sql = format!(
            "{AUTHORIZED_CTE}, lookup_context AS (SELECT $4::boolean AS administrator, $5::uuid AS source_id) SELECT resource.id, {name} AS name FROM {table} resource WHERE {active} AND ({condition}) ORDER BY name, resource.id{limit}"
        );
        sqlx::query(AssertSqlSafe(sql.as_str()))
            .bind(caller.actor_id.value())
            .bind(permission as i32)
            .bind(7_i32)
            .bind(caller.administrator)
            .bind(source_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| {
                Ok(LookupResourceInfo {
                    id: row.try_get("id").map_err(storage)?,
                    name: row.try_get("name").map_err(storage)?,
                    group: None,
                })
            })
            .collect()
    }

    async fn authorize_source(
        &self,
        caller: &LookupCaller,
        kind: Kind,
        id: Uuid,
    ) -> Result<(), LookupError> {
        if !matches!(
            kind,
            Kind::Platform
                | Kind::Deployment
                | Kind::Stack
                | Kind::SwarmService
                | Kind::Alert
                | Kind::User
        ) {
            return Err(LookupError::Validation(
                "Source resource type is not supported for lookups.".into(),
            ));
        }
        let rows = self
            .rows(
                caller,
                kind,
                &format!("resource.id=$5 AND {ACCESS}"),
                Some(id),
            )
            .await?;
        if rows.is_empty() {
            Err(LookupError::NotFound)
        } else {
            Ok(())
        }
    }

    async fn platform_of(&self, kind: Kind, id: Uuid) -> Result<Uuid, LookupError> {
        let sql = match kind {
            Kind::Deployment => "SELECT platformid FROM deployments WHERE id=$1",
            Kind::Stack => {
                "SELECT release.platformid FROM stacks stack JOIN stackreleases release ON release.id=stack.currentstackreleaseid WHERE stack.id=$1"
            }
            _ => return Ok(id),
        };
        sqlx::query_scalar(sql)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(LookupError::NotFound)
    }
}

impl LookupReader for PostgresLookupStore {
    fn lookup<'a>(
        &'a self,
        caller: &'a LookupCaller,
        request: &'a LookupQuery,
    ) -> BoxFuture<'a, Result<LookupResult, LookupError>> {
        Box::pin(async move {
            request.validate(caller.administrator)?;
            if let (Some(kind), Some(id)) = (request.source, request.source_id) {
                self.authorize_source(caller, kind, id).await?;
            }
            use Kind::*;
            if matches!(request.target, Image | Network | Volume) {
                let platform_id = if request.source == Some(Platform)
                    || request.target == Image && request.source == Some(Deployment)
                {
                    self.platform_of(request.source.unwrap(), request.source_id.unwrap())
                        .await?
                } else {
                    let id = request
                        .platform_id
                        .ok_or_else(|| LookupError::Validation("platformId is required.".into()))?;
                    self.authorize_source(caller, Platform, id).await?;
                    id
                };
                return if request.target == Image {
                    Ok(LookupResult::Rows(
                        self.rows(caller, Image, "resource.platformid=$5", Some(platform_id))
                            .await?,
                    ))
                } else {
                    Ok(LookupResult::PlatformResources {
                        platform_id,
                        kind: request.target,
                    })
                };
            }
            if request.target == ResourceBinding {
                let scope = match request.source {
                    Some(Deployment) => "Deployment",
                    Some(Stack) => "Stack",
                    Some(SwarmService) => "SwarmService",
                    _ => "Global",
                };
                let rows = sqlx::query("SELECT DISTINCT ON (lower(name)) id,name FROM resourcebindings WHERE (scope='Global' AND resourceid IS NULL) OR (scope=$1 AND resourceid=$2) ORDER BY lower(name), (scope='Global'),id")
                    .bind(scope).bind(request.source_id).fetch_all(&self.pool).await.map_err(storage)?;
                return Ok(LookupResult::Rows(
                    rows.into_iter()
                        .map(|row| {
                            Ok(LookupResourceInfo {
                                id: row.try_get("id").map_err(storage)?,
                                name: row.try_get("name").map_err(storage)?,
                                group: None,
                            })
                        })
                        .collect::<Result<_, LookupError>>()?,
                ));
            }
            if matches!(request.target, UserActor | RunAsActor) {
                let mut rows = sqlx::query("SELECT actorid AS id,name FROM users WHERE $1 OR id=$2 ORDER BY name,id LIMIT 50")
                    .bind(caller.administrator).bind(caller.user_id).fetch_all(&self.pool).await.map_err(storage)?
                    .into_iter().map(|row| Ok(LookupResourceInfo {id: row.try_get("id").map_err(storage)?,name: row.try_get("name").map_err(storage)?,group: (request.target==RunAsActor).then(|| "Users".into())})).collect::<Result<Vec<_>,LookupError>>()?;
                if request.target == RunAsActor && caller.service_accounts_enabled {
                    let sql = format!(
                        "{AUTHORIZED_CTE} SELECT resource.actorid AS id,resource.name FROM serviceaccounts resource JOIN actors actor ON actor.id=resource.actorid AND actor.isenabled WHERE resource.archivedatutc IS NULL AND ($4 OR EXISTS (SELECT 1 FROM actor_scope scope JOIN actorroles assignment ON assignment.actorid=scope.actorid JOIN permissions p ON p.roleid=assignment.roleid WHERE p.resourcetype=$2 AND (p.permissionlevel & $3)<>0 AND (p.specificpermissions & $5)=$5) OR EXISTS (SELECT 1 FROM actor_scope scope JOIN resourceaccesses a ON a.actorid=scope.actorid WHERE a.resourcetype=$2 AND a.resourceid=resource.id AND (a.permissionlevel & $3)<>0 AND (a.specificpermissions & $5)=$5)) ORDER BY resource.name,resource.id"
                    );
                    let candidates = sqlx::query(AssertSqlSafe(sql.as_str()))
                        .bind(caller.actor_id.value())
                        .bind(ResourceType::ServiceAccount as i32)
                        .bind(7_i32)
                        .bind(caller.administrator)
                        .bind(SpecificPermission::Use as i32)
                        .fetch_all(&self.pool)
                        .await
                        .map_err(storage)?;
                    let mut services = candidates
                        .into_iter()
                        .map(|row| {
                            Ok(LookupResourceInfo {
                                id: row.try_get("id").map_err(storage)?,
                                name: row.try_get("name").map_err(storage)?,
                                group: Some("Service Accounts".into()),
                            })
                        })
                        .collect::<Result<Vec<_>, LookupError>>()?;
                    services.append(&mut rows);
                    rows = services;
                }
                return Ok(LookupResult::Rows(rows));
            }
            if request.target == License {
                let mut transaction = self.pool.begin().await.map_err(storage)?;
                let identity =
                    crate::persistence::postgres::licensing::store::get_or_create_identity(
                        &mut transaction,
                        chrono::Utc::now(),
                    )
                    .await
                    .map_err(|error| LookupError::Storage(error.to_string()))?;
                transaction.commit().await.map_err(storage)?;
                return Ok(LookupResult::Rows(vec![LookupResourceInfo {
                    id: identity.instance_id,
                    name: "License".into(),
                    group: None,
                }]));
            }
            let condition = match (request.source, request.target, request.source_id) {
                (Some(Deployment), Platform, None) => {
                    format!("{ACCESS} AND resource.platformdescriptor->>'$type'<>'DockerSwarm'")
                }
                (Some(Deployment | Stack), Platform, Some(id)) => {
                    let platform = self.platform_of(request.source.unwrap(), id).await?;
                    return Ok(LookupResult::Rows(self.rows(caller, Platform, &format!("resource.id=$5 OR ({ACCESS} AND resource.platformdescriptor->>'$type'=(SELECT platformdescriptor->>'$type' FROM platforms WHERE id=$5))"), Some(platform)).await?));
                }
                (Some(Deployment), Registry, Some(_)) => format!(
                    "{ACCESS} OR resource.id=(SELECT NULLIF(spec->'image'->>'registryId','')::uuid FROM deployments WHERE id=$5 AND spec->'image'->>'$type'='External')"
                ),
                (Some(Stack), Registry, Some(_)) => format!(
                    "{ACCESS} OR resource.id=(SELECT NULLIF(release.spec->>'registryId','')::uuid FROM stacks stack JOIN stackreleases release ON release.id=stack.currentstackreleaseid WHERE stack.id=$5)"
                ),
                (Some(Stack), GitRepository, Some(_)) => format!(
                    "{ACCESS} OR resource.id=(SELECT NULLIF(release.spec->>'gitRepoId','')::uuid FROM stacks stack JOIN stackreleases release ON release.id=stack.currentstackreleaseid WHERE stack.id=$5)"
                ),
                (Some(Platform), Deployment, Some(_)) => {
                    format!("{ACCESS} OR resource.platformid=$5")
                }
                (Some(Platform), Stack, Some(_)) => format!(
                    "{ACCESS} OR EXISTS (SELECT 1 FROM stackreleases release WHERE release.id=resource.currentstackreleaseid AND release.platformid=$5)"
                ),
                (Some(Platform), Registry, Some(_)) => format!(
                    "{ACCESS} OR resource.id IN (SELECT registryid FROM images WHERE platformid=$5)"
                ),
                _ => ACCESS.into(),
            };
            let rows = self
                .rows(caller, request.target, &condition, request.source_id)
                .await?;
            Ok(LookupResult::Rows(rows))
        })
    }
}

fn metadata(
    kind: Kind,
) -> Result<(&'static str, ResourceType, &'static str, &'static str), LookupError> {
    use Kind::*;
    let (table, permission) = match kind {
        Platform => ("platforms", ResourceType::Platform),
        Deployment => ("deployments", ResourceType::Deployment),
        Stack => ("stacks", ResourceType::Stack),
        Registry => ("registries", ResourceType::Registry),
        GitRepository => ("gitrepositories", ResourceType::GitRepository),
        Image => ("images", ResourceType::Platform),
        User => ("users", ResourceType::User),
        Team => ("teams", ResourceType::Team),
        Role => ("roles", ResourceType::Role),
        OidcProvider => ("oidcproviders", ResourceType::User),
        AutomationAction => ("actions", ResourceType::AutomationAction),
        Alert => ("alertrules", ResourceType::Alert),
        AlertChannel => ("alertchannels", ResourceType::AlertChannel),
        BackupRepository => ("backuprepositories", ResourceType::BackupRepository),
        BackupPolicy => ("backuppolicies", ResourceType::BackupPolicy),
        Build => ("buildprojects", ResourceType::Build),
        BuildAgentPool => ("buildagentpools", ResourceType::BuildAgentPool),
        SwarmService => ("swarmservices", ResourceType::SwarmService),
        ServiceAccount => ("serviceaccounts", ResourceType::ServiceAccount),
        _ => return Err(LookupError::Validation("Unsupported lookup target.".into())),
    };
    let active = match kind {
        BackupRepository | BackupPolicy | Build | BuildAgentPool => "resource.archivedat IS NULL",
        ServiceAccount => "resource.archivedatutc IS NULL",
        _ => "TRUE",
    };
    Ok((
        table,
        permission,
        if kind == OidcProvider {
            "resource.displayname"
        } else {
            "resource.name"
        },
        active,
    ))
}
fn storage(error: sqlx::Error) -> LookupError {
    LookupError::Storage(error.to_string())
}
