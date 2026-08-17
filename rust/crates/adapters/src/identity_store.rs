use chrono::{DateTime, Utc};
use citadel_domain::{
    ActivityEvent, ActivityEventInfo, ActorId, AuthenticatedPrincipalType, PermissionLevel,
    ResourceType,
};
use citadel_identity::{
    ADMIN_ROLE_ID, ActorPrincipal, AuthorizationSnapshot, EntitlementService, IdentityError,
    IdentityStore, NewSession, PermissionGrant, ServiceAccountCredential,
    ServiceAccountLastUsedStore, SessionMetadata, User, UserAuthentication, UserSessionRecord,
};
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::activity_store::{insert_activity, invalid_activity};

#[derive(Clone)]
pub struct PostgresIdentityStore {
    pool: PgPool,
}

impl PostgresIdentityStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl IdentityStore for PostgresIdentityStore {
    fn setup_required(&self) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            sqlx::query_scalar::<_, bool>(
                "SELECT initializedat IS NULL FROM instancesetupstates WHERE id = 1",
            )
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or_else(|| IdentityError::Storage("setup state row is missing".to_owned()))
        })
    }

    fn initialize_administrator<'a>(
        &'a self,
        administrator: &'a User,
    ) -> BoxFuture<'a, Result<UserAuthentication, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let pending = sqlx::query_scalar::<_, bool>(
                "SELECT initializedat IS NULL FROM instancesetupstates WHERE id = 1 FOR UPDATE",
            )
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?
            .ok_or_else(|| IdentityError::Storage("setup state row is missing".to_owned()))?;
            if !pending {
                return Err(IdentityError::SetupAlreadyComplete);
            }
            let conflict = sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM users WHERE lower(name) = lower($1) OR lower(email) = lower($2))",
            )
            .bind(administrator.name())
            .bind(administrator.email())
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            if conflict {
                return Err(IdentityError::Conflict(
                    "A User with that name or email already exists.".to_owned(),
                ));
            }

            sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
                .bind(administrator.actor_id().value())
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query(
                r#"
INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name, password)
VALUES ($1, $2, $3, $4, $5, $6, $7)
"#,
            )
            .bind(administrator.id())
            .bind(administrator.actor_id().value())
            .bind(administrator.created_at())
            .bind(administrator.created_by_actor_id().value())
            .bind(administrator.email())
            .bind(administrator.name())
            .bind(administrator.password_hash())
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            sqlx::query("INSERT INTO actorroles (actorid, roleid) VALUES ($1, $2)")
                .bind(administrator.actor_id().value())
                .bind(ADMIN_ROLE_ID)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            let completed = sqlx::query(
                r#"
UPDATE instancesetupstates
SET initialadministratoractorid = $1, initializedat = $2, updatedat = $2
WHERE id = 1 AND initializedat IS NULL
"#,
            )
            .bind(administrator.actor_id().value())
            .bind(administrator.created_at())
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if completed != 1 {
                return Err(IdentityError::SetupAlreadyComplete);
            }
            transaction.commit().await.map_err(storage)?;
            Ok(UserAuthentication {
                user_id: administrator.id(),
                actor_id: administrator.actor_id(),
                name: administrator.name().to_owned(),
                email: administrator.email().to_owned(),
                password_hash: administrator.password_hash().map(str::to_owned),
                enabled: true,
                roles: vec!["Admin".to_owned()],
            })
        })
    }

    fn find_user_for_login<'a>(
        &'a self,
        identifier: &'a str,
    ) -> BoxFuture<'a, Result<Option<UserAuthentication>, IdentityError>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
SELECT u.id, u.actorid, u.name, u.email, u.password, a.isenabled,
       COALESCE(array_agg(DISTINCT r.name) FILTER (WHERE r.name IS NOT NULL), ARRAY[]::text[]) AS roles
