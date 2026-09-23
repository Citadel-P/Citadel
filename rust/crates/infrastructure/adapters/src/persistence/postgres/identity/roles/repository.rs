use citadel_activities::{
    ActivityEvent, ActivityEventInfo, RoleActivitySnapshot, RolePermissionActivitySnapshot,
};
use citadel_identity::RoleType;
use citadel_identity::{
    IdentityError, NewRoleMutation, RoleDetails, RolePermissionDetails, RoleReader, RoleRepository,
    role_permissions_expand,
};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType, SpecificPermission};
use futures_util::future::BoxFuture;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;
use crate::persistence::postgres::activities::store::invalid_activity;
use crate::persistence::postgres::identity::users::repository::lock_identity_mutations;

const ROLE_PROJECTION: &str = r#"
SELECT role.id, role.name, role.roletype,
       permission.id AS permissionid,
       permission.resourcetype,
       permission.permissionlevel,
       permission.specificpermissions
FROM roles role
LEFT JOIN permissions permission ON permission.roleid = role.id
"#;

#[derive(Clone)]
pub struct PostgresRoleRepository {
    pool: PgPool,
}

impl PostgresRoleRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl RoleReader for PostgresRoleRepository {
    fn list(&self) -> BoxFuture<'_, Result<Vec<RoleDetails>, IdentityError>> {
        Box::pin(async move {
            let query =
                format!("{ROLE_PROJECTION} ORDER BY role.name, role.id, permission.resourcetype");
            let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            map_roles(rows)
        })
    }

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<RoleDetails>, IdentityError>> {
        Box::pin(async move {
            let query =
                format!("{ROLE_PROJECTION} WHERE role.id = $1 ORDER BY permission.resourcetype");
            let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
            Ok(map_roles(rows)?.pop())
        })
    }
}

