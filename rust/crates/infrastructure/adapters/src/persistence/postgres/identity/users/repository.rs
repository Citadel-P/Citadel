use crate::persistence::postgres::identity::authorization_cache::{Impact, Mutation};
use crate::persistence::postgres::identity::resource_access::{
    expands_resource_access, map_accesses, specific_mask,
};
use citadel_activities::{
    ActivityEvent, ActivityEventInfo, IdentityResourceAccessSnapshot, UserActivitySnapshot,
};
use citadel_identity::{
    IdentityError, NewUserMutation, ResourceAccessDetails, ResourceAccessInput, ResourceInfo,
    StoredPage, UserDetails, UserPasswordContext, UserPatchMutation, UserReader, UserRepository,
    UserSearchItemDetails,
};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;
use crate::persistence::postgres::activities::store::invalid_activity;

const USER_MUTATION_LOCK_ID: i64 = 4_859_382_590_227_736_942;

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
pub struct PostgresUserRepository {
    pool: PgPool,
}

impl PostgresUserRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl UserReader for PostgresUserRepository {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<UserDetails>, IdentityError>> {
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

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<UserDetails>, IdentityError>> {
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
    ) -> BoxFuture<'a, Result<Vec<UserSearchItemDetails>, IdentityError>> {
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
                Ok(UserSearchItemDetails {
                    id: row.try_get("id").map_err(storage)?,
                    name: row.try_get("name").map_err(storage)?,
                    email: required_email(&row)?,
                })
            })
            .collect()
        })
    }
}

impl UserRepository for PostgresUserRepository {
    fn password_context(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserPasswordContext>, IdentityError>> {
        Box::pin(async move {
            sqlx::query("SELECT name, email FROM users WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(|row| {
                    Ok(UserPasswordContext {
                        name: row.try_get("name").map_err(storage)?,
                        email: required_email(&row)?,
                    })
                })
                .transpose()
        })
    }

    fn create<'a>(
        &'a self,
        user: &'a NewUserMutation,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(vec![user.id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            ensure_user_name_and_email_available(&mut transaction, &user.name, &user.email, None)
                .await?;
            validate_assignment_targets(
                &mut transaction,
                &user.team_ids,
                &user.role_ids,
                &[],
                &[],
                custom_access_control_enabled,
            )
            .await?;
            if !user.resource_accesses.is_empty() && !custom_access_control_enabled {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }

            sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, $2, 'User')")
                .bind(user.actor_id.value())
                .bind(user.is_enabled)
                .execute(&mut *transaction)
                .await
                .map_err(conflict_or_storage)?;
            sqlx::query(
                r#"
INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name, password)
VALUES ($1, $2, $3, $4, $5, $6, $7)
"#,
            )
            .bind(user.id)
            .bind(user.actor_id.value())
            .bind(user.created_at)
            .bind(user.created_by_actor_id.value())
            .bind(&user.email)
            .bind(&user.name)
            .bind(&user.password_hash)
            .execute(&mut *transaction)
            .await
            .map_err(conflict_or_storage)?;
            replace_teams(&mut transaction, user.actor_id, &user.team_ids).await?;
            replace_roles(&mut transaction, user.actor_id, &user.role_ids).await?;
            replace_resource_accesses(&mut transaction, user.actor_id, &user.resource_accesses)
                .await?;

            let snapshot = load_user_snapshot(&mut transaction, user.id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            let activity = ActivityEvent::new_user_event(
                user.id,
                user.name.clone(),
                user.created_by_actor_id,
                ActivityEventInfo::user_created(snapshot),
                user.created_at,
            )
            .map_err(invalid_activity)?;
            insert_activity(&mut transaction, &activity).await?;
            let view = fetch_user_view(&mut transaction, user.id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn patch<'a>(
        &'a self,
        id: Uuid,
        patch: &'a UserPatchMutation,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_user_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old_snapshot = load_user_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            let email = patch.email.as_deref().unwrap_or(&state.email);
            ensure_user_name_and_email_available(&mut transaction, &state.name, email, Some(id))
                .await?;

            let current_team_ids = old_snapshot.team_ids.clone();
            let current_role_ids = old_snapshot.role_ids.clone();
            if patch.team_ids.is_some() || patch.role_ids.is_some() {
                validate_assignment_targets(
                    &mut transaction,
                    patch.team_ids.as_deref().unwrap_or(&current_team_ids),
                    patch.role_ids.as_deref().unwrap_or(&current_role_ids),
                    &current_team_ids,
                    &current_role_ids,
                    custom_access_control_enabled,
                )
                .await?;
            }
            if let Some(accesses) = &patch.resource_accesses
                && !custom_access_control_enabled
                && expands_resource_access(&old_snapshot.resource_accesses, accesses)
            {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }

            if let Some(team_ids) = &patch.team_ids {
                replace_teams(&mut transaction, state.actor_id, team_ids).await?;
            }
            if let Some(role_ids) = &patch.role_ids {
                replace_roles(&mut transaction, state.actor_id, role_ids).await?;
            }
            if let Some(accesses) = &patch.resource_accesses {
                replace_resource_accesses(&mut transaction, state.actor_id, accesses).await?;
            }
            sqlx::query(
                r#"
UPDATE users
SET email = COALESCE($2, email), password = COALESCE($3, password)
WHERE id = $1
"#,
            )
            .bind(id)
            .bind(&patch.email)
            .bind(&patch.password_hash)
            .execute(&mut *transaction)
            .await
            .map_err(conflict_or_storage)?;
            if let Some(is_enabled) = patch.is_enabled {
                sqlx::query("UPDATE actors SET isenabled = $2 WHERE id = $1")
                    .bind(state.actor_id.value())
                    .bind(is_enabled)
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?;
            }
            if patch.password_hash.is_some() || patch.is_enabled == Some(false) {
                sqlx::query("DELETE FROM refreshtokens WHERE userid = $1")
                    .bind(id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?;
            }
            if patch.team_ids.is_some()
                || patch.role_ids.is_some()
                || patch.is_enabled == Some(false)
            {
                ensure_enabled_administrator_remains(&mut transaction).await?;
            }

            let new_snapshot = load_user_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            if old_snapshot != new_snapshot || patch.password_hash.is_some() {
                let activity = ActivityEvent::new_user_event(
                    id,
                    state.name,
                    changed_by_actor_id,
                    ActivityEventInfo::user_updated(
                        old_snapshot,
                        new_snapshot,
                        patch.password_hash.is_some(),
                    ),
                    changed_at,
                )
                .map_err(invalid_activity)?;
                insert_activity(&mut transaction, &activity).await?;
            }
            let view = fetch_user_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_user_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            ensure_user_name_and_email_available(&mut transaction, name, &state.email, Some(id))
                .await?;
            if state.name != name {
                sqlx::query("UPDATE users SET name = $2 WHERE id = $1")
                    .bind(id)
                    .bind(name)
                    .execute(&mut *transaction)
                    .await
                    .map_err(conflict_or_storage)?;
                let activity = ActivityEvent::new_user_event(
                    id,
                    name.to_owned(),
                    changed_by_actor_id,
                    ActivityEventInfo::user_renamed(state.name, name.to_owned()),
                    changed_at,
                )
                .map_err(invalid_activity)?;
                insert_activity(&mut transaction, &activity).await?;
            }
            let view = fetch_user_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            transaction.commit().await.map_err(storage)?;
            Ok(view)
        })
    }

    fn add_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_user_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old_snapshot = load_user_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            let role_type =
                sqlx::query_scalar::<_, String>("SELECT roletype FROM roles WHERE id = $1")
                    .bind(role_id)
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(storage)?
                    .ok_or(IdentityError::NotFound)?;
            if role_type == "Custom" && !custom_access_control_enabled {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            let inserted = sqlx::query(
                "INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(state.actor_id.value())
            .bind(role_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if inserted == 0 {
                return Err(IdentityError::Conflict(
                    "The Role is already assigned to the User.".to_owned(),
                ));
            }
            record_user_update(
                &mut transaction,
                id,
                state.name,
                old_snapshot,
                false,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_user_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn remove_role(
        &self,
        id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'_, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_user_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old_snapshot = load_user_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            if !role_exists(&mut transaction, role_id).await? {
                return Err(IdentityError::NotFound);
            }
            let deleted = sqlx::query("DELETE FROM actorroles WHERE actorid = $1 AND roleid = $2")
                .bind(state.actor_id.value())
                .bind(role_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?
                .rows_affected();
            if deleted == 0 {
                return Err(IdentityError::NotFound);
            }
            ensure_enabled_administrator_remains(&mut transaction).await?;
            record_user_update(
                &mut transaction,
                id,
                state.name,
                old_snapshot,
                false,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_user_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn add_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a ResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            if !custom_access_control_enabled {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_user_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old_snapshot = load_user_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            let inserted = sqlx::query(
                r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT (resourcetype, resourceid, actorid) DO NOTHING
"#,
            )
            .bind(Uuid::now_v7())
            .bind(state.actor_id.value())
            .bind(access.permission_level as i32)
            .bind(access.resource_id)
            .bind(access.resource_type as i32)
            .bind(specific_mask(&access.specific_permissions))
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if inserted == 0 {
                return Err(IdentityError::Conflict(
                    "The resource access is already assigned to the User.".to_owned(),
                ));
            }
            record_user_update(
                &mut transaction,
                id,
                state.name,
                old_snapshot,
                false,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_user_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn remove_resource_access<'a>(
        &'a self,
        id: Uuid,
        access: &'a ResourceAccessInput,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<UserDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_user_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old_snapshot = load_user_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            let deleted = sqlx::query(
                r#"
DELETE FROM resourceaccesses
WHERE actorid = $1 AND resourcetype = $2 AND resourceid = $3
  AND permissionlevel = $4 AND specificpermissions = $5
"#,
            )
            .bind(state.actor_id.value())
            .bind(access.resource_type as i32)
            .bind(access.resource_id)
            .bind(access.permission_level as i32)
            .bind(specific_mask(&access.specific_permissions))
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if deleted == 0 {
                return Err(IdentityError::NotFound);
            }
            record_user_update(
                &mut transaction,
                id,
                state.name,
                old_snapshot,
                false,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_user_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_user)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
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
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Users(ids.to_vec());
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let rows =
                sqlx::query("SELECT id, name FROM users WHERE id = ANY($1) ORDER BY name, id")
                    .bind(ids)
                    .fetch_all(&mut *transaction)
                    .await
                    .map_err(storage)?;
            if rows.is_empty() {
                return Err(IdentityError::NotFound);
            }
            let mut deleted_users = Vec::with_capacity(rows.len());
            for row in rows {
                let id = row.try_get("id").map_err(storage)?;
                let name = row.try_get("name").map_err(storage)?;
                let snapshot = load_user_snapshot(&mut transaction, id)
                    .await?
                    .ok_or_else(missing_persisted_user)?;
                deleted_users.push((id, name, snapshot));
            }
            sqlx::query("DELETE FROM users WHERE id = ANY($1)")
                .bind(ids)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            ensure_enabled_administrator_remains(&mut transaction).await?;
            for (id, name, snapshot) in deleted_users {
                let activity = ActivityEvent::new_user_event(
                    id,
                    name,
                    changed_by_actor_id,
                    ActivityEventInfo::user_deleted(snapshot),
                    changed_at,
                )
                .map_err(invalid_activity)?;
                insert_activity(&mut transaction, &activity).await?;
            }
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(())
        })
    }
}

#[derive(Debug)]
struct UserState {
    name: String,
    email: String,
    actor_id: ActorId,
}

pub(crate) async fn lock_identity_mutations(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), IdentityError> {
    sqlx::query("SELECT pg_advisory_xact_lock($1)")
        .bind(USER_MUTATION_LOCK_ID)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    Ok(())
}

async fn load_user_state(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<UserState>, IdentityError> {
    sqlx::query("SELECT name, email, actorid FROM users WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .map(|row| {
            Ok(UserState {
                name: row.try_get("name").map_err(storage)?,
                email: row
                    .try_get::<Option<String>, _>("email")
                    .map_err(storage)?
                    .ok_or_else(|| {
                        IdentityError::Storage("a persisted User has no email address".to_owned())
                    })?,
                actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
            })
        })
        .transpose()
}

async fn ensure_user_name_and_email_available(
    transaction: &mut Transaction<'_, Postgres>,
    name: &str,
    email: &str,
    exclude_id: Option<Uuid>,
) -> Result<(), IdentityError> {
    let row = sqlx::query(
        r#"
SELECT EXISTS (
           SELECT 1 FROM users WHERE name = $1 AND ($3::uuid IS NULL OR id <> $3)
       ) AS name_exists,
       EXISTS (
           SELECT 1 FROM users WHERE email = $2 AND ($3::uuid IS NULL OR id <> $3)
       ) AS email_exists
"#,
    )
    .bind(name)
    .bind(email)
    .bind(exclude_id)
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    if row.try_get("name_exists").map_err(storage)? {
        return Err(IdentityError::Conflict("Name already exists".to_owned()));
    }
    if row.try_get("email_exists").map_err(storage)? {
        return Err(IdentityError::Conflict("Email already exists".to_owned()));
    }
    Ok(())
}

async fn validate_assignment_targets(
    transaction: &mut Transaction<'_, Postgres>,
    team_ids: &[Uuid],
    role_ids: &[Uuid],
    current_team_ids: &[Uuid],
    current_role_ids: &[Uuid],
    custom_access_control_enabled: bool,
) -> Result<(), IdentityError> {
    if !team_ids.is_empty() {
        let rows = sqlx::query(
            r#"
SELECT team.id,
       EXISTS (
           SELECT 1 FROM actorroles assignment
           JOIN roles role ON role.id = assignment.roleid
           WHERE assignment.actorid = team.actorid AND role.roletype = 'Custom'
       ) OR EXISTS (
           SELECT 1 FROM resourceaccesses access WHERE access.actorid = team.actorid
       ) AS requires_custom_access
FROM teams team
WHERE team.id = ANY($1)
"#,
        )
        .bind(team_ids)
        .fetch_all(&mut **transaction)
        .await
        .map_err(storage)?;
        if rows.len() != team_ids.len() {
            return Err(IdentityError::NotFound);
        }
        if !custom_access_control_enabled
            && rows.iter().any(|row| {
                let id = row.try_get::<Uuid, _>("id").expect("selected Team ID");
                !current_team_ids.contains(&id)
                    && row
                        .try_get::<bool, _>("requires_custom_access")
                        .expect("selected custom-access flag")
            })
        {
            return Err(IdentityError::LicenseRequired("custom-access-control"));
        }
    }
    if !role_ids.is_empty() {
        let rows = sqlx::query("SELECT id, roletype FROM roles WHERE id = ANY($1)")
            .bind(role_ids)
            .fetch_all(&mut **transaction)
            .await
            .map_err(storage)?;
        if rows.len() != role_ids.len() {
            return Err(IdentityError::NotFound);
        }
        if !custom_access_control_enabled
            && rows.iter().any(|row| {
                let id = row.try_get::<Uuid, _>("id").expect("selected Role ID");
                !current_role_ids.contains(&id)
                    && row
                        .try_get::<String, _>("roletype")
                        .expect("selected Role type")
                        == "Custom"
            })
        {
            return Err(IdentityError::LicenseRequired("custom-access-control"));
        }
    }
    Ok(())
}

async fn role_exists(
    transaction: &mut Transaction<'_, Postgres>,
    role_id: Uuid,
) -> Result<bool, IdentityError> {
    sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM roles WHERE id = $1)")
        .bind(role_id)
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)
}

async fn replace_teams(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    team_ids: &[Uuid],
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM actorteammemberships WHERE memberactorid = $1")
        .bind(actor_id.value())
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    if !team_ids.is_empty() {
        sqlx::query(
            "INSERT INTO actorteammemberships (memberactorid, teamid) SELECT $1, id FROM unnest($2::uuid[]) AS id",
        )
        .bind(actor_id.value())
        .bind(team_ids)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    }
    Ok(())
}

async fn replace_roles(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    role_ids: &[Uuid],
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM actorroles WHERE actorid = $1")
        .bind(actor_id.value())
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    if !role_ids.is_empty() {
        sqlx::query(
            "INSERT INTO actorroles (actorid, roleid) SELECT $1, id FROM unnest($2::uuid[]) AS id",
        )
        .bind(actor_id.value())
        .bind(role_ids)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    }
    Ok(())
}

async fn replace_resource_accesses(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    accesses: &[ResourceAccessInput],
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM resourceaccesses WHERE actorid = $1")
        .bind(actor_id.value())
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    for access in accesses {
        sqlx::query(
            r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, $3, $4, $5, $6)
"#,
        )
        .bind(Uuid::now_v7())
        .bind(actor_id.value())
        .bind(access.permission_level as i32)
        .bind(access.resource_id)
        .bind(access.resource_type as i32)
        .bind(specific_mask(&access.specific_permissions))
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    }
    Ok(())
}