FROM users u
JOIN actors a ON a.id = u.actorid AND a.type = 'User'
LEFT JOIN (
    SELECT ar.actorid, ar.roleid FROM actorroles ar
    UNION
    SELECT atm.memberactorid, ar.roleid
    FROM actorteammemberships atm
    JOIN teams t ON t.id = atm.teamid
    JOIN actors ta ON ta.id = t.actorid AND ta.isenabled
    JOIN actorroles ar ON ar.actorid = t.actorid
) effective_roles ON effective_roles.actorid = u.actorid
LEFT JOIN roles r ON r.id = effective_roles.roleid
WHERE lower(u.name) = lower($1) OR lower(u.email) = lower($1)
GROUP BY u.id, u.actorid, u.name, u.email, u.password, a.isenabled
LIMIT 1
"#,
            )
            .bind(identifier)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            row.map(map_user_authentication).transpose()
        })
    }

    fn load_user_by_id(
        &self,
        user_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<UserAuthentication>, IdentityError>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
SELECT u.id, u.actorid, u.name, u.email, u.password, a.isenabled,
       COALESCE(array_agg(DISTINCT r.name) FILTER (WHERE r.name IS NOT NULL), ARRAY[]::text[]) AS roles
FROM users u
JOIN actors a ON a.id = u.actorid AND a.type = 'User'
LEFT JOIN (
    SELECT ar.actorid, ar.roleid FROM actorroles ar
    UNION
    SELECT atm.memberactorid, ar.roleid
    FROM actorteammemberships atm
    JOIN teams t ON t.id = atm.teamid
    JOIN actors ta ON ta.id = t.actorid AND ta.isenabled
    JOIN actorroles ar ON ar.actorid = t.actorid
) effective_roles ON effective_roles.actorid = u.actorid
LEFT JOIN roles r ON r.id = effective_roles.roleid
WHERE u.id = $1
GROUP BY u.id, u.actorid, u.name, u.email, u.password, a.isenabled
"#,
            )
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            row.map(map_user_authentication).transpose()
        })
    }

    fn create_session<'a>(
        &'a self,
        session: &'a NewSession,
        expected_password_hash: Option<&'a str>,
        maximum_sessions: i64,
    ) -> BoxFuture<'a, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            insert_session(
                &mut transaction,
                session,
                expected_password_hash,
                maximum_sessions,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }

    fn load_session_user(
        &self,
        session_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<UserAuthentication>, IdentityError>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
SELECT u.id, u.actorid, u.name, u.email, u.password, a.isenabled,
       COALESCE(array_agg(DISTINCT r.name) FILTER (WHERE r.name IS NOT NULL), ARRAY[]::text[]) AS roles
FROM refreshtokens rt
JOIN users u ON u.id = rt.userid
JOIN actors a ON a.id = u.actorid AND a.type = 'User'
LEFT JOIN (
    SELECT ar.actorid, ar.roleid FROM actorroles ar
    UNION
    SELECT atm.memberactorid, ar.roleid
    FROM actorteammemberships atm
    JOIN teams t ON t.id = atm.teamid
    JOIN actors ta ON ta.id = t.actorid AND ta.isenabled
    JOIN actorroles ar ON ar.actorid = t.actorid
) effective_roles ON effective_roles.actorid = u.actorid
LEFT JOIN roles r ON r.id = effective_roles.roleid
WHERE rt.id = $1 AND rt.expiresat > $2
GROUP BY u.id, u.actorid, u.name, u.email, u.password, a.isenabled
"#,
            )
            .bind(session_id)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            row.map(map_user_authentication).transpose()
        })
    }

    fn touch_session<'a>(
        &'a self,
        session_id: Uuid,
        now: DateTime<Utc>,
        metadata: &'a SessionMetadata,
    ) -> BoxFuture<'a, Result<bool, IdentityError>> {
        Box::pin(async move {
            let affected = sqlx::query(
                r#"
UPDATE refreshtokens
SET lastseenat = $2, useragent = $3, ipaddress = $4
WHERE id = $1 AND expiresat > $2
"#,
            )
            .bind(session_id)
            .bind(now)
            .bind(&metadata.user_agent)
            .bind(&metadata.ip_address)
            .execute(&self.pool)
            .await
            .map_err(storage)?
            .rows_affected();
            Ok(affected == 1)
        })
    }

    fn delete_session(&self, session_id: Uuid) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            sqlx::query("DELETE FROM refreshtokens WHERE id = $1")
                .bind(session_id)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
            Ok(())
        })
    }

    fn active_session_id(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<Uuid>, IdentityError>> {
        Box::pin(async move {
            sqlx::query_scalar(
                r#"
SELECT id
FROM refreshtokens
WHERE id = $1 AND userid = $2 AND expiresat > $3
"#,
            )
            .bind(session_id)
            .bind(user_id)
            .bind(now)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)
        })
    }

    fn list_active_sessions(
        &self,
        user_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Vec<UserSessionRecord>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                r#"
SELECT id, useragent, ipaddress, createdat, lastseenat, expiresat
FROM refreshtokens
WHERE userid = $1 AND expiresat > $2
ORDER BY lastseenat DESC, createdat DESC, id DESC
"#,
            )
            .bind(user_id)
            .bind(now)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_user_session)
            .collect()
        })
    }

    fn delete_owned_session(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        current_session_id: Option<Uuid>,
        actor_id: ActorId,
        revoked_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let resource_name =
                sqlx::query_scalar::<_, String>("SELECT name FROM users WHERE id = $1 FOR UPDATE")
                    .bind(user_id)
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(storage)?
                    .ok_or(IdentityError::NotFound)?;
            let affected = sqlx::query(
                r#"
DELETE FROM refreshtokens
WHERE id = $1
  AND userid = $2
  AND ($3::uuid IS NULL OR id <> $3)
"#,
            )
            .bind(session_id)
            .bind(user_id)
            .bind(current_session_id)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?
            .rows_affected();
            if affected == 1 {
                let activity = ActivityEvent::new_user_event(
                    user_id,
                    resource_name,
                    actor_id,
                    ActivityEventInfo::user_session_revoked(session_id),
                    revoked_at,
                )
                .map_err(invalid_activity)?;
                insert_activity(&mut transaction, &activity).await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(affected == 1)
        })
    }

    fn delete_other_sessions(
        &self,
        user_id: Uuid,
        current_session_id: Uuid,
        now: DateTime<Utc>,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<Option<i64>, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let Some(resource_name) =
                sqlx::query_scalar::<_, String>("SELECT name FROM users WHERE id = $1 FOR UPDATE")
                    .bind(user_id)
                    .fetch_optional(&mut *transaction)
                    .await
                    .map_err(storage)?
            else {
                return Ok(None);
            };
            let row = sqlx::query(
                r#"
WITH current_session AS MATERIALIZED (
    SELECT id
    FROM refreshtokens
    WHERE id = $2 AND userid = $1 AND expiresat > $3
    FOR UPDATE
),
deleted AS (
    DELETE FROM refreshtokens
    WHERE userid = $1
      AND id <> $2
      AND EXISTS (SELECT 1 FROM current_session)
    RETURNING id
)
SELECT EXISTS (SELECT 1 FROM current_session) AS current_exists,
       (SELECT count(*) FROM deleted) AS deleted_count
"#,
            )
            .bind(user_id)
            .bind(current_session_id)
            .bind(now)
            .fetch_one(&mut *transaction)
            .await
            .map_err(storage)?;
            let current_exists = row.try_get::<bool, _>("current_exists").map_err(storage)?;
            let deleted_count = row.try_get::<i64, _>("deleted_count").map_err(storage)?;
            if current_exists && deleted_count > 0 {
                let info = ActivityEventInfo::user_other_sessions_revoked(deleted_count)
                    .map_err(invalid_activity)?;
                let activity =
                    ActivityEvent::new_user_event(user_id, resource_name, actor_id, info, now)
                        .map_err(invalid_activity)?;
                insert_activity(&mut transaction, &activity).await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(current_exists.then_some(deleted_count))
        })
    }

    fn load_principal(
        &self,
        actor_id: ActorId,
        expected_subject_id: Uuid,
        principal_type: AuthenticatedPrincipalType,
        credential_id: Option<Uuid>,
    ) -> BoxFuture<'_, Result<Option<ActorPrincipal>, IdentityError>> {
        Box::pin(async move {
            let expected_actor_type = match principal_type {
                AuthenticatedPrincipalType::User => "User",
                AuthenticatedPrincipalType::ServiceAccount => "ServiceAccount",
                _ => return Ok(None),
            };
            let row = sqlx::query(
                r#"
WITH effective_roles AS (
    SELECT ar.actorid, ar.roleid FROM actorroles ar WHERE ar.actorid = $1
    UNION
    SELECT atm.memberactorid, ar.roleid
    FROM actorteammemberships atm
    JOIN teams t ON t.id = atm.teamid
    JOIN actors ta ON ta.id = t.actorid AND ta.isenabled
    JOIN actorroles ar ON ar.actorid = t.actorid
    WHERE atm.memberactorid = $1
), subjects AS (
    SELECT id, actorid, name, 'User'::text AS actortype, NULL::timestamptz AS archivedatutc
    FROM users
    UNION ALL
    SELECT id, actorid, name, 'ServiceAccount'::text AS actortype, archivedatutc
    FROM serviceaccounts
)
SELECT subject.id, subject.name,
       COALESCE(array_agg(DISTINCT r.name) FILTER (WHERE r.name IS NOT NULL), ARRAY[]::text[]) AS roles
FROM subjects subject
JOIN actors a ON a.id = subject.actorid AND a.type = $3 AND a.isenabled
LEFT JOIN effective_roles er ON er.actorid = a.id
LEFT JOIN roles r ON r.id = er.roleid
WHERE subject.actorid = $1 AND subject.id = $2 AND subject.actortype = $3
  AND ($3 <> 'ServiceAccount' OR subject.archivedatutc IS NULL)
GROUP BY subject.id, subject.name
"#,
            )
                .bind(actor_id.value())
                .bind(expected_subject_id)
                .bind(expected_actor_type)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?;
            row.map(|row| {
                Ok(ActorPrincipal {
                    subject_id: row.try_get("id").map_err(storage)?,
                    actor_id,
                    name: row.try_get("name").map_err(storage)?,
                    principal_type,
                    credential_id,
                    roles: row.try_get("roles").map_err(storage)?,
                })
            })
            .transpose()
        })
    }

    fn load_service_account_credential(
        &self,
        credential_id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ServiceAccountCredential>, IdentityError>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
WITH effective_roles AS (
    SELECT ar.actorid, ar.roleid
    FROM actorroles ar
    JOIN serviceaccounts sa ON sa.actorid = ar.actorid
    JOIN serviceaccounttokens token ON token.serviceaccountid = sa.id
    WHERE token.id = $1
    UNION
    SELECT atm.memberactorid, ar.roleid
    FROM serviceaccounttokens token
    JOIN serviceaccounts sa ON sa.id = token.serviceaccountid
    JOIN actorteammemberships atm ON atm.memberactorid = sa.actorid
    JOIN teams t ON t.id = atm.teamid
    JOIN actors ta ON ta.id = t.actorid AND ta.isenabled
    JOIN actorroles ar ON ar.actorid = t.actorid
    WHERE token.id = $1
)
SELECT token.id AS credentialid, sa.id AS serviceaccountid, sa.actorid, sa.name,
       token.secrethash, token.expiresatutc, token.revokedatutc, sa.archivedatutc,
       actor.isenabled,
       COALESCE(array_agg(DISTINCT role.name) FILTER (WHERE role.name IS NOT NULL), ARRAY[]::text[]) AS roles
FROM serviceaccounttokens token
JOIN serviceaccounts sa ON sa.id = token.serviceaccountid
JOIN actors actor ON actor.id = sa.actorid AND actor.type = 'ServiceAccount'
LEFT JOIN effective_roles er ON er.actorid = sa.actorid
LEFT JOIN roles role ON role.id = er.roleid
WHERE token.id = $1
GROUP BY token.id, sa.id, sa.actorid, sa.name, token.secrethash, token.expiresatutc,
         token.revokedatutc, sa.archivedatutc, actor.isenabled
"#,
            )
            .bind(credential_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            row.map(map_service_account_credential).transpose()
        })
    }

    fn authorization_snapshot(
        &self,
        actor_id: ActorId,
    ) -> BoxFuture<'_, Result<AuthorizationSnapshot, IdentityError>> {
        Box::pin(async move {
            let enabled =
                sqlx::query_scalar::<_, bool>("SELECT isenabled FROM actors WHERE id = $1")
                    .bind(actor_id.value())
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .unwrap_or(false);
            let rows = sqlx::query(
                r#"
WITH effective_roles AS (
    SELECT ar.actorid, ar.roleid
    FROM actorroles ar
    WHERE ar.actorid = $1
    UNION
    SELECT atm.memberactorid, ar.roleid
    FROM actorteammemberships atm
    JOIN teams t ON t.id = atm.teamid
    JOIN actors team_actor ON team_actor.id = t.actorid AND team_actor.isenabled
    JOIN actorroles ar ON ar.actorid = t.actorid
    WHERE atm.memberactorid = $1
)
SELECT p.resourcetype, MAX(p.permissionlevel) AS permissionlevel,
       bit_or(p.specificpermissions) AS specificpermissions
FROM effective_roles er
JOIN permissions p ON p.roleid = er.roleid
GROUP BY p.resourcetype
ORDER BY p.resourcetype
"#,
            )
            .bind(actor_id.value())
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            let grants = rows
                .into_iter()
                .map(|row| {
                    let resource = row.try_get::<i32, _>("resourcetype").map_err(storage)?;
                    let level = row.try_get::<i32, _>("permissionlevel").map_err(storage)?;
                    Ok(PermissionGrant {
                        resource_type: ResourceType::from_i32(resource).ok_or_else(|| {
                            IdentityError::Storage(format!(
                                "unknown persisted ResourceType value {resource}"
                            ))
                        })?,
                        level: PermissionLevel::from_i32(level).ok_or_else(|| {
                            IdentityError::Storage(format!(
                                "unknown persisted PermissionLevel value {level}"
                            ))
                        })?,
                        specific_mask: row.try_get("specificpermissions").map_err(storage)?,
                    })
                })
                .collect::<Result<Vec<_>, IdentityError>>()?;
            Ok(AuthorizationSnapshot {
                actor_id,
                enabled,
                direct_and_team_permissions: grants,
            })
        })
    }

    fn resource_permission<'a>(
        &'a self,
        actor_id: ActorId,
        resource_type: ResourceType,
        resource_id: Uuid,
    ) -> BoxFuture<'a, Result<Option<PermissionGrant>, IdentityError>> {
        Box::pin(async move {
            let row = sqlx::query(
                r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT t.actorid
    FROM actorteammemberships membership
    JOIN actors member_actor ON member_actor.id = membership.memberactorid AND member_actor.isenabled
    JOIN teams t ON t.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = t.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), effective_roles AS (
    SELECT DISTINCT role_assignment.roleid
    FROM actor_scope scope
    JOIN actorroles role_assignment ON role_assignment.actorid = scope.actorid
), candidates AS (
    SELECT permission.permissionlevel, permission.specificpermissions
    FROM effective_roles role
    JOIN permissions permission ON permission.roleid = role.roleid
    WHERE permission.resourcetype = $2
    UNION ALL
    SELECT access.permissionlevel, access.specificpermissions
    FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $2 AND access.resourceid = $3
)
SELECT MAX(permissionlevel) AS permissionlevel,
       bit_or(specificpermissions) AS specificpermissions
FROM candidates
"#,
            )
            .bind(actor_id.value())
            .bind(resource_type as i32)
            .bind(resource_id)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)?;
            let level = row
                .try_get::<Option<i32>, _>("permissionlevel")
                .map_err(storage)?;
            let Some(level) = level else {
                return Ok(None);
            };
            Ok(Some(PermissionGrant {
                resource_type,
                level: PermissionLevel::from_i32(level).ok_or_else(|| {
                    IdentityError::Storage(format!(
                        "unknown persisted PermissionLevel value {level}"
                    ))
                })?,
                specific_mask: row
                    .try_get::<Option<i32>, _>("specificpermissions")
                    .map_err(storage)?
                    .unwrap_or_default(),
            }))
        })
    }
}

