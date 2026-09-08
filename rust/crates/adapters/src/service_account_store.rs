use chrono::{DateTime, Utc};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActorId, PermissionLevel, ResourceType,
    ServiceAccountActivitySnapshot, ServiceAccountResourceAccessSnapshot,
};
use citadel_identity::{
    IdentityError, NewServiceAccount, NewServiceAccountToken, PatchField, ResourceInfo,
    ServiceAccountResourceAccess, ServiceAccountStore, ServiceAccountTokenView, ServiceAccountView,
    StoredPage,
};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use sqlx::postgres::PgRow;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::{insert_activity, invalid_activity};

#[derive(Clone)]
pub struct PostgresServiceAccountStore {
    pool: PgPool,
}

impl PostgresServiceAccountStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl ServiceAccountStore for PostgresServiceAccountStore {
    fn usages(
        &self,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<Vec<citadel_identity::RunAsActorUsageView>, IdentityError>> {
        Box::pin(async move {
            let rows: Vec<(Uuid, String, i32, bool)> = sqlx::query_as("SELECT id,name,13 AS resourcetype,enabled AS isactive FROM actions WHERE runasactorid=$1 UNION ALL SELECT id,name,16 AS resourcetype,(enabled AND archivedat IS NULL) AS isactive FROM backuppolicies WHERE runasactorid=$1 ORDER BY resourcetype,name,id")
                .bind(actor_id.value()).fetch_all(&self.pool).await.map_err(storage)?;
            Ok(rows
                .into_iter()
                .map(
                    |(id, name, kind, is_active)| citadel_identity::RunAsActorUsageView {
                        id,
                        name,
                        resource_type: if kind == 13 {
                            ResourceType::AutomationAction
                        } else {
                            ResourceType::BackupPolicy
                        },
                        is_active,
                    },
                )
                .collect())
        })
    }
    fn list<'a>(
        &'a self,
        actor_id: ActorId,
        is_administrator: bool,
        include_archived: bool,
        name: Option<&'a str>,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'a, Result<StoredPage<ServiceAccountView>, IdentityError>> {
        Box::pin(async move {
            let rows = sqlx::query(
                r#"
WITH actor_scope AS (
    SELECT $1::uuid AS actorid
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_read AS (
    SELECT EXISTS (
        SELECT 1
        FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = 21 AND permission.permissionlevel >= 1
    ) AS allowed
)
SELECT account.id, account.name, account.description, account.actorid, actor.isenabled,
       COUNT(*) OVER() AS totalitems,
       account.createdat, account.createdbyactorid, account.updatedat, account.archivedatutc,
       (SELECT COUNT(*) FROM serviceaccounttokens token
        WHERE token.serviceaccountid = account.id AND token.revokedatutc IS NULL
          AND (token.expiresatutc IS NULL OR token.expiresatutc > CURRENT_TIMESTAMP)) AS activetokencount,
       (SELECT MAX(token.lastusedatutc) FROM serviceaccounttokens token
        WHERE token.serviceaccountid = account.id) AS lastusedatutc,
       COALESCE((SELECT jsonb_agg(jsonb_build_object('id', role.id, 'name', role.name) ORDER BY role.name, role.id)
                 FROM actorroles assignment JOIN roles role ON role.id = assignment.roleid
                 WHERE assignment.actorid = account.actorid), '[]'::jsonb) AS roles,
       COALESCE((SELECT jsonb_agg(jsonb_build_object('id', team.id, 'name', team.name) ORDER BY team.name, team.id)
                 FROM actorteammemberships membership JOIN teams team ON team.id = membership.teamid
                 WHERE membership.memberactorid = account.actorid), '[]'::jsonb) AS teams,
       COALESCE((SELECT jsonb_agg(jsonb_build_object(
                    'id', access.id,
                    'resourceType', access.resourcetype,
                    'resourceId', access.resourceid,
                    'permissionLevel', access.permissionlevel,
                    'specificPermissions', access.specificpermissions) ORDER BY access.resourcetype, access.resourceid)
                 FROM resourceaccesses access WHERE access.actorid = account.actorid), '[]'::jsonb) AS resourceaccesses
FROM serviceaccounts account
JOIN actors actor ON actor.id = account.actorid AND actor.type = 'ServiceAccount'
CROSS JOIN global_read
WHERE ($2 OR account.archivedatutc IS NULL)
  AND ($3::text IS NULL OR account.name ILIKE '%' || $3 || '%')
  AND ($4 OR global_read.allowed OR EXISTS (
      SELECT 1 FROM actor_scope scope
      JOIN resourceaccesses access ON access.actorid = scope.actorid
      WHERE access.resourcetype = 21 AND access.resourceid = account.id
        AND access.permissionlevel >= 1))
ORDER BY account.name, account.id
LIMIT $5 OFFSET $6
"#,
            )
            .bind(actor_id.value())
            .bind(include_archived)
            .bind(name)
            .bind(is_administrator)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            let total_items = rows
                .first()
                .map(|row| row.try_get("totalitems").map_err(storage))
                .transpose()?
                .unwrap_or_default();
            Ok(StoredPage {
                total_items,
                items: rows
                    .into_iter()
                    .map(map_account)
                    .collect::<Result<_, _>>()?,
            })
        })
    }

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<ServiceAccountView>, IdentityError>> {
        Box::pin(async move { load_account(&self.pool, id).await })
    }

    fn create<'a>(
        &'a self,
        account: &'a NewServiceAccount,
    ) -> BoxFuture<'a, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let roles = sqlx::query_scalar::<_, Uuid>("SELECT id FROM roles WHERE id = ANY($1)")
                .bind(&account.role_ids)
                .fetch_all(&mut *transaction)
                .await
                .map_err(storage)?;
            if roles.len() != account.role_ids.len() {
                return Err(IdentityError::NotFound);
            }
            let teams = sqlx::query_scalar::<_, Uuid>(
                r#"
SELECT team.id
FROM teams team
JOIN actors actor ON actor.id = team.actorid AND actor.isenabled
WHERE team.id = ANY($1)
"#,
            )
            .bind(&account.team_ids)
            .fetch_all(&mut *transaction)
            .await
            .map_err(storage)?;
            if teams.len() != account.team_ids.len() {
                return Err(IdentityError::NotFound);
            }

            sqlx::query(
                "INSERT INTO actors (id, isenabled, type) VALUES ($1, $2, 'ServiceAccount')",
            )
            .bind(account.actor_id.value())
            .bind(account.is_enabled)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            sqlx::query(
                r#"
INSERT INTO serviceaccounts
    (id, actorid, name, description, createdbyactorid, createdat, updatedat, archivedatutc)
VALUES ($1, $2, $3, $4, $5, $6, $6, NULL)
"#,
            )
            .bind(account.id)
            .bind(account.actor_id.value())
            .bind(&account.name)
            .bind(&account.description)
            .bind(account.created_by_actor_id.value())
            .bind(account.created_at)
            .execute(&mut *transaction)
            .await
            .map_err(conflict_or_storage)?;
            for role_id in &account.role_ids {
                sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
                    .bind(account.actor_id.value())
                    .bind(role_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?;
            }
            for team_id in &account.team_ids {
                sqlx::query(
                    "INSERT INTO actorteammemberships (teamid, memberactorid) VALUES ($1, $2)",
                )
                .bind(team_id)
                .bind(account.actor_id.value())
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
            for access in &account.resource_accesses {
                sqlx::query(
                    r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, $3, $4, $5, $6)
"#,
                )
                .bind(access.id.unwrap_or_else(Uuid::now_v7))
                .bind(account.actor_id.value())
                .bind(access.permission_level as i32)
                .bind(access.resource_id)
                .bind(access.resource_type as i32)
                .bind(specific_mask(&access.specific_permissions))
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
            insert_service_account_activity(
                &mut transaction,
                account.id,
                &account.name,
                account.created_by_actor_id,
                ActivityEventInfo::service_account_created(ServiceAccountActivitySnapshot {
                    id: account.id,
                    name: account.name.clone(),
                    description: account.description.clone(),
                    is_enabled: account.is_enabled,
                    team_ids: sorted_ids(&account.team_ids),
                    role_ids: sorted_ids(&account.role_ids),
                    resource_accesses: snapshot_accesses(&account.resource_accesses),
                }),
                account.created_at,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, account.id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn update<'a>(
        &'a self,
        id: Uuid,
        description: &'a PatchField<String>,
        is_enabled: Option<bool>,
        changed_by_actor_id: ActorId,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_id = sqlx::query_scalar::<_, Uuid>(
                "SELECT actorid FROM serviceaccounts WHERE id = $1 AND archivedatutc IS NULL FOR UPDATE",
            )
            .bind(id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?
            .ok_or(IdentityError::NotFound)?;
            let (_, old_snapshot) = load_activity_snapshot(&mut transaction, id).await?;
            if let Some(is_enabled) = is_enabled {
                sqlx::query("UPDATE actors SET isenabled = $2 WHERE id = $1")
                    .bind(actor_id)
                    .bind(is_enabled)
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?;
            }
            let (description_present, description) = match description {
                PatchField::Missing => (false, None),
                PatchField::Null => (true, None),
                PatchField::Value(value) => (true, Some(value.as_str())),
            };
            sqlx::query(
                r#"
UPDATE serviceaccounts
SET description = CASE WHEN $2 THEN $3 ELSE description END,
    updatedat = $4
WHERE id = $1
"#,
            )
            .bind(id)
            .bind(description_present)
            .bind(description)
            .bind(updated_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            let (resource_name, new_snapshot) =
                load_activity_snapshot(&mut transaction, id).await?;
            if old_snapshot != new_snapshot {
                let info = if old_snapshot.name == new_snapshot.name
                    && old_snapshot.description == new_snapshot.description
                    && old_snapshot.team_ids == new_snapshot.team_ids
                    && old_snapshot.role_ids == new_snapshot.role_ids
                    && old_snapshot.resource_accesses == new_snapshot.resource_accesses
                    && old_snapshot.is_enabled != new_snapshot.is_enabled
                {
                    if new_snapshot.is_enabled {
                        ActivityEventInfo::service_account_enabled(id)
                    } else {
                        ActivityEventInfo::service_account_disabled(id)
                    }
                } else {
                    ActivityEventInfo::service_account_updated(old_snapshot, new_snapshot)
                };
                insert_service_account_activity(
                    &mut transaction,
                    id,
                    &resource_name,
                    changed_by_actor_id,
                    info,
                    updated_at,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        changed_by_actor_id: ActorId,
        updated_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let (old_name, old_snapshot) = load_activity_snapshot(&mut transaction, id).await?;
            let affected = sqlx::query(
                r#"
UPDATE serviceaccounts SET name = $2, updatedat = $3
WHERE id = $1 AND archivedatutc IS NULL
"#,
            )
            .bind(id)
            .bind(name)
            .bind(updated_at)
            .execute(&mut *transaction)
            .await
            .map_err(conflict_or_storage)?
            .rows_affected();
            if affected != 1 {
                return Err(IdentityError::NotFound);
            }
            if old_name != name {
                insert_service_account_activity(
                    &mut transaction,
                    id,
                    name,
                    changed_by_actor_id,
                    ActivityEventInfo::service_account_renamed(old_snapshot.name, name.to_owned()),
                    updated_at,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn archive<'a>(
        &'a self,
        ids: &'a [Uuid],
        actor_id: ActorId,
        archived_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_ids = sqlx::query_scalar::<_, Uuid>(
                r#"
SELECT actorid FROM serviceaccounts
WHERE id = ANY($1)
ORDER BY actorid
FOR UPDATE
"#,
            )
            .bind(ids)
            .fetch_all(&mut *transaction)
            .await
            .map_err(storage)?;
            if actor_ids.len() != ids.len() {
                return Err(IdentityError::NotFound);
            }
            let blockers = sqlx::query_scalar::<_, i64>(
                r#"
SELECT
    (SELECT COUNT(*) FROM actions
     WHERE runasactorid = ANY($1) AND enabled AND (scheduleenabled OR webhook IS NOT NULL))
  + (SELECT COUNT(*) FROM backuppolicies
     WHERE runasactorid = ANY($1) AND archivedat IS NULL AND enabled
       AND (cron IS NOT NULL OR webhook IS NOT NULL))
"#,
            )
            .bind(&actor_ids)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if blockers > 0 {
                return Err(IdentityError::Conflict(format!(
                    "The selected Service Accounts are used by {blockers} active resource(s). Replace those run-as bindings before archiving."
                )));
            }
            let mut archived_accounts = Vec::with_capacity(ids.len());
            for id in ids {
                let (name, _) = load_activity_snapshot(&mut transaction, *id).await?;
                archived_accounts.push((*id, name));
            }
            sqlx::query("UPDATE actors SET isenabled = FALSE WHERE id = ANY($1)")
                .bind(&actor_ids)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query(
                "UPDATE serviceaccounts SET archivedatutc = COALESCE(archivedatutc, $2), updatedat = $2 WHERE id = ANY($1)",
            )
            .bind(ids)
            .bind(archived_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            sqlx::query(
                r#"
UPDATE serviceaccounttokens
SET revokedatutc = COALESCE(revokedatutc, $3), revokedbyactorid = COALESCE(revokedbyactorid, $2)
WHERE serviceaccountid = ANY($1)
"#,
            )
            .bind(ids)
            .bind(actor_id.value())
            .bind(archived_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            for (id, name) in archived_accounts {
                insert_service_account_activity(
                    &mut transaction,
                    id,
                    &name,
                    actor_id,
                    ActivityEventInfo::service_account_archived(id),
                    archived_at,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn add_role(
        &self,
        account_id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_id = lock_active_account(&mut transaction, account_id).await?;
            let (_, old_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            if !sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1)")
                .bind(role_id)
                .fetch_one(&mut *transaction)
                .await
                .map_err(storage)?
            {
                return Err(IdentityError::NotFound);
            }
            let affected = sqlx::query(
                "INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2) ON CONFLICT DO NOTHING",
            )
            .bind(actor_id)
            .bind(role_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if affected != 1 {
                return Err(IdentityError::Conflict(
                    "The Role is already assigned to the Service Account.".to_owned(),
                ));
            }
            let (name, new_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            insert_service_account_activity(
                &mut transaction,
                account_id,
                &name,
                changed_by_actor_id,
                ActivityEventInfo::service_account_updated(old_snapshot, new_snapshot),
                changed_at,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, account_id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn remove_role(
        &self,
        account_id: Uuid,
        role_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_id = lock_active_account(&mut transaction, account_id).await?;
            let (_, old_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            let affected = sqlx::query("DELETE FROM actorroles WHERE actorid = $1 AND roleid = $2")
                .bind(actor_id)
                .bind(role_id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?
                .rows_affected();
            if affected != 1 {
                return Err(IdentityError::NotFound);
            }
            let (name, new_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            insert_service_account_activity(
                &mut transaction,
                account_id,
                &name,
                changed_by_actor_id,
                ActivityEventInfo::service_account_updated(old_snapshot, new_snapshot),
                changed_at,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, account_id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn add_resource_access<'a>(
        &'a self,
        account_id: Uuid,
        access: &'a ServiceAccountResourceAccess,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_id = lock_active_account(&mut transaction, account_id).await?;
            let (_, old_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            let affected = sqlx::query(
                r#"
INSERT INTO resourceaccesses
    (id, actorid, permissionlevel, resourceid, resourcetype, specificpermissions)
VALUES ($1, $2, $3, $4, $5, $6)
ON CONFLICT DO NOTHING
"#,
            )
            .bind(access.id.ok_or_else(|| {
                IdentityError::Validation("Resource access ID is required.".to_owned())
            })?)
            .bind(actor_id)
            .bind(access.permission_level as i32)
            .bind(access.resource_id)
            .bind(access.resource_type as i32)
            .bind(specific_mask(&access.specific_permissions))
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if affected != 1 {
                return Err(IdentityError::Conflict(
                    "The resource access is already assigned to the Service Account.".to_owned(),
                ));
            }
            let (name, new_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            insert_service_account_activity(
                &mut transaction,
                account_id,
                &name,
                changed_by_actor_id,
                ActivityEventInfo::service_account_updated(old_snapshot, new_snapshot),
                changed_at,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, account_id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn remove_resource_access(
        &self,
        account_id: Uuid,
        resource_access_id: Uuid,
        changed_by_actor_id: ActorId,
        changed_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<ServiceAccountView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_id = lock_active_account(&mut transaction, account_id).await?;
            let (_, old_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            let affected =
                sqlx::query("DELETE FROM resourceaccesses WHERE id = $1 AND actorid = $2")
                    .bind(resource_access_id)
                    .bind(actor_id)
                    .execute(&mut *transaction)
                    .await
                    .map_err(storage)?
                    .rows_affected();
            if affected != 1 {
                return Err(IdentityError::NotFound);
            }
            let (name, new_snapshot) = load_activity_snapshot(&mut transaction, account_id).await?;
            insert_service_account_activity(
                &mut transaction,
                account_id,
                &name,
                changed_by_actor_id,
                ActivityEventInfo::service_account_updated(old_snapshot, new_snapshot),
                changed_at,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            load_account(&self.pool, account_id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn list_tokens(
        &self,
        account_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> BoxFuture<'_, Result<StoredPage<ServiceAccountTokenView>, IdentityError>> {
        Box::pin(async move {
            if !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM serviceaccounts WHERE id = $1)",
            )
            .bind(account_id)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?
            {
                return Err(IdentityError::NotFound);
            }
            let rows = sqlx::query(
                r#"
SELECT token.id, token.name, token.expiresatutc, token.lastusedatutc,
       COUNT(*) OVER() AS totalitems,
       token.revokedatutc, token.revokedbyactorid, token.createdbyactorid, token.createdatutc,
       COALESCE(user_actor.name, service_actor.name, team_actor.name,
                CASE WHEN token.createdbyactorid = '00000000-0000-0000-0000-000000000001'::uuid
                     THEN 'Citadel' END, 'Unknown') AS createdbyname
FROM serviceaccounttokens token
LEFT JOIN users user_actor ON user_actor.actorid = token.createdbyactorid
LEFT JOIN serviceaccounts service_actor ON service_actor.actorid = token.createdbyactorid
LEFT JOIN teams team_actor ON team_actor.actorid = token.createdbyactorid
WHERE token.serviceaccountid = $1
ORDER BY token.createdatutc DESC, token.id DESC
LIMIT $2 OFFSET $3
"#,
            )
            .bind(account_id)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            let total_items = rows
                .first()
                .map(|row| row.try_get("totalitems").map_err(storage))
                .transpose()?
                .unwrap_or_default();
            Ok(StoredPage {
                total_items,
                items: rows.into_iter().map(map_token).collect::<Result<_, _>>()?,
            })
        })
    }

    fn create_token<'a>(
        &'a self,
        token: &'a NewServiceAccountToken,
        maximum_active: i64,
    ) -> BoxFuture<'a, Result<ServiceAccountTokenView, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let actor_available = sqlx::query_scalar::<_, bool>(
                r#"
SELECT actor.isenabled
FROM serviceaccounts account
JOIN actors actor ON actor.id = account.actorid
WHERE account.id = $1 AND account.archivedatutc IS NULL
FOR UPDATE OF account
"#,
            )
            .bind(token.service_account_id)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?;
            if actor_available != Some(true) {
                return Err(IdentityError::NotFound);
            }
            if sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM serviceaccounttokens WHERE serviceaccountid = $1 AND name = $2)",
            )
            .bind(token.service_account_id)
            .bind(&token.name)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?
            {
                return Err(IdentityError::Conflict(
                    "A token with this name already exists for the Service Account.".to_owned(),
                ));
            }
            let active = sqlx::query_scalar::<_, i64>(
                r#"
SELECT COUNT(*) FROM serviceaccounttokens
WHERE serviceaccountid = $1 AND revokedatutc IS NULL
  AND (expiresatutc IS NULL OR expiresatutc > $2)
"#,
            )
            .bind(token.service_account_id)
            .bind(token.created_at_utc)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if active >= maximum_active {
                return Err(IdentityError::Conflict(
                    "The Service Account has reached its active-token limit.".to_owned(),
                ));
            }
            sqlx::query(
                r#"
INSERT INTO serviceaccounttokens
    (id, serviceaccountid, name, secrethash, expiresatutc, lastusedatutc,
     revokedatutc, revokedbyactorid, createdbyactorid, createdatutc)
VALUES ($1, $2, $3, $4, $5, NULL, NULL, NULL, $6, $7)
"#,
            )
            .bind(token.id)
            .bind(token.service_account_id)
            .bind(&token.name)
            .bind(token.secret_hash.as_slice())
            .bind(token.expires_at_utc)
            .bind(token.created_by_actor_id.value())
            .bind(token.created_at_utc)
            .execute(&mut *transaction)
            .await
            .map_err(conflict_or_storage)?;
            let (account_name, _) =
                load_activity_snapshot(&mut transaction, token.service_account_id).await?;
            insert_service_account_activity(
                &mut transaction,
                token.service_account_id,
                &account_name,
                token.created_by_actor_id,
                ActivityEventInfo::service_account_token_created(
                    token.service_account_id,
                    token.id,
                    token.name.clone(),
                    token_public_hint(token.id),
                    token.expires_at_utc,
                ),
                token.created_at_utc,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            load_token(&self.pool, token.service_account_id, token.id)
                .await?
                .ok_or(IdentityError::NotFound)
        })
    }

    fn revoke_token(
        &self,
        account_id: Uuid,
        token_id: Uuid,
        actor_id: ActorId,
        revoked_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            if !sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM serviceaccounts WHERE id = $1)",
            )
            .bind(account_id)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?
            {
                return Err(IdentityError::NotFound);
            }
            let affected = sqlx::query(
                r#"
UPDATE serviceaccounttokens
SET revokedatutc = COALESCE(revokedatutc, $3), revokedbyactorid = COALESCE(revokedbyactorid, $4)
WHERE serviceaccountid = $1 AND id = $2 AND revokedatutc IS NULL
"#,
            )
            .bind(account_id)
            .bind(token_id)
            .bind(revoked_at)
            .bind(actor_id.value())
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if affected == 1 {
                let (account_name, _) =
                    load_activity_snapshot(&mut transaction, account_id).await?;
                insert_service_account_activity(
                    &mut transaction,
                    account_id,
                    &account_name,
                    actor_id,
                    ActivityEventInfo::service_account_token_revoked(
                        account_id,
                        token_id,
                        token_public_hint(token_id),
                    ),
                    revoked_at,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

async fn lock_active_account(
    transaction: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    account_id: Uuid,
) -> Result<Uuid, IdentityError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT actorid FROM serviceaccounts WHERE id = $1 AND archivedatutc IS NULL FOR UPDATE",
    )
    .bind(account_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?
    .ok_or(IdentityError::NotFound)
}

async fn load_activity_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: Uuid,
) -> Result<(String, ServiceAccountActivitySnapshot), IdentityError> {
    let row = sqlx::query(
        r#"
SELECT account.name, account.description, account.actorid, actor.isenabled
FROM serviceaccounts account
JOIN actors actor ON actor.id = account.actorid AND actor.type = 'ServiceAccount'
WHERE account.id = $1
"#,
    )
    .bind(account_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?
    .ok_or(IdentityError::NotFound)?;
    let actor_id: Uuid = row.try_get("actorid").map_err(storage)?;
    let name: String = row.try_get("name").map_err(storage)?;
    let mut team_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT teamid FROM actorteammemberships WHERE memberactorid = $1 ORDER BY teamid",
    )
    .bind(actor_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    let mut role_ids = sqlx::query_scalar::<_, Uuid>(
        "SELECT roleid FROM actorroles WHERE actorid = $1 ORDER BY roleid",
    )
    .bind(actor_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    team_ids.sort_unstable();
    role_ids.sort_unstable();
    let access_rows = sqlx::query(
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
    .map_err(storage)?;
    let resource_accesses = access_rows
        .into_iter()
        .map(|access| {
            let resource_type_value = access.try_get("resourcetype").map_err(storage)?;
            let permission_level_value = access.try_get("permissionlevel").map_err(storage)?;
            Ok(ServiceAccountResourceAccessSnapshot {
                resource_type: ResourceType::from_i32(resource_type_value).ok_or_else(|| {
                    IdentityError::Storage(format!(
                        "unknown persisted ResourceType value {resource_type_value}"
                    ))
                })?,
                resource_id: access.try_get("resourceid").map_err(storage)?,
                permission_level: PermissionLevel::from_i32(permission_level_value).ok_or_else(
                    || {
                        IdentityError::Storage(format!(
                            "unknown persisted PermissionLevel value {permission_level_value}"
                        ))
                    },
                )?,
                specific_permissions: access.try_get("specificpermissions").map_err(storage)?,
            })
        })
        .collect::<Result<Vec<_>, IdentityError>>()?;
    Ok((
        name.clone(),
        ServiceAccountActivitySnapshot {
            id: account_id,
            name,
            description: row.try_get("description").map_err(storage)?,
            is_enabled: row.try_get("isenabled").map_err(storage)?,
            team_ids,
            role_ids,
            resource_accesses,
        },
    ))
}

async fn insert_service_account_activity(
    transaction: &mut Transaction<'_, Postgres>,
    account_id: Uuid,
    account_name: &str,
    actor_id: ActorId,
    info: ActivityEventInfo,
    created_at: DateTime<Utc>,
) -> Result<(), IdentityError> {
    let activity = ActivityEvent::new_service_account_event(
        account_id,
        account_name.to_owned(),
        actor_id,
        info,
        created_at,
    )
    .map_err(invalid_activity)?;
    insert_activity(transaction, &activity).await
}

fn sorted_ids(values: &[Uuid]) -> Vec<Uuid> {
    let mut values = values.to_vec();
    values.sort_unstable();
    values
}

fn snapshot_accesses(
    values: &[ServiceAccountResourceAccess],
) -> Vec<ServiceAccountResourceAccessSnapshot> {
    let mut values = values
        .iter()
        .map(|access| ServiceAccountResourceAccessSnapshot {
            resource_type: access.resource_type,
            resource_id: access.resource_id,
            permission_level: access.permission_level,
            specific_permissions: specific_mask(&access.specific_permissions),
        })
        .collect::<Vec<_>>();
    values.sort_unstable_by_key(|access| (access.resource_type, access.resource_id));
    values
}

fn token_public_hint(id: Uuid) -> String {
    format!("cit_sa_{}", id.simple())[..15].to_owned()
}

async fn load_account(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<ServiceAccountView>, IdentityError> {
    let row = sqlx::query(
        r#"
SELECT account.id, account.name, account.description, account.actorid, actor.isenabled,
       account.createdat, account.createdbyactorid, account.updatedat, account.archivedatutc,
       (SELECT COUNT(*) FROM serviceaccounttokens token
        WHERE token.serviceaccountid = account.id AND token.revokedatutc IS NULL
          AND (token.expiresatutc IS NULL OR token.expiresatutc > CURRENT_TIMESTAMP)) AS activetokencount,
       (SELECT MAX(token.lastusedatutc) FROM serviceaccounttokens token
        WHERE token.serviceaccountid = account.id) AS lastusedatutc,
       COALESCE((SELECT jsonb_agg(jsonb_build_object('id', role.id, 'name', role.name) ORDER BY role.name, role.id)
                 FROM actorroles assignment JOIN roles role ON role.id = assignment.roleid
                 WHERE assignment.actorid = account.actorid), '[]'::jsonb) AS roles,
       COALESCE((SELECT jsonb_agg(jsonb_build_object('id', team.id, 'name', team.name) ORDER BY team.name, team.id)
                 FROM actorteammemberships membership JOIN teams team ON team.id = membership.teamid
                 WHERE membership.memberactorid = account.actorid), '[]'::jsonb) AS teams,
       COALESCE((SELECT jsonb_agg(jsonb_build_object(
                    'id', access.id,
                    'resourceType', access.resourcetype,
                    'resourceId', access.resourceid,
                    'permissionLevel', access.permissionlevel,
                    'specificPermissions', access.specificpermissions) ORDER BY access.resourcetype, access.resourceid)
                 FROM resourceaccesses access WHERE access.actorid = account.actorid), '[]'::jsonb) AS resourceaccesses
FROM serviceaccounts account
JOIN actors actor ON actor.id = account.actorid AND actor.type = 'ServiceAccount'
WHERE account.id = $1
"#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    row.map(map_account).transpose()
}

async fn load_token(
    pool: &PgPool,
    account_id: Uuid,
    token_id: Uuid,
) -> Result<Option<ServiceAccountTokenView>, IdentityError> {
    let row = sqlx::query(
        r#"
SELECT token.id, token.name, token.expiresatutc, token.lastusedatutc,
       token.revokedatutc, token.revokedbyactorid, token.createdbyactorid, token.createdatutc,
       COALESCE(user_actor.name, service_actor.name, team_actor.name,
                CASE WHEN token.createdbyactorid = '00000000-0000-0000-0000-000000000001'::uuid
                     THEN 'Citadel' END, 'Unknown') AS createdbyname
FROM serviceaccounttokens token
LEFT JOIN users user_actor ON user_actor.actorid = token.createdbyactorid
LEFT JOIN serviceaccounts service_actor ON service_actor.actorid = token.createdbyactorid
LEFT JOIN teams team_actor ON team_actor.actorid = token.createdbyactorid
WHERE token.serviceaccountid = $1 AND token.id = $2
"#,
    )
    .bind(account_id)
    .bind(token_id)
    .fetch_optional(pool)
    .await
    .map_err(storage)?;
    row.map(map_token).transpose()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PersistedAccess {
    id: Uuid,
    resource_type: i32,
    resource_id: Uuid,
    permission_level: i32,
    specific_permissions: i32,
}

#[derive(Deserialize)]
struct PersistedResourceInfo {
    id: Uuid,
    name: String,
}

fn map_account(row: PgRow) -> Result<ServiceAccountView, IdentityError> {
    let accesses = serde_json::from_value::<Vec<PersistedAccess>>(
        row.try_get("resourceaccesses").map_err(storage)?,
    )
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
        let specific_permissions = citadel_domain::SpecificPermission::ALL
            .into_iter()
            .filter(|permission| access.specific_permissions & *permission as i32 != 0)
            .collect();
        Ok(ServiceAccountResourceAccess {
            id: Some(access.id),
            resource_type,
            resource_id: access.resource_id,
            resource_name: None,
            permission_level,
            specific_permissions,
        })
    })
    .collect::<Result<Vec<_>, IdentityError>>()?;
    Ok(ServiceAccountView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
        is_enabled: row.try_get("isenabled").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        created_by_actor_id: ActorId::new(row.try_get("createdbyactorid").map_err(storage)?),
        updated_at: row.try_get("updatedat").map_err(storage)?,
        archived_at_utc: row.try_get("archivedatutc").map_err(storage)?,
        active_token_count: row.try_get("activetokencount").map_err(storage)?,
        last_used_at_utc: row.try_get("lastusedatutc").map_err(storage)?,
        teams: map_resource_info(row.try_get("teams").map_err(storage)?)?,
        roles: map_resource_info(row.try_get("roles").map_err(storage)?)?,
        resource_accesses: accesses,
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

fn map_token(row: PgRow) -> Result<ServiceAccountTokenView, IdentityError> {
    let id: Uuid = row.try_get("id").map_err(storage)?;
    Ok(ServiceAccountTokenView {
        id,
        name: row.try_get("name").map_err(storage)?,
        hint: format!("cit_sa_{}", &id.simple().to_string()[..8]),
        expires_at_utc: row.try_get("expiresatutc").map_err(storage)?,
        last_used_at_utc: row.try_get("lastusedatutc").map_err(storage)?,
        revoked_at_utc: row.try_get("revokedatutc").map_err(storage)?,
        revoked_by_actor_id: row
            .try_get::<Option<Uuid>, _>("revokedbyactorid")
            .map_err(storage)?
            .map(ActorId::new),
        created_by_actor_id: ActorId::new(row.try_get("createdbyactorid").map_err(storage)?),
        created_by_name: row.try_get("createdbyname").map_err(storage)?,
        created_at_utc: row.try_get("createdatutc").map_err(storage)?,
    })
}

fn specific_mask(permissions: &[citadel_domain::SpecificPermission]) -> i32 {
    permissions
        .iter()
        .fold(0, |mask, permission| mask | *permission as i32)
}

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

fn conflict_or_storage(error: sqlx::Error) -> IdentityError {
    if error
        .as_database_error()
        .is_some_and(|database| database.code().as_deref() == Some("23505"))
    {
        IdentityError::Conflict("A resource with the same name already exists.".to_owned())
    } else {
        storage(error)
    }
}