async fn load_user_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<Option<UserActivitySnapshot>, IdentityError> {
    let Some(row) = sqlx::query(
        "SELECT email, actorid, isenabled FROM users JOIN actors ON actors.id = users.actorid WHERE users.id = $1",
    )
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?
    else {
        return Ok(None);
    };
    let actor_id = row.try_get::<Uuid, _>("actorid").map_err(storage)?;
    let team_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT teamid FROM actorteammemberships WHERE memberactorid = $1 ORDER BY teamid",
    )
    .bind(actor_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    let role_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT roleid FROM actorroles WHERE actorid = $1 ORDER BY roleid",
    )
    .bind(actor_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    let resource_accesses = sqlx::query(
        r#"
SELECT resourcetype, resourceid, permissionlevel, specificpermissions
FROM resourceaccesses
WHERE actorid = $1
ORDER BY resourcetype, resourceid
"#,
    )
    .bind(actor_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        let resource_type = row.try_get::<i32, _>("resourcetype").map_err(storage)?;
        let permission_level = row.try_get::<i32, _>("permissionlevel").map_err(storage)?;
        Ok(IdentityResourceAccessSnapshot {
            resource_type: ResourceType::from_i32(resource_type).ok_or_else(|| {
                IdentityError::Storage(format!(
                    "unknown persisted ResourceType value {resource_type}"
                ))
            })?,
            resource_id: row.try_get("resourceid").map_err(storage)?,
            permission_level: PermissionLevel::from_i32(permission_level).ok_or_else(|| {
                IdentityError::Storage(format!(
                    "unknown persisted PermissionLevel value {permission_level}"
                ))
            })?,
            specific_permissions: row.try_get("specificpermissions").map_err(storage)?,
        })
    })
    .collect::<Result<Vec<_>, IdentityError>>()?;
    Ok(Some(UserActivitySnapshot {
        email: row
            .try_get::<Option<String>, _>("email")
            .map_err(storage)?
            .ok_or_else(|| {
                IdentityError::Storage("a persisted User has no email address".to_owned())
            })?,
        is_enabled: row.try_get("isenabled").map_err(storage)?,
        team_ids,
        role_ids,
        resource_accesses,
    }))
}