pub(crate) async fn insert_session(
    transaction: &mut Transaction<'_, Postgres>,
    session: &NewSession,
    expected_password_hash: Option<&str>,
    maximum_sessions: i64,
) -> Result<(), IdentityError> {
    let persisted_password = sqlx::query_scalar::<_, Option<String>>(
        r#"
SELECT u.password
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User' AND actor.isenabled
WHERE u.id = $1
FOR UPDATE OF u
"#,
    )
    .bind(session.user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?
    .ok_or(IdentityError::NotFound)?;
    if persisted_password.as_deref() != expected_password_hash {
        return Err(IdentityError::InvalidCredentials);
    }
    sqlx::query(
        r#"
INSERT INTO refreshtokens (id, createdat, expiresat, ipaddress, lastseenat, useragent, userid)
VALUES ($1, $2, $3, $4, $2, $5, $6)
"#,
    )
    .bind(session.id)
    .bind(session.created_at)
    .bind(session.expires_at)
    .bind(&session.metadata.ip_address)
    .bind(&session.metadata.user_agent)
    .bind(session.user_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    sqlx::query(
        r#"
DELETE FROM refreshtokens
WHERE id IN (
    SELECT id FROM refreshtokens
    WHERE userid = $1
    ORDER BY createdat DESC, id DESC
    OFFSET $2
)
"#,
    )
    .bind(session.user_id)
    .bind(maximum_sessions)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

impl ServiceAccountLastUsedStore for PostgresIdentityStore {
    fn update_service_account_last_used(
        &self,
        credential_id: Uuid,
        used_at: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                r#"
UPDATE serviceaccounttokens
SET lastusedatutc = GREATEST(COALESCE(lastusedatutc, $2), $2)
WHERE id = $1
"#,
            )
            .bind(credential_id)
            .bind(used_at)
            .execute(&self.pool)
            .await
            .map_err(storage)?;
            Ok(())
        })
    }
}

