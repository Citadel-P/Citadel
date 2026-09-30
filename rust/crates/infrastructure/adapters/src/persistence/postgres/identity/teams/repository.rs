use crate::persistence::postgres::identity::authorization_cache::{Impact, Mutation};
use crate::persistence::postgres::identity::resource_access::{
    expands_resource_access, map_accesses, specific_mask,
};
use citadel_activities::{
    ActivityEvent, ActivityEventInfo, IdentityResourceAccessSnapshot, TeamActivitySnapshot,
};
use citadel_identity::ActorType;
use citadel_identity::{
    IdentityError, NewTeamMutation, ResourceAccessDetails, ResourceAccessInput, ResourceInfo,
    StoredPage, TeamDetails, TeamMemberDetails, TeamPatchMutation, TeamReader, TeamRepository,
    TeamSearchItemDetails,
};
use citadel_primitives::{ActorId, PermissionLevel, ResourceType};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;
use crate::persistence::postgres::activities::store::invalid_activity;
use crate::persistence::postgres::identity::users::repository::ensure_enabled_administrator_remains;
use crate::persistence::postgres::identity::users::repository::lock_identity_mutations;

const TEAM_AGGREGATE_SQL: &str = r#"
SELECT team.id, team.name, team.actorid, actor.isenabled,
       members.total_members, members.users, members.members, roles.value AS roles
FROM teams team
JOIN actors actor ON actor.id = team.actorid AND actor.type = 'Team'
LEFT JOIN LATERAL (
    SELECT COUNT(*)::int AS total_members,
           COALESCE(jsonb_agg(jsonb_build_object('id', principal.resource_id, 'name', principal.name)
                    ORDER BY principal.name, principal.resource_id)
                    FILTER (WHERE principal.principal_type = 'User'), '[]'::jsonb) AS users,
           COALESCE(jsonb_agg(jsonb_build_object(
                    'actorId', principal.actor_id,
                    'resourceId', principal.resource_id,
                    'name', principal.name,
                    'principalType', principal.principal_type)
                    ORDER BY principal.name, principal.actor_id), '[]'::jsonb) AS members
    FROM (
        SELECT member.id AS actor_id, user_account.id AS resource_id,
               user_account.name, 'User'::text AS principal_type
        FROM actorteammemberships membership
        JOIN actors member ON member.id = membership.memberactorid AND member.type = 'User'
        JOIN users user_account ON user_account.actorid = member.id
        WHERE membership.teamid = team.id
        UNION ALL
        SELECT member.id, account.id, account.name, 'ServiceAccount'::text
        FROM actorteammemberships membership
        JOIN actors member ON member.id = membership.memberactorid
          AND member.type IN ('Service', 'ServiceAccount')
        JOIN serviceaccounts account ON account.actorid = member.id
        WHERE membership.teamid = team.id
    ) principal
) members ON TRUE
LEFT JOIN LATERAL (
    SELECT COALESCE(jsonb_agg(jsonb_build_object('id', role.id, 'name', role.name)
                    ORDER BY role.name, role.id), '[]'::jsonb) AS value
    FROM actorroles assignment
    JOIN roles role ON role.id = assignment.roleid
    WHERE assignment.actorid = team.actorid
) roles ON TRUE
"#;

const RESOURCE_LOOKUP_CTE: &str = r#"
WITH resource_lookup(resourceid, resourcetype, resourcename) AS (
    SELECT id, 0, name FROM platforms
    UNION ALL SELECT id, 1, name FROM deployments
    UNION ALL SELECT id, 2, name FROM stacks
    UNION ALL SELECT id, 3, name FROM registries
    UNION ALL SELECT id, 4, name FROM gitrepositories
    UNION ALL SELECT id, 6, name FROM alertrules
    UNION ALL SELECT id, 7, name FROM alertchannels
    UNION ALL SELECT id, 12, name FROM tags
    UNION ALL SELECT id, 13, name FROM actions
    UNION ALL SELECT id, 15, name FROM backuprepositories
    UNION ALL SELECT id, 18, name FROM buildprojects
    UNION ALL SELECT id, 19, name FROM buildagentpools
    UNION ALL SELECT id, 20, name FROM swarmservices
)
"#;

