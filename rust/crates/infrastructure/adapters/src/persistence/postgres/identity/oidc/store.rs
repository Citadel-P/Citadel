use chrono::{DateTime, Utc};
use citadel_activities::{
    ActivityEvent, ActivityEventInfo, IdentityResourceAccessSnapshot, UserActivitySnapshot,
};
use citadel_identity::{
    IdentityError, OidcIdentity, OidcLoginState, OidcProvider, OidcStore, SYSTEM_ACTOR_ID,
    UserAuthentication,
};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use sqlx::{PgPool, Postgres, Row, Transaction};
use uuid::Uuid;

use crate::persistence::postgres::activities::store::insert_activity;
use crate::persistence::postgres::activities::store::invalid_activity;
use crate::persistence::postgres::identity::users::repository::lock_identity_mutations;

const PROVIDER_PROJECTION: &str = r#"
SELECT id, name, description, displayname, issuer, clientid, clientsecretciphertext,
       scopes, enabled, autoprovisionusers, allowemailautolink, requireemailverified,
       allowedemaildomains, requiredclaimname, requiredclaimvalues, defaultroleid,
       createdbyactorid, createdat, updatedat
FROM oidcproviders
"#;

#[derive(Clone)]
pub struct PostgresOidcStore {
    pool: PgPool,
}