impl RoleRepository for PostgresRoleRepository {
    fn create<'a>(
        &'a self,
        role: &'a NewRoleMutation,
    ) -> BoxFuture<'a, Result<RoleDetails, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            ensure_role_name_available(&mut transaction, &role.name, None).await?;
            sqlx::query("INSERT INTO roles (id, name, roletype) VALUES ($1, $2, 'Custom')")
                .bind(role.id)
                .bind(&role.name)
                .execute(&mut *transaction)
                .await
                .map_err(conflict_or_storage)?;
            replace_permissions(&mut transaction, role.id, &role.permissions).await?;
            insert_role_activity(
                &mut transaction,
                role.id,
                &role.name,
                role.changed_by_actor_id,
                ActivityEventInfo::role_created(role_snapshot(RoleType::Custom, &role.permissions)),
                role.changed_at,
            )
            .await?;
            let view = load_role(&mut transaction, role.id, false)
                .await?
                .ok_or_else(missing_persisted_role)?;
            transaction.commit().await.map_err(storage)?;
            Ok(view)
        })
    }

    fn patch_permissions<'a>(
        &'a self,
        id: Uuid,
        permissions: Option<&'a [RolePermissionDetails]>,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<RoleDetails, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let current = load_role(&mut transaction, id, true)
                .await?
                .ok_or(IdentityError::NotFound)?;
            if current.role_type == RoleType::System {
                return Err(IdentityError::Conflict(
                    "System roles cannot be updated.".to_owned(),
                ));
            }
            let proposed = permissions.unwrap_or(&current.permissions);
            if role_permissions_expand(&current.permissions, proposed)
                && !custom_access_control_enabled
            {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            let old = role_snapshot(current.role_type, &current.permissions);
            if permissions.is_some() {
                replace_permissions(&mut transaction, id, proposed).await?;
            }
            let new = role_snapshot(current.role_type, proposed);
            if old != new {
                insert_role_activity(
                    &mut transaction,
                    id,
                    &current.name,
                    changed_by_actor_id,
                    ActivityEventInfo::role_updated(old, new),
                    changed_at,
                )
                .await?;
            }
            let view = load_role(&mut transaction, id, false)
                .await?
                .ok_or_else(missing_persisted_role)?;
            transaction.commit().await.map_err(storage)?;
            Ok(view)
        })
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<RoleDetails, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let current = load_role(&mut transaction, id, true)
                .await?
                .ok_or(IdentityError::NotFound)?;
            if current.role_type == RoleType::System {
                return Err(IdentityError::Conflict(
                    "System roles cannot be updated.".to_owned(),
                ));
            }
            ensure_role_name_available(&mut transaction, name, Some(id)).await?;
            if current.name != name {
                sqlx::query("UPDATE roles SET name = $2 WHERE id = $1")
                    .bind(id)
                    .bind(name)
                    .execute(&mut *transaction)
                    .await
                    .map_err(conflict_or_storage)?;
                insert_role_activity(
                    &mut transaction,
                    id,
                    name,
                    changed_by_actor_id,
                    ActivityEventInfo::role_renamed(current.name, name.to_owned()),
                    changed_at,
                )
                .await?;
            }
            let view = load_role(&mut transaction, id, false)
                .await?
                .ok_or_else(missing_persisted_role)?;
            transaction.commit().await.map_err(storage)?;
            Ok(view)
        })
    }

    fn delete<'a>(
        &'a self,
        ids: &'a [Uuid],
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let roles = load_roles(&mut transaction, ids, true).await?;
            if roles.is_empty() {
                return Err(IdentityError::NotFound);
            }
            if roles.iter().any(|role| role.role_type == RoleType::System) {
                return Err(IdentityError::Conflict(
                    "System roles cannot be deleted.".to_owned(),
                ));
            }
            sqlx::query("DELETE FROM roles WHERE id = ANY($1)")
                .bind(ids)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            for role in roles {
                insert_role_activity(
                    &mut transaction,
                    role.id,
                    &role.name,
                    changed_by_actor_id,
                    ActivityEventInfo::role_deleted(role_snapshot(
                        role.role_type,
                        &role.permissions,
                    )),
                    changed_at,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

async fn ensure_role_name_available(
    transaction: &mut Transaction<'_, Postgres>,
    name: &str,
    exclude_id: Option<Uuid>,
) -> Result<(), IdentityError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM roles WHERE name = $1 AND ($2::uuid IS NULL OR id <> $2))",
    )
    .bind(name)
    .bind(exclude_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    if exists {
        Err(IdentityError::Conflict("Name already exists".to_owned()))
    } else {
        Ok(())
    }
}

async fn replace_permissions(
    transaction: &mut Transaction<'_, Postgres>,
    role_id: Uuid,
    permissions: &[RolePermissionDetails],
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM permissions WHERE roleid = $1")
        .bind(role_id)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    for permission in permissions {
        sqlx::query("INSERT INTO permissions (id, roleid, resourcetype, permissionlevel, specificpermissions) VALUES ($1, $2, $3, $4, $5)")
            .bind(Uuid::now_v7())
            .bind(role_id)
            .bind(permission.resource_type as i32)
            .bind(permission.permission_level as i32)
            .bind(specific_mask(&permission.specific_permissions))
            .execute(&mut **transaction)
            .await
            .map_err(conflict_or_storage)?;
    }
    Ok(())
}

async fn load_role(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
    for_update: bool,
) -> Result<Option<RoleDetails>, IdentityError> {
    let locking = if for_update {
        " FOR UPDATE OF role"
    } else {
        ""
    };
    let query =
        format!("{ROLE_PROJECTION} WHERE role.id = $1 ORDER BY permission.resourcetype{locking}");
    let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(id)
        .fetch_all(&mut **transaction)
        .await
        .map_err(storage)?;
    Ok(map_roles(rows)?.pop())
}

async fn load_roles(
    transaction: &mut Transaction<'_, Postgres>,
    ids: &[Uuid],
    for_update: bool,
) -> Result<Vec<RoleDetails>, IdentityError> {
    let locking = if for_update {
        " FOR UPDATE OF role"
    } else {
        ""
    };
    let query = format!(
        "{ROLE_PROJECTION} WHERE role.id = ANY($1) ORDER BY role.name, role.id, permission.resourcetype{locking}"
    );
    let rows = sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(ids)
        .fetch_all(&mut **transaction)
        .await
        .map_err(storage)?;
    map_roles(rows)
}

