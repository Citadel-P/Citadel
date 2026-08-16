use citadel_application::{
    IdentityError, ResourceInfo, StoredPage, UserReadStore, UserResourceAccessView,
    UserSearchItemView, UserView,
};
use citadel_domain::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Row};
use uuid::Uuid;

const LIST_USERS_SQL: &str = r#"
SELECT u.id, u.name, u.email, u.actorid, actor.isenabled,
       teams.value AS teams, roles.value AS roles
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User'
LEFT JOIN LATERAL (
    SELECT COALESCE(
        jsonb_agg(jsonb_build_object('id', team.id, 'name', team.name)
                  ORDER BY team.name, team.id),
        '[]'::jsonb) AS value
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    WHERE membership.memberactorid = u.actorid
) teams ON TRUE
LEFT JOIN LATERAL (
    SELECT COALESCE(
        jsonb_agg(jsonb_build_object('id', role.id, 'name', role.name)
                  ORDER BY role.name, role.id),
        '[]'::jsonb) AS value
    FROM actorroles assignment
    JOIN roles role ON role.id = assignment.roleid
    WHERE assignment.actorid = u.actorid
) roles ON TRUE
WHERE ($1::text IS NULL OR u.name ILIKE '%' || $1 || '%')
ORDER BY u.name, u.id
LIMIT $2 OFFSET $3
"#;

const GET_USER_SQL: &str = r#"
WITH resource_lookup(resourceid, resourcetype, resourcename) AS (
    SELECT id, 0, name FROM platforms
    UNION ALL SELECT id, 1, name FROM deployments
    UNION ALL SELECT id, 2, name FROM stacks
    UNION ALL SELECT id, 3, name FROM registries
    UNION ALL SELECT id, 4, name FROM gitrepositories
    UNION ALL SELECT id, 6, name FROM alertrules
    UNION ALL SELECT id, 7, name FROM alertchannels
    UNION ALL SELECT id, 12, name FROM tags
    UNION ALL SELECT id, 15, name FROM backuprepositories
    UNION ALL SELECT id, 13, name FROM actions
    UNION ALL SELECT id, 18, name FROM buildprojects
    UNION ALL SELECT id, 19, name FROM buildagentpools
    UNION ALL SELECT id, 20, name FROM swarmservices
)
SELECT u.id, u.name, u.email, u.actorid, actor.isenabled,
       teams.value AS teams, roles.value AS roles,
       COALESCE((
           SELECT jsonb_agg(jsonb_build_object(
               'id', access.id,
               'resourceType', access.resourcetype,
               'resourceId', access.resourceid,
               'resourceName', lookup.resourcename,
               'permissionLevel', access.permissionlevel,
               'specificPermissions', access.specificpermissions)
               ORDER BY access.resourcetype, access.resourceid, access.id)
           FROM resourceaccesses access
           LEFT JOIN resource_lookup lookup
             ON lookup.resourceid = access.resourceid
            AND lookup.resourcetype = access.resourcetype
           WHERE access.actorid = u.actorid
       ), '[]'::jsonb) AS resourceaccesses
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User'
LEFT JOIN LATERAL (
    SELECT COALESCE(
        jsonb_agg(jsonb_build_object('id', team.id, 'name', team.name)
                  ORDER BY team.name, team.id),
        '[]'::jsonb) AS value
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    WHERE membership.memberactorid = u.actorid
) teams ON TRUE
LEFT JOIN LATERAL (
    SELECT COALESCE(
        jsonb_agg(jsonb_build_object('id', role.id, 'name', role.name)
                  ORDER BY role.name, role.id),
        '[]'::jsonb) AS value
    FROM actorroles assignment
    JOIN roles role ON role.id = assignment.roleid
    WHERE assignment.actorid = u.actorid
) roles ON TRUE
WHERE u.id = $1
"#;

#[derive(Clone)]
pub struct PostgresUserReadStore {
    pool: PgPool,
}