impl PostgresOidcStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl OidcStore for PostgresOidcStore {
    fn list(&self) -> BoxFuture<'_, Result<Vec<OidcProvider>, IdentityError>> {
        Box::pin(async move {
            let query = format!("{PROVIDER_PROJECTION} ORDER BY displayname, id");
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_provider)
                .collect()
        })
    }

    fn list_enabled(&self) -> BoxFuture<'_, Result<Vec<OidcProvider>, IdentityError>> {
        Box::pin(async move {
            let query = format!("{PROVIDER_PROJECTION} WHERE enabled ORDER BY displayname, id");
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_provider)
                .collect()
        })
    }

    fn get(&self, id: Uuid) -> BoxFuture<'_, Result<Option<OidcProvider>, IdentityError>> {
        Box::pin(async move {
            let query = format!("{PROVIDER_PROJECTION} WHERE id = $1");
            sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_provider)
                .transpose()
        })
    }

    fn role_exists(&self, id: Uuid) -> BoxFuture<'_, Result<bool, IdentityError>> {
        Box::pin(async move {
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1)")
                .bind(id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
        })
    }

    fn create<'a>(
        &'a self,
        provider: &'a OidcProvider,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            insert_provider(&mut transaction, provider).await?;
            insert_provider_activity(
                &mut transaction,
                provider,
                actor_id,
                ActivityEventInfo::oidc_provider_created(provider.activity_snapshot()),
                now,
            )
            .await?;
            transaction.commit().await.map_err(storage)?;
            Ok(provider.clone())
        })
    }

    fn update<'a>(
        &'a self,
        provider: &'a OidcProvider,
        expected_updated_at: DateTime<Utc>,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = load_provider(&mut transaction, provider.id(), true)
                .await?
                .ok_or(IdentityError::NotFound)?;
            if current.updated_at() != expected_updated_at {
                return Err(IdentityError::Conflict(
                    "OIDC provider was changed by another request.".to_owned(),
                ));
            }
            update_provider(&mut transaction, provider).await?;
            if current.activity_snapshot() != provider.activity_snapshot() {
                insert_provider_activity(
                    &mut transaction,
                    provider,
                    actor_id,
                    ActivityEventInfo::oidc_provider_updated(
                        current.activity_snapshot(),
                        provider.activity_snapshot(),
                    ),
                    now,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(provider.clone())
        })
    }

    fn rename<'a>(
        &'a self,
        id: Uuid,
        name: &'a str,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = load_provider(&mut transaction, id, true)
                .await?
                .ok_or(IdentityError::NotFound)?;
            if current.name() != name {
                sqlx::query("UPDATE oidcproviders SET name = $2, updatedat = $3 WHERE id = $1")
                    .bind(id)
                    .bind(name)
                    .bind(now)
                    .execute(&mut *transaction)
                    .await
                    .map_err(conflict_or_storage)?;
                let renamed = load_provider(&mut transaction, id, false)
                    .await?
                    .ok_or(IdentityError::NotFound)?;
                insert_provider_activity(
                    &mut transaction,
                    &renamed,
                    actor_id,
                    ActivityEventInfo::oidc_provider_renamed(
                        current.name().to_owned(),
                        name.to_owned(),
                    ),
                    now,
                )
                .await?;
                transaction.commit().await.map_err(storage)?;
                return Ok(renamed);
            }
            transaction.commit().await.map_err(storage)?;
            Ok(current)
        })
    }

    fn update_description<'a>(
        &'a self,
        id: Uuid,
        description: Option<&'a str>,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<OidcProvider, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = load_provider(&mut transaction, id, true)
                .await?
                .ok_or(IdentityError::NotFound)?;
            let description = description.map(str::trim).filter(|value| !value.is_empty());
            if current.description() != description {
                sqlx::query(
                    "UPDATE oidcproviders SET description = $2, updatedat = $3 WHERE id = $1",
                )
                .bind(id)
                .bind(description)
                .bind(now)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            }
            let updated = load_provider(&mut transaction, id, false)
                .await?
                .ok_or(IdentityError::NotFound)?;
            if current.activity_snapshot() != updated.activity_snapshot() {
                insert_provider_activity(
                    &mut transaction,
                    &updated,
                    actor_id,
                    ActivityEventInfo::oidc_provider_updated(
                        current.activity_snapshot(),
                        updated.activity_snapshot(),
                    ),
                    now,
                )
                .await?;
            }
            transaction.commit().await.map_err(storage)?;
            Ok(updated)
        })
    }

    fn delete(
        &self,
        id: Uuid,
        actor_id: ActorId,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let current = load_provider(&mut transaction, id, true)
                .await?
                .ok_or(IdentityError::NotFound)?;
            sqlx::query("DELETE FROM oidcproviders WHERE id = $1")
                .bind(id)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            insert_provider_activity(
                &mut transaction,
                &current,
                actor_id,
                ActivityEventInfo::oidc_provider_deleted(current.activity_snapshot()),
                now,
            )
            .await?;
            transaction.commit().await.map_err(storage)
        })
    }

    fn start_login(
        &self,
        state: OidcLoginState,
        now: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("DELETE FROM oidcloginstates WHERE expiresat <= $1")
                .bind(now)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query(
                r#"
INSERT INTO oidcloginstates
    (id, providerid, statehash, nonce, codeverifier, returnurl, createdat, expiresat)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
"#,
            )
            .bind(state.id)
            .bind(state.provider_id)
            .bind(&state.state_hash)
            .bind(&state.nonce)
            .bind(&state.code_verifier)
            .bind(&state.return_url)
            .bind(state.created_at)
            .bind(state.expires_at)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            transaction.commit().await.map_err(storage)
        })
    }

    fn consume_login_state<'a>(
        &'a self,
        state_hash: &'a str,
    ) -> BoxFuture<'a, Result<Option<OidcLoginState>, IdentityError>> {
        Box::pin(async move {
            sqlx::query(
                r#"
DELETE FROM oidcloginstates
WHERE statehash = $1
RETURNING id, providerid, statehash, nonce, codeverifier, returnurl, createdat, expiresat
"#,
            )
            .bind(state_hash)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .map(map_login_state)
            .transpose()
        })
    }

    fn resolve_identity<'a>(
        &'a self,
        provider: &'a OidcProvider,
        identity: &'a OidcIdentity,
        proposed_name: &'a str,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<UserAuthentication, IdentityError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut transaction).await?;
            if let Some((external_login_id, user_id)) = sqlx::query_as::<_, (Uuid, Uuid)>(
                "SELECT id, userid FROM oidcexternallogins WHERE providerid = $1 AND subject = $2 FOR UPDATE",
            )
            .bind(provider.id())
            .bind(&identity.subject)
            .fetch_optional(&mut *transaction)
            .await
            .map_err(storage)?
            {
                let user = load_user_authentication(&mut transaction, user_id)
                    .await?
                    .filter(|user| user.enabled)
                    .ok_or(IdentityError::InvalidCredentials)?;
                sqlx::query(
                    "UPDATE oidcexternallogins SET email = $2, updatedat = $3 WHERE id = $1",
                )
                .bind(external_login_id)
                .bind(&identity.email)
                .bind(now)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
                transaction.commit().await.map_err(storage)?;
                return Ok(user);
            }

            let linked_user_id = if provider.allow_email_auto_link() && identity.email_verified {
                match identity.email.as_deref() {
                    Some(email) => find_user_by_email(&mut transaction, email).await?,
                    None => None,
                }
            } else {
                None
            };
            let user_id = match linked_user_id {
                Some(user_id) => user_id,
                None if !provider.auto_provision_users() => {
                    return Err(IdentityError::NotFound);
                }
                None => {
                    let email = identity.email.as_deref().ok_or_else(|| {
                        IdentityError::Validation(
                            "OIDC user provisioning requires an email claim.".to_owned(),
                        )
                    })?;
                    if find_user_by_email(&mut transaction, email).await?.is_some() {
                        return Err(IdentityError::Conflict(
                            "A Citadel User with this email already exists and email auto-link is disabled."
                                .to_owned(),
                        ));
                    }
                    provision_user(&mut transaction, provider, proposed_name, email, now).await?
                }
            };
            let user = load_user_authentication(&mut transaction, user_id)
                .await?
                .filter(|user| user.enabled)
                .ok_or(IdentityError::InvalidCredentials)?;
            sqlx::query(
                r#"
INSERT INTO oidcexternallogins
    (id, providerid, subject, userid, email, createdat, updatedat)
VALUES ($1, $2, $3, $4, $5, $6, $6)
"#,
            )
            .bind(Uuid::now_v7())
            .bind(provider.id())
            .bind(&identity.subject)
            .bind(user_id)
            .bind(&identity.email)
            .bind(now)
            .execute(&mut *transaction)
            .await
            .map_err(conflict_or_storage)?;
            transaction.commit().await.map_err(storage)?;
            Ok(user)
        })
    }
}