async fn record_user_update(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
    user_name: String,
    old_snapshot: UserActivitySnapshot,
    password_changed: bool,
    changed_by_actor_id: ActorId,
    changed_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), IdentityError> {
    let new_snapshot = load_user_snapshot(transaction, user_id)
        .await?
        .ok_or_else(missing_persisted_user)?;
    if old_snapshot == new_snapshot && !password_changed {
        return Ok(());
    }
    let activity = ActivityEvent::new_user_event(
        user_id,
        user_name,
        changed_by_actor_id,
        ActivityEventInfo::user_updated(old_snapshot, new_snapshot, password_changed),
        changed_at,
    )
    .map_err(invalid_activity)?;
    insert_activity(transaction, &activity).await
}

pub(crate) async fn ensure_enabled_administrator_remains(
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<(), IdentityError> {
    let remains = sqlx::query_scalar::<_, bool>(
        r#"
SELECT EXISTS (
    SELECT 1
    FROM users user_account
    JOIN actors user_actor ON user_actor.id = user_account.actorid
    WHERE user_actor.isenabled
      AND EXISTS (
          SELECT 1
          FROM actorroles assignment
          JOIN roles role ON role.id = assignment.roleid
          WHERE role.name = 'Admin' AND role.roletype = 'System'
            AND (
                assignment.actorid = user_account.actorid
                OR EXISTS (
                    SELECT 1
                    FROM actorteammemberships membership
                    JOIN teams team ON team.id = membership.teamid
                    JOIN actors team_actor ON team_actor.id = team.actorid
                    WHERE membership.memberactorid = user_account.actorid
                      AND team.actorid = assignment.actorid
                      AND team_actor.isenabled
                )
            )
      )
)
"#,
    )
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    if remains {
        Ok(())
    } else {
        Err(IdentityError::Conflict(
            "At least one enabled administrator must remain.".to_owned(),
        ))
    }
}

async fn fetch_user_view(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<UserDetails>, IdentityError> {
    sqlx::query(GET_USER_SQL)
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .map(|row| {
            let accesses = map_accesses(row.try_get("resourceaccesses").map_err(storage)?)?;
            map_user(row, Some(accesses))
        })
        .transpose()
}

fn missing_persisted_user() -> IdentityError {
    IdentityError::Storage("a User mutation did not leave a readable User projection".to_owned())
}

#[derive(Deserialize)]
struct PersistedResourceInfo {
    id: Uuid,
    name: String,
}

fn map_user(
    row: PgRow,
    resource_accesses: Option<Vec<ResourceAccessDetails>>,
) -> Result<UserDetails, IdentityError> {
    Ok(UserDetails {
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

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

fn conflict_or_storage(error: sqlx::Error) -> IdentityError {
    if error
        .as_database_error()
        .is_some_and(|database| database.code().as_deref() == Some("23505"))
    {
        IdentityError::Conflict("A User with the same value already exists.".to_owned())
    } else {
        storage(error)
    }
}