#[derive(Clone)]
pub struct PostgresTeamRepository {
    pool: PgPool,
}

impl PostgresTeamRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl TeamReader for PostgresTeamRepository {
    fn list<'a>(
        &'a self,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<TeamDetails>, IdentityError>> {
        Box::pin(async move {
            let total_items = sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM teams WHERE ($1::text IS NULL OR name ILIKE '%' || $1 || '%')",
            )
            .bind(name)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?;
            let query = format!(
                "{TEAM_AGGREGATE_SQL} WHERE ($1::text IS NULL OR team.name ILIKE '%' || $1 || '%') ORDER BY team.name, team.id LIMIT $2 OFFSET $3"
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
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
                    .map(|row| map_team(row, None))
                    .collect::<Result<_, _>>()?,
            })
        })
    }

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<TeamDetails>, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let result = fetch_team_view(&mut transaction, id).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(result)
        })
    }

    fn search<'a>(
        &'a self,
        query: &'a str,
        limit: i64,
    ) -> BoxFuture<'a, Result<Vec<TeamSearchItemDetails>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                "SELECT id, name FROM teams WHERE name ILIKE '%' || $1 || '%' ORDER BY name, id LIMIT $2",
            )
            .bind(query)
            .bind(limit)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| {
                Ok(TeamSearchItemDetails {
                    id: row.try_get("id").map_err(storage)?,
                    name: row.try_get("name").map_err(storage)?,
                })
            })
            .collect()
        })
    }
}