impl PostgresUserReadStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserReadStore for PostgresUserReadStore {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<UserView>, IdentityError>> {
        Box::pin(async move {
            let total_items = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM users WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')",
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?;
            let rows = sqlx::query(LIST_USERS_SQL)
                .bind(name)
                .bind(limit)
                .bind(offset)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            Ok(StoredPage {
                total_items,
                items: rows
                    .into_iter()
                    .map(|row| map_user(row, None))
                    .collect::<Result<_, _>>()?,
            })
        })
    }

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<UserView>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(GET_USER_SQL)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(|row| {
                    let accesses = map_accesses(row.try_get("resourceaccesses").map_err(storage)?)?;
                    map_user(row, Some(accesses))
                })
                .transpose()
        })
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<UserSearchItemView>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                r#"
SELECT id, name, email
FROM users
WHERE name ILIKE '%' || $1 || '%' OR email ILIKE '%' || $1 || '%'
ORDER BY name, id
LIMIT $2
"#,
            )
            .bind(query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| {
                Ok(UserSearchItemView {
                    id: row.try_get("id").map_err(storage)?,
                    name: row.try_get("name").map_err(storage)?,
                    email: required_email(&row)?,
                })
            })
            .collect()
        })
    }
}

#[derive(Deserialize)]
struct PersistedResourceInfo {
    id: Uuid,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedAccess {
    id: Uuid,
    resource_type: i32,
    resource_id: Uuid,
    resource_name: Option<String>,
    permission_level: i32,
    specific_permissions: i32,
}

fn map_user(
    row: PgRow,
    resource_accesses: Option<Vec<UserResourceAccessView>>,
) -> Result<UserView, IdentityError> {
    Ok(UserView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        email: required_email(&row)?,
        actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
        is_enabled: row.try_get("isenabled").map_err(storage)?,
        teams: Some(map_resource_info(row.try_get("teams").map_err(storage)?)?),
        roles: Some(map_resource_info(row.try_get("roles").map_err(storage)?)?),
        resource_accesses,
    })
}

fn required_email(row: &PgRow) -> Result<String, IdentityError> {
    row.try_get::<Option<String>, _>("email")
        .map_err(storage)?
        .ok_or_else(|| IdentityError::Storage("a persisted User has no email address".to_owned()))
}

fn map_resource_info(value: serde_json::Value) -> Result<Vec<ResourceInfo>, IdentityError> {
    serde_json::from_value::<Vec<PersistedResourceInfo>>(value)
        .map_err(|error| IdentityError::Storage(error.to_string()))
        .map(|values| {
            values
                .into_iter()
                .map(|value| ResourceInfo {
                    id: value.id,
                    name: value.name,
                    group: None,
                })
                .collect()
        })
}

fn map_accesses(value: serde_json::Value) -> Result<Vec<UserResourceAccessView>, IdentityError> {
    serde_json::from_value::<Vec<PersistedAccess>>(value)
        .map_err(|error| IdentityError::Storage(error.to_string()))?
        .into_iter()
        .map(|access| {
            let resource_type = ResourceType::from_i32(access.resource_type).ok_or_else(|| {
                IdentityError::Storage(format!(
                    "unknown persisted ResourceType value {}",
                    access.resource_type
                ))
            })?;
            let permission_level =
                PermissionLevel::from_i32(access.permission_level).ok_or_else(|| {
                    IdentityError::Storage(format!(
                        "unknown persisted PermissionLevel value {}",
                        access.permission_level
                    ))
                })?;
            Ok(UserResourceAccessView {
                resource_type,
                resource_id: access.resource_id,
                resource_name: access.resource_name,
                permission_level,
                specific_permissions: Some(
                    SpecificPermission::ALL
                        .into_iter()
                        .filter(|permission| access.specific_permissions & *permission as i32 != 0)
                        .collect(),
                ),
                id: Some(access.id),
            })
        })
        .collect()
}

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