fn map_user_authentication(
    row: sqlx::postgres::PgRow,
) -> Result<UserAuthentication, IdentityError> {
    Ok(UserAuthentication {
        user_id: row.try_get("id").map_err(storage)?,
        actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
        name: row.try_get("name").map_err(storage)?,
        email: row
            .try_get::<Option<String>, _>("email")
            .map_err(storage)?
            .unwrap_or_default(),
        password_hash: row.try_get("password").map_err(storage)?,
        enabled: row.try_get("isenabled").map_err(storage)?,
        roles: row.try_get("roles").map_err(storage)?,
    })
}

fn map_user_session(row: sqlx::postgres::PgRow) -> Result<UserSessionRecord, IdentityError> {
    Ok(UserSessionRecord {
        id: row.try_get("id").map_err(storage)?,
        user_agent: row.try_get("useragent").map_err(storage)?,
        ip_address: row.try_get("ipaddress").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        last_seen_at: row.try_get("lastseenat").map_err(storage)?,
        expires_at: row.try_get("expiresat").map_err(storage)?,
    })
}

fn map_service_account_credential(
    row: sqlx::postgres::PgRow,
) -> Result<ServiceAccountCredential, IdentityError> {
    let secret_hash = row.try_get::<Vec<u8>, _>("secrethash").map_err(storage)?;
    let secret_hash: [u8; 32] = secret_hash.try_into().map_err(|_| {
        IdentityError::Storage("Service Account digest has an invalid length".to_owned())
    })?;
    Ok(ServiceAccountCredential {
        credential_id: row.try_get("credentialid").map_err(storage)?,
        service_account_id: row.try_get("serviceaccountid").map_err(storage)?,
        actor_id: ActorId::new(row.try_get("actorid").map_err(storage)?),
        name: row.try_get("name").map_err(storage)?,
        secret_hash,
        expires_at: row.try_get("expiresatutc").map_err(storage)?,
        revoked_at: row.try_get("revokedatutc").map_err(storage)?,
        archived_at: row.try_get("archivedatutc").map_err(storage)?,
        enabled: row.try_get("isenabled").map_err(storage)?,
        roles: row.try_get("roles").map_err(storage)?,
    })
}

fn storage(error: sqlx::Error) -> IdentityError {
    IdentityError::Storage(error.to_string())
}

#[derive(Debug, Clone)]
pub struct StaticEntitlementService {
    custom_access_control: bool,
}

impl StaticEntitlementService {
    #[must_use]
    pub const fn new(custom_access_control: bool) -> Self {
        Self {
            custom_access_control,
        }
    }
}

impl EntitlementService for StaticEntitlementService {
    fn custom_access_control_enabled(&self) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move { Ok(self.custom_access_control) })
    }
}

pub async fn lock_actor(
    transaction: &mut Transaction<'_, Postgres>,
    actor_id: ActorId,
) -> Result<(), IdentityError> {
    sqlx::query("SELECT id FROM actors WHERE id = $1 FOR UPDATE")
        .bind(actor_id.value())
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .ok_or(IdentityError::NotFound)?;
    Ok(())
}