async fn insert_provider(
    transaction: &mut Transaction<'_, Postgres>,
    provider: &OidcProvider,
) -> Result<(), IdentityError> {
    sqlx::query(
        r#"
INSERT INTO oidcproviders (
    id, name, description, displayname, issuer, clientid, clientsecretciphertext,
    scopes, enabled, autoprovisionusers, allowemailautolink, requireemailverified,
    allowedemaildomains, requiredclaimname, requiredclaimvalues, defaultroleid,
    createdbyactorid, createdat, updatedat)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19)
"#,
    )
    .bind(provider.id())
    .bind(provider.name())
    .bind(provider.description())
    .bind(provider.display_name())
    .bind(provider.issuer())
    .bind(provider.client_id())
    .bind(provider.client_secret_ciphertext())
    .bind(provider.scopes())
    .bind(provider.enabled())
    .bind(provider.auto_provision_users())
    .bind(provider.allow_email_auto_link())
    .bind(provider.require_email_verified())
    .bind(provider.allowed_email_domains())
    .bind(provider.required_claim_name())
    .bind(provider.required_claim_values())
    .bind(provider.default_role_id())
    .bind(provider.created_by_actor_id().value())
    .bind(provider.created_at())
    .bind(provider.updated_at())
    .execute(&mut **transaction)
    .await
    .map_err(conflict_or_storage)?;
    Ok(())
}

async fn update_provider(
    transaction: &mut Transaction<'_, Postgres>,
    provider: &OidcProvider,
) -> Result<(), IdentityError> {
    sqlx::query(
        r#"
UPDATE oidcproviders SET
    name = $2, description = $3, displayname = $4, issuer = $5, clientid = $6,
    clientsecretciphertext = $7, scopes = $8, enabled = $9, autoprovisionusers = $10,
    allowemailautolink = $11, requireemailverified = $12, allowedemaildomains = $13,
    requiredclaimname = $14, requiredclaimvalues = $15, defaultroleid = $16, updatedat = $17
WHERE id = $1
"#,
    )
    .bind(provider.id())
    .bind(provider.name())
    .bind(provider.description())
    .bind(provider.display_name())
    .bind(provider.issuer())
    .bind(provider.client_id())
    .bind(provider.client_secret_ciphertext())
    .bind(provider.scopes())
    .bind(provider.enabled())
    .bind(provider.auto_provision_users())
    .bind(provider.allow_email_auto_link())
    .bind(provider.require_email_verified())
    .bind(provider.allowed_email_domains())
    .bind(provider.required_claim_name())
    .bind(provider.required_claim_values())
    .bind(provider.default_role_id())
    .bind(provider.updated_at())
    .execute(&mut **transaction)
    .await
    .map_err(conflict_or_storage)?;
    Ok(())
}