impl TeamRepository for PostgresTeamRepository {
    fn create<'a>(
        &'a self,
        team: &'a NewTeamMutation,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![team.id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            ensure_team_name_available(&mut transaction, &team.name, None).await?;
            let member_actor_ids = user_actor_ids(&mut transaction, &team.user_ids).await?;
            validate_roles(
                &mut transaction,
                &team.role_ids,
                &[],
                custom_access_control_enabled,
            )
            .await?;
            if !team.resource_accesses.is_empty() && !custom_access_control_enabled {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'Team')")
                .bind(team.actor_id.value())
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query("INSERT INTO teams (id, actorid, name) VALUES ($1, $2, $3)")
                .bind(team.id)
                .bind(team.actor_id.value())
                .bind(&team.name)
                .execute(&mut *transaction)
                .await
                .map_err(conflict_or_storage)?;
            replace_user_members(&mut transaction, team.id, &team.user_ids).await?;
            replace_roles(&mut transaction, team.actor_id, &team.role_ids).await?;
            replace_resource_accesses(&mut transaction, team.actor_id, &team.resource_accesses)
                .await?;
            let snapshot = TeamActivitySnapshot {
                is_enabled: true,
                member_actor_ids,
                role_ids: team.role_ids.clone(),
                resource_accesses: snapshot_accesses(&team.resource_accesses),
            };
            insert_team_activity(
                &mut transaction,
                team.id,
                &team.name,
                team.created_by_actor_id,
                ActivityEventInfo::team_created(snapshot),
                team.created_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, team.id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
        patch: &'a TeamPatchMutation,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old_snapshot = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            let current_user_ids = team_user_ids(&mut transaction, id).await?;
            let current_role_ids = old_snapshot.role_ids.clone();
            let proposed_user_ids = patch.user_ids.as_deref().unwrap_or(&current_user_ids);
            let proposed_role_ids = patch.role_ids.as_deref().unwrap_or(&current_role_ids);
            if patch.user_ids.is_some() {
                user_actor_ids(&mut transaction, proposed_user_ids).await?;
            }
            if patch.role_ids.is_some() {
                validate_roles(
                    &mut transaction,
                    proposed_role_ids,
                    &current_role_ids,
                    custom_access_control_enabled,
                )
                .await?;
            }
            let resulting_has_custom_access =
                roles_include_custom(&mut transaction, proposed_role_ids).await?
                    || patch
                        .resource_accesses
                        .as_ref()
                        .map_or(!old_snapshot.resource_accesses.is_empty(), |items| {
                            !items.is_empty()
                        });
            let adds_user = proposed_user_ids
                .iter()
                .any(|user| !current_user_ids.contains(user));
            if !custom_access_control_enabled && adds_user && resulting_has_custom_access {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            if let Some(accesses) = &patch.resource_accesses
                && !custom_access_control_enabled
                && expands_resource_access(&old_snapshot.resource_accesses, accesses)
            {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            if let Some(user_ids) = &patch.user_ids {
                replace_user_members(&mut transaction, id, user_ids).await?;
            }
            if let Some(role_ids) = &patch.role_ids {
                replace_roles(&mut transaction, state.actor_id, role_ids).await?;
            }
            if let Some(accesses) = &patch.resource_accesses {
                replace_resource_accesses(&mut transaction, state.actor_id, accesses).await?;
            }
            if let Some(is_enabled) = patch.is_enabled {
                sqlx::query("UPDATE actors SET isenabled = $2 WHERE id = $1")
                    .bind(state.actor_id.value())
                    .bind(is_enabled)
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?;
            }
            if patch.user_ids.is_some()
                || patch.role_ids.is_some()
                || patch.is_enabled == Some(false)
            {
                ensure_enabled_administrator_remains(&mut transaction).await?;
            }
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old_snapshot,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            ensure_team_name_available(&mut transaction, name, Some(id)).await?;
            if state.name != name {
                sqlx::query("UPDATE teams SET name = $2 WHERE id = $1")
                    .bind(id)
                    .bind(name)
                    .execute(&mut *transaction)
                    .await
                    .map_err(conflict_or_storage)?;
                insert_team_activity(
                    &mut transaction,
                    id,
                    name,
                    changed_by_actor_id,
                    ActivityEventInfo::team_renamed(state.name, name.to_owned()),
                    changed_at,
                )
                .await?;
            }
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            validate_roles(
                &mut transaction,
                &[role_id],
                &[],
                custom_access_control_enabled,
            )
            .await?;
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
                    "The Role is already assigned to the Team.".to_owned(),
                ));
            }
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn add_member(
        &self,
        id: Uuid,
        member_actor_id: ActorId,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
        custom_access_control_enabled: bool,
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let member_type = member_type(&mut transaction, member_actor_id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            if member_type == ActorType::ServiceAccount
                && service_account_archived(&mut transaction, member_actor_id).await?
            {
                return Err(IdentityError::NotFound);
            }
            if !custom_access_control_enabled
                && (member_type == ActorType::ServiceAccount
                    || team_has_custom_access(&mut transaction, state.actor_id).await?)
            {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            let old = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            let inserted = sqlx::query(
                "INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(id).bind(member_actor_id.value())
            .execute(&mut *transaction).await.map_err(storage)?.rows_affected();
            if inserted == 0 {
                return Err(IdentityError::Conflict(
                    "The Actor is already a member of the Team.".to_owned(),
                ));
            }
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            authorization
                .commit(transaction, impact)
                .await
                .map_err(storage)?;
            Ok(view)
        })
    }

    fn remove_member(
        &self,
        id: Uuid,
        member_actor_id: ActorId,
        changed_by_actor_id: ActorId,
        changed_at: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'_, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            member_type(&mut transaction, member_actor_id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            let deleted = sqlx::query(
                "DELETE FROM actorteammemberships WHERE teamid = $1 AND memberactorid = $2",
            )
            .bind(id)
            .bind(member_actor_id.value())
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if deleted == 0 {
                return Err(IdentityError::NotFound);
            }
            ensure_enabled_administrator_remains(&mut transaction).await?;
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            if !custom_access_control_enabled {
                return Err(IdentityError::LicenseRequired("custom-access-control"));
            }
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            let inserted = insert_access(&mut transaction, state.actor_id, access, true).await?;
            if !inserted {
                return Err(IdentityError::Conflict(
                    "The resource access is already assigned to the Team.".to_owned(),
                ));
            }
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
    ) -> BoxFuture<'a, Result<TeamDetails, IdentityError>> {
        Box::pin(async move {
            let mut authorization = Mutation::enter(&self.pool).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let impact = Impact::Teams(vec![id]);
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let state = load_team_state(&mut transaction, id)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let old = load_team_snapshot(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
            let deleted = sqlx::query(
                "DELETE FROM resourceaccesses WHERE actorid = $1 AND resourcetype = $2 AND resourceid = $3 AND permissionlevel = $4 AND specificpermissions = $5",
            )
            .bind(state.actor_id.value()).bind(access.resource_type as i32).bind(access.resource_id)
            .bind(access.permission_level as i32).bind(specific_mask(&access.specific_permissions))
            .execute(&mut *transaction).await.map_err(storage)?.rows_affected();
            if deleted == 0 {
                return Err(IdentityError::NotFound);
            }
            record_team_update(
                &mut transaction,
                id,
                &state.name,
                old,
                changed_by_actor_id,
                changed_at,
            )
            .await?;
            let view = fetch_team_view(&mut transaction, id)
                .await?
                .ok_or_else(missing_persisted_team)?;
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
            let impact = Impact::Teams(ids.to_vec());
            authorization
                .capture(&mut transaction, &impact)
                .await
                .map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            let rows =
                sqlx::query("SELECT id, name FROM teams WHERE id = ANY($1) ORDER BY name, id")
                    .bind(ids)
                    .fetch_all(&mut *transaction)
                    .await
                    .map_err(storage)?;
            if rows.is_empty() {
                return Err(IdentityError::NotFound);
            }
            let mut deleted = Vec::with_capacity(rows.len());
            for row in rows {
                let id = row.try_get("id").map_err(storage)?;
                let name: String = row.try_get("name").map_err(storage)?;
                let snapshot = load_team_snapshot(&mut transaction, id)
                    .await?
                    .ok_or_else(missing_persisted_team)?;
                deleted.push((id, name, snapshot));
            }
            sqlx::query("DELETE FROM teams WHERE id = ANY($1)")
                .bind(ids)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            ensure_enabled_administrator_remains(&mut transaction).await?;
            for (id, name, snapshot) in deleted {
                insert_team_activity(
                    &mut transaction,
                    id,
                    &name,
                    changed_by_actor_id,
                    ActivityEventInfo::team_deleted(snapshot),
                    changed_at,
                )
                .await?;
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
struct TeamState {
    name: String,
    actor_id: ActorId,
}

async fn load_team_state(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<TeamState>, IdentityError> {
    sqlx::query("SELECT name, actorid FROM teams WHERE id = $1")
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .map(|row| {
            Ok(TeamState {
                name: row.try_get("name").map_err(storage)?,
                actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
            })
        })
        .transpose()
}

async fn ensure_team_name_available(
    transaction: &mut Transaction<'_, Postgres>,
    name: &str,
    exclude_id: Option<Uuid>,
) -> Result<(), IdentityError> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM teams WHERE name = $1 AND ($2::uuid IS NULL OR id <> $2))",
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

async fn user_actor_ids(
    transaction: &mut Transaction<'_, Postgres>,
    user_ids: &[Uuid],
) -> Result<Vec<Uuid>, IdentityError> {
    if user_ids.is_empty() {
        return Ok(Vec::new());
    }
    let actor_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT actorid FROM users WHERE id = ANY($1) ORDER BY actorid",
    )
    .bind(user_ids)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    if actor_ids.len() != user_ids.len() {
        return Err(IdentityError::NotFound);
    }
    Ok(actor_ids)
}

async fn team_user_ids(
    transaction: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
) -> Result<Vec<Uuid>, IdentityError> {
    sqlx::query_scalar("SELECT users.id FROM actorteammemberships membership JOIN users ON users.actorid = membership.memberactorid WHERE membership.teamid = $1 ORDER BY users.id")
        .bind(team_id).fetch_all(&mut **transaction).await.map_err(storage)
}

async fn validate_roles(
    transaction: &mut Transaction<'_, Postgres>,
    role_ids: &[Uuid],
    current: &[Uuid],
    licensed: bool,
) -> Result<(), IdentityError> {
    if role_ids.is_empty() {
        return Ok(());
    }
    let rows = sqlx::query("SELECT id, roletype FROM roles WHERE id = ANY($1)")
        .bind(role_ids)
        .fetch_all(&mut **transaction)
        .await
        .map_err(storage)?;
    if rows.len() != role_ids.len() {
        return Err(IdentityError::NotFound);
    }
    if !licensed
        && rows.iter().any(|row| {
            let id = row.try_get::<Uuid, _>("id").expect("selected Role ID");
            !current.contains(&id)
                && row
                    .try_get::<String, _>("roletype")
                    .expect("selected Role type")
                    == "Custom"
        })
    {
        return Err(IdentityError::LicenseRequired("custom-access-control"));
    }
    Ok(())
}

async fn roles_include_custom(
    transaction: &mut Transaction<'_, Postgres>,
    role_ids: &[Uuid],
) -> Result<bool, IdentityError> {
    if role_ids.is_empty() {
        return Ok(false);
    }
    sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM roles WHERE id = ANY($1) AND roletype = 'Custom')",
    )
    .bind(role_ids)
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)
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

async fn replace_user_members(
    transaction: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
    user_ids: &[Uuid],
) -> Result<(), IdentityError> {
    sqlx::query("DELETE FROM actorteammemberships membership USING users WHERE membership.teamid = $1 AND users.actorid = membership.memberactorid")
        .bind(team_id).execute(&mut **transaction).await.map_err(storage)?;
    if !user_ids.is_empty() {
        sqlx::query("INSERT INTO actorteammemberships (teamid, memberactorid) SELECT $1, actorid FROM users WHERE id = ANY($2)")
            .bind(team_id).bind(user_ids).execute(&mut **transaction).await.map_err(storage)?;
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
        insert_access(transaction, actor_id, access, false).await?;
    }
    Ok(())
}

async fn insert_access(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
    access: &ResourceAccessInput,
    ignore_conflict: bool,
) -> Result<bool, IdentityError> {
    let query = if ignore_conflict {
        "INSERT INTO resourceaccesses (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) VALUES ($1, $2, $3, $4, $5, $6) ON CONFLICT (resourcetype, resourceid, actorid) DO NOTHING"
    } else {
        "INSERT INTO resourceaccesses (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions) VALUES ($1, $2, $3, $4, $5, $6)"
    };
    Ok(sqlx::query(query)
        .bind(Uuid::now_v7())
        .bind(actor_id.value())
        .bind(access.permission_level as i32)
        .bind(access.resource_id)
        .bind(access.resource_type as i32)
        .bind(specific_mask(&access.specific_permissions))
        .execute(&mut **transaction)
        .await
        .map_err(storage)?
        .rows_affected()
        == 1)
}

async fn member_type(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
) -> Result<Option<ActorType>, IdentityError> {
    let value = sqlx::query_scalar::<_, String>(r#"SELECT actor.type FROM actors actor
        WHERE actor.id = $1 AND ((actor.type = 'User' AND EXISTS (SELECT 1 FROM users WHERE actorid = actor.id))
        OR (actor.type IN ('Service', 'ServiceAccount') AND EXISTS (SELECT 1 FROM serviceaccounts WHERE actorid = actor.id)))"#)
        .bind(actor_id.value()).fetch_optional(&mut **transaction).await.map_err(storage)?;
    value
        .map(|value| match value.as_str() {
            "User" => Ok(ActorType::User),
            "Service" | "ServiceAccount" => Ok(ActorType::ServiceAccount),
            _ => Err(IdentityError::Storage(format!(
                "unknown member Actor type '{value}'"
            ))),
        })
        .transpose()
}

async fn service_account_archived(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
) -> Result<bool, IdentityError> {
    sqlx::query_scalar("SELECT archivedatutc IS NOT NULL FROM serviceaccounts WHERE actorid = $1")
        .bind(actor_id.value())
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)
}

async fn team_has_custom_access(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
) -> Result<bool, IdentityError> {
    sqlx::query_scalar(r#"SELECT EXISTS (SELECT 1 FROM actorroles assignment JOIN roles role ON role.id = assignment.roleid WHERE assignment.actorid = $1 AND role.roletype = 'Custom')
        OR EXISTS (SELECT 1 FROM resourceaccesses WHERE actorid = $1)"#)
        .bind(actor_id.value()).fetch_one(&mut **transaction).await.map_err(storage)
}

async fn load_team_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
) -> Result<Option<TeamActivitySnapshot>, IdentityError> {
    let Some(row) = sqlx::query("SELECT team.actorid, actor.isenabled FROM teams team JOIN actors actor ON actor.id = team.actorid WHERE team.id = $1")
        .bind(team_id).fetch_optional(&mut **transaction).await.map_err(storage)? else { return Ok(None); };
    let actor_id = row.try_get::<Uuid, _>("actorid").map_err(storage)?;
    let member_actor_ids = sqlx::query_scalar(
        "SELECT memberactorid FROM actorteammemberships WHERE teamid = $1 ORDER BY memberactorid",
    )
    .bind(team_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    let role_ids =
        sqlx::query_scalar("SELECT roleid FROM actorroles WHERE actorid = $1 ORDER BY roleid")
            .bind(actor_id)
            .fetch_all(&mut **transaction)
            .await
            .map_err(storage)?;
    let resource_accesses = load_snapshot_accesses(transaction, ActorId::new(actor_id)).await?;
    Ok(Some(TeamActivitySnapshot {
        is_enabled: row.try_get("isenabled").map_err(storage)?,
        member_actor_ids,
        role_ids,
        resource_accesses,
    }))
}

async fn load_snapshot_accesses(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
) -> Result<Vec<IdentityResourceAccessSnapshot>, IdentityError> {
    sqlx::query("SELECT resourcetype, resourceid, permissionlevel, specificpermissions FROM resourceaccesses WHERE actorid = $1 ORDER BY resourcetype, resourceid")
        .bind(actor_id.value()).fetch_all(&mut **transaction).await.map_err(storage)?.into_iter()
        .map(|row| {
            let resource = row.try_get::<i32, _>("resourcetype").map_err(storage)?;
            let level = row.try_get::<i32, _>("permissionlevel").map_err(storage)?;
            Ok(IdentityResourceAccessSnapshot {
                resource_type: ResourceType::from_i32(resource).ok_or_else(|| IdentityError::Storage(format!("unknown persisted ResourceType value {resource}")))?,
                resource_id: row.try_get("resourceid").map_err(storage)?,
                permission_level: PermissionLevel::from_i32(level).ok_or_else(|| IdentityError::Storage(format!("unknown persisted PermissionLevel value {level}")))?,
                specific_permissions: row.try_get("specificpermissions").map_err(storage)?,
            })
        }).collect()
}

fn snapshot_accesses(accesses: &[ResourceAccessInput]) -> Vec<IdentityResourceAccessSnapshot> {
    let mut values = accesses
        .iter()
        .map(|access| IdentityResourceAccessSnapshot {
            resource_type: access.resource_type,
            resource_id: access.resource_id,
            permission_level: access.permission_level,
            specific_permissions: specific_mask(&access.specific_permissions),
        })
        .collect::<Vec<_>>();
    values.sort_by_key(|access| (access.resource_type, access.resource_id));
    values
}

async fn record_team_update(
    transaction: &mut Transaction<'_, Postgres>,
    team_id: Uuid,
    name: &str,
    old: TeamActivitySnapshot,
    changed_by: ActorId,
    changed_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), IdentityError> {
    let new = load_team_snapshot(transaction, team_id)
        .await?
        .ok_or_else(missing_persisted_team)?;
    if old == new {
        return Ok(());
    }
    insert_team_activity(
        transaction,
        team_id,
        name,
        changed_by,
        ActivityEventInfo::team_updated(old, new),
        changed_at,
    )
    .await
}

async fn insert_team_activity(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    actor_id: ActorId,
    info: ActivityEventInfo,
    created_at: chrono::DateTime<chrono::Utc>,
) -> Result<(), IdentityError> {
    let event = ActivityEvent::new_team_event(id, name.to_owned(), actor_id, info, created_at)
        .map_err(invalid_activity)?;
    insert_activity(transaction, &event).await
}

async fn fetch_team_view(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
) -> Result<Option<TeamDetails>, IdentityError> {
    let query = format!(
        "{RESOURCE_LOOKUP_CTE} SELECT aggregate.*, accesses.value AS resource_accesses FROM ({TEAM_AGGREGATE_SQL} WHERE team.id = $1) aggregate LEFT JOIN LATERAL (SELECT COALESCE(jsonb_agg(jsonb_build_object('id', access.id, 'resourceType', access.resourcetype, 'resourceId', access.resourceid, 'resourceName', lookup.resourcename, 'permissionLevel', access.permissionlevel, 'specificPermissions', access.specificpermissions) ORDER BY access.resourcetype, access.resourceid), '[]'::jsonb) AS value FROM resourceaccesses access LEFT JOIN resource_lookup lookup ON lookup.resourceid = access.resourceid AND lookup.resourcetype = access.resourcetype WHERE access.actorid = aggregate.actorid) accesses ON TRUE"
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .map(|row| {
            let accesses = map_accesses(row.try_get("resource_accesses").map_err(storage)?)?;
            map_team(row, Some(accesses))
        })
        .transpose()
}

#[derive(Deserialize)]
struct PersistedResourceInfo {
    id: Uuid,
    name: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedMember {
    actor_id: Uuid,
    resource_id: Uuid,
    name: String,
    principal_type: ActorType,
}

fn map_team(
    row: PgRow,
    accesses: Option<Vec<ResourceAccessDetails>>,
) -> Result<TeamDetails, IdentityError> {
    Ok(TeamDetails {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
        is_enabled: row.try_get("isenabled").map_err(storage)?,
        total_members: row.try_get("total_members").map_err(storage)?,
        users: Some(map_resource_info(row.try_get("users").map_err(storage)?)?),
        roles: Some(map_resource_info(row.try_get("roles").map_err(storage)?)?),
        resource_accesses: accesses,
        members: Some(map_members(row.try_get("members").map_err(storage)?)?),
    })
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

fn map_members(value: serde_json::Value) -> Result<Vec<TeamMemberDetails>, IdentityError> {
    serde_json::from_value::<Vec<PersistedMember>>(value)
        .map_err(|error| IdentityError::Storage(error.to_string()))
        .map(|values| {
            values
                .into_iter()
                .map(|value| TeamMemberDetails {
                    actor_id: ActorId::new(value.actor_id),
                    resource_id: value.resource_id,
                    name: value.name,
                    principal_type: value.principal_type,
                })
                .collect()
        })
}

fn missing_persisted_team() -> IdentityError {
    IdentityError::Storage("a Team mutation did not leave a readable Team projection".to_owned())
}
fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
fn conflict_or_storage(error: sqlx::Error) -> IdentityError {
    if error
        .as_database_error()
        .is_some_and(|database| database.code().as_deref() == Some("23505"))
    {
        IdentityError::Conflict("A Team with the same value already exists.".to_owned())
    } else {
        storage(error)
    }
}