fn map_roles(rows: Vec<PgRow>) -> Result<Vec<RoleDetails>, IdentityError> {
    let mut roles: Vec<RoleDetails> = Vec::new();
    for row in rows {
        let id: Uuid = row.try_get("id").map_err(storage)?;
        if roles.last().is_none_or(|role| role.id != id) {
            let role_type_value: String = row.try_get("roletype").map_err(storage)?;
            let role_type = RoleType::from_database_str(&role_type_value).ok_or_else(|| {
                IdentityError::Storage(format!(
                    "unknown persisted RoleType value '{role_type_value}'"
                ))
            })?;
            roles.push(RoleDetails {
                id,
                name: row.try_get("name").map_err(storage)?,
                role_type,
                permissions: Vec::new(),
            });
        }
        if row
            .try_get::<Option<Uuid>, _>("permissionid")
            .map_err(storage)?
            .is_some()
        {
            let resource_value: i32 = row.try_get("resourcetype").map_err(storage)?;
            let level_value: i32 = row.try_get("permissionlevel").map_err(storage)?;
            let mask: i32 = row.try_get("specificpermissions").map_err(storage)?;
            roles
                .last_mut()
                .ok_or_else(missing_persisted_role)?
                .permissions
                .push(RolePermissionDetails {
                    resource_type: ResourceType::from_i32(resource_value).ok_or_else(|| {
                        IdentityError::Storage(format!(
                            "unknown persisted ResourceType value {resource_value}"
                        ))
                    })?,
                    permission_level: PermissionLevel::from_i32(level_value).ok_or_else(|| {
                        IdentityError::Storage(format!(
                            "unknown persisted PermissionLevel value {level_value}"
                        ))
                    })?,
                    specific_permissions: specifics_from_mask(mask)?,
                });
        }
    }
    Ok(roles)
}

fn role_snapshot(
    role_type: RoleType,
    permissions: &[RolePermissionDetails],
) -> RoleActivitySnapshot {
    let mut permissions = permissions
        .iter()
        .map(|permission| RolePermissionActivitySnapshot {
            resource_type: permission.resource_type,
            permission_level: permission.permission_level,
            specific_permissions: specific_mask(&permission.specific_permissions),
        })
        .collect::<Vec<_>>();
    permissions
        .sort_by_key(|permission| (permission.resource_type, permission.permission_level as i32));
    RoleActivitySnapshot {
        role_type: role_type.as_database_str().to_owned(),
        permissions,
    }
}

async fn insert_role_activity(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    actor_id: ActorId,
    info: ActivityEventInfo,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), IdentityError> {
    let event = ActivityEvent::new_role_event(id, name.to_owned(), actor_id, info, created_at)
        .map_err(invalid_activity)?;
    insert_activity(transaction, &event).await
}

fn specific_mask(permissions: &[SpecificPermission]) -> i32 {
    permissions
        .iter()
        .fold(0, |mask, permission| mask | *permission as i32)
}

fn specifics_from_mask(mask: i32) -> Result<Vec<SpecificPermission>, IdentityError> {
    let known_mask = SpecificPermission::ALL
        .into_iter()
        .fold(0, |value, permission| value | permission as i32);
    if mask & !known_mask != 0 {
        return Err(IdentityError::Storage(format!(
            "unknown persisted SpecificPermission mask {mask}"
        )));
    }
    Ok(SpecificPermission::ALL
        .into_iter()
        .filter(|permission| mask & *permission as i32 != 0)
        .collect())
}

fn missing_persisted_role() -> IdentityError {
    IdentityError::Storage("a Role mutation did not leave a readable Role projection".to_owned())
}

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

fn conflict_or_storage(error: sqlx::Error) -> IdentityError {
    if error
        .as_database_error()
        .is_some_and(|database| database.code().as_deref() == Some("23505"))
    {
        IdentityError::Conflict("A Role with the same value already exists.".to_owned())
    } else {
        storage(error)
    }
}