async fn load_provider(
    transaction: &mut Transaction<'_, Postgres>,
    id: Uuid,
    lock: bool,
) -> Result<Option<OidcProvider>, IdentityError> {
    let suffix = if lock { " FOR UPDATE" } else { "" };
    let query = format!("{PROVIDER_PROJECTION} WHERE id = $1{suffix}");
    sqlx::query(sqlx::AssertSqlSafe(query.as_str()))
        .bind(id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?
        .map(map_provider)
        .transpose()
}

fn map_provider(row: sqlx::postgres::PgRow) -> Result<OidcProvider, IdentityError> {
    OidcProvider::from_persistence(
        row.try_get("id").map_err(storage)?,
        row.try_get("name").map_err(storage)?,
        row.try_get("description").map_err(storage)?,
        row.try_get("displayname").map_err(storage)?,
        row.try_get("issuer").map_err(storage)?,
        row.try_get("clientid").map_err(storage)?,
        row.try_get("clientsecretciphertext").map_err(storage)?,
        row.try_get("scopes").map_err(storage)?,
        row.try_get("enabled").map_err(storage)?,
        row.try_get("autoprovisionusers").map_err(storage)?,
        row.try_get("allowemailautolink").map_err(storage)?,
        row.try_get("requireemailverified").map_err(storage)?,
        row.try_get("allowedemaildomains").map_err(storage)?,
        row.try_get("requiredclaimname").map_err(storage)?,
        row.try_get("requiredclaimvalues").map_err(storage)?,
        row.try_get("defaultroleid").map_err(storage)?,
        ActorId::new(row.try_get("createdbyactorid").map_err(storage)?),
        row.try_get("createdat").map_err(storage)?,
        row.try_get("updatedat").map_err(storage)?,
    )
    .map_err(|error| IdentityError::Storage(error.to_string()))
}

fn map_login_state(row: sqlx::postgres::PgRow) -> Result<OidcLoginState, IdentityError> {
    Ok(OidcLoginState {
        id: row.try_get("id").map_err(storage)?,
        provider_id: row.try_get("providerid").map_err(storage)?,
        state_hash: row.try_get("statehash").map_err(storage)?,
        nonce: row.try_get("nonce").map_err(storage)?,
        code_verifier: row.try_get("codeverifier").map_err(storage)?,
        return_url: row.try_get("returnurl").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        expires_at: row.try_get("expiresat").map_err(storage)?,
    })
}

async fn provision_user(
    transaction: &mut Transaction<'_, Postgres>,
    provider: &OidcProvider,
    proposed_name: &str,
    email: &str,
    now: DateTime<Utc>,
) -> Result<Uuid, IdentityError> {
    let name = available_user_name(transaction, proposed_name).await?;
    let actor_id = Uuid::now_v7();
    let user_id = Uuid::now_v7();
    sqlx::query("INSERT INTO actors (id, isenabled, type) VALUES ($1, TRUE, 'User')")
        .bind(actor_id)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?;
    sqlx::query(
        r#"
INSERT INTO users (id, actorid, createdat, createdbyactorid, email, name, password)
VALUES ($1, $2, $3, $4, $5, $6, NULL)
"#,
    )
    .bind(user_id)
    .bind(actor_id)
    .bind(now)
    .bind(SYSTEM_ACTOR_ID)
    .bind(email.to_ascii_lowercase())
    .bind(&name)
    .execute(&mut **transaction)
    .await
    .map_err(conflict_or_storage)?;
    let mut role_ids = Vec::new();
    if let Some(role_id) = provider.default_role_id() {
        let inserted = sqlx::query(
            "INSERT INTO actorroles (actorid, roleid) SELECT $1, id FROM roles WHERE id = $2",
        )
        .bind(actor_id)
        .bind(role_id)
        .execute(&mut **transaction)
        .await
        .map_err(storage)?
        .rows_affected();
        if inserted != 1 {
            return Err(IdentityError::Validation(
                "Default role does not exist.".to_owned(),
            ));
        }
        role_ids.push(role_id);
    }
    let activity = ActivityEvent::new_user_event(
        user_id,
        name,
        ActorId::new(SYSTEM_ACTOR_ID),
        ActivityEventInfo::user_created(UserActivitySnapshot {
            email: email.to_ascii_lowercase(),
            is_enabled: true,
            team_ids: Vec::new(),
            role_ids,
            resource_accesses: Vec::<IdentityResourceAccessSnapshot>::new(),
        }),
        now,
    )
    .map_err(invalid_activity)?;
    insert_activity(transaction, &activity).await?;
    Ok(user_id)
}

async fn available_user_name(
    transaction: &mut Transaction<'_, Postgres>,
    base: &str,
) -> Result<String, IdentityError> {
    for suffix in 0_u32..10_000 {
        let candidate = if suffix == 0 {
            base.to_owned()
        } else {
            format!("{base}-{suffix}")
        };
        let candidate = candidate.chars().take(64).collect::<String>();
        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM users WHERE lower(name) = lower($1))",
        )
        .bind(&candidate)
        .fetch_one(&mut **transaction)
        .await
        .map_err(storage)?;
        if !exists {
            return Ok(candidate);
        }
    }
    Err(IdentityError::Conflict(
        "Could not allocate a unique OIDC user name.".to_owned(),
    ))
}

async fn find_user_by_email(
    transaction: &mut Transaction<'_, Postgres>,
    email: &str,
) -> Result<Option<Uuid>, IdentityError> {
    let matches = sqlx::query_scalar("SELECT id FROM users WHERE lower(email) = lower($1) LIMIT 2")
        .bind(email)
        .fetch_all(&mut **transaction)
        .await
        .map_err(storage)?;

    match matches.as_slice() {
        [] => Ok(None),
        [id] => Ok(Some(*id)),
        _ => Err(IdentityError::Conflict(
            "The verified OIDC email matches more than one Citadel user.".to_owned(),
        )),
    }
}

async fn load_user_authentication(
    transaction: &mut Transaction<'_, Postgres>,
    user_id: Uuid,
) -> Result<Option<UserAuthentication>, IdentityError> {
    sqlx::query(
        r#"
SELECT u.id, u.actorid, u.name, u.email, u.password, actor.isenabled,
       COALESCE(array_agg(DISTINCT role.name) FILTER (WHERE role.name IS NOT NULL), ARRAY[]::text[]) AS roles
FROM users u
JOIN actors actor ON actor.id = u.actorid AND actor.type = 'User'
LEFT JOIN (
    SELECT actorid, roleid FROM actorroles
    UNION
    SELECT membership.memberactorid, actor_role.roleid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    JOIN actorroles actor_role ON actor_role.actorid = team.actorid
) effective_role ON effective_role.actorid = u.actorid
LEFT JOIN roles role ON role.id = effective_role.roleid
WHERE u.id = $1
GROUP BY u.id, u.actorid, u.name, u.email, u.password, actor.isenabled
"#,
    )
    .bind(user_id)
    .fetch_optional(&mut **transaction)
    .await
    .map_err(storage)?
    .map(|row| {
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
    })
    .transpose()
}

async fn insert_provider_activity(
    transaction: &mut Transaction<'_, Postgres>,
    provider: &OidcProvider,
    actor_id: ActorId,
    info: ActivityEventInfo,
    now: DateTime<Utc>,
) -> Result<(), IdentityError> {
    let activity = ActivityEvent::new_oidc_provider_event(
        provider.id(),
        provider.name().to_owned(),
        actor_id,
        info,
        now,
    )
    .map_err(invalid_activity)?;
    insert_activity(transaction, &activity).await
}

fn conflict_or_storage(error: sqlx::Error) -> IdentityError {
    if error
        .as_database_error()
        .is_some_and(|error| error.is_unique_violation())
    {
        IdentityError::Conflict("OIDC provider name or identity already exists.".to_owned())
    } else {
        storage(error)
    }
}

fn storage(error: impl std::fmt::Display) -> IdentityError {
    IdentityError::Storage(error.to_string())
}
