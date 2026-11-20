use crate::persistence::postgres::authorization::AUTHORIZED_BY_LEVELS_CTE as AUTHORIZED_CTE;
use citadel_git::{
    GitAccount, GitAccountError, GitAccountRepository, GitAuthType, GitTransport, StoredGitAccount,
};
use citadel_primitives::{ActorId, ResourceType};
use futures_util::future::BoxFuture;
use sqlx::{AssertSqlSafe, PgPool, Row};
use uuid::Uuid;

#[derive(Clone)]
pub struct PostgresGitAccountRepository {
    pool: PgPool,
}

impl PostgresGitAccountRepository {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl GitAccountRepository for PostgresGitAccountRepository {
    fn list<'a>(
        &'a self,
        actor_id: ActorId,
        administrator: bool,
    ) -> BoxFuture<'a, Result<Vec<GitAccount>, GitAccountError>> {
        Box::pin(async move {
            let query = format!(
                r#"{AUTHORIZED_CTE}
SELECT account.id,account.name,account.domain,account.transport,account.authtype,account.createdat,account.createdbyactorid
FROM gitaccounts account
WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $2 AND access.resourceid = account.id
      AND access.permissionlevel = ANY($3)
))
ORDER BY account.createdat DESC, account.id"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(ResourceType::GitAccount as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(|row| map_summary(&row))
                .collect()
        })
    }

    fn get<'a>(&'a self, id: Uuid) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>> {
        Box::pin(async move {
            sqlx::query("SELECT * FROM gitaccounts WHERE id=$1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .ok_or(GitAccountError::NotFound)
                .and_then(map_account)
        })
    }

    fn create<'a>(
        &'a self,
        account: &'a StoredGitAccount,
    ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>> {
        Box::pin(async move {
            sqlx::query(
                r#"INSERT INTO gitaccounts
(id, name, domain, transport, authtype, createdat, createdbyactorid, configuration)
VALUES ($1,$2,$3,$4,$5,$6,$7,$8)"#,
            )
            .bind(account.id)
            .bind(&account.name)
            .bind(&account.domain)
            .bind(account.transport.as_database_str())
            .bind(account.auth_type.as_database_str())
            .bind(account.audit.created_at)
            .bind(account.audit.created_by_actor_id.value())
            .bind(&account.protected_configuration)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
            Ok(account.clone())
        })
    }

    fn update<'a>(
        &'a self,
        account: &'a StoredGitAccount,
    ) -> BoxFuture<'a, Result<StoredGitAccount, GitAccountError>> {
        Box::pin(async move {
            let result = sqlx::query(
                r#"UPDATE gitaccounts
SET name=$2, domain=$3, transport=$4, authtype=$5, configuration=$6
WHERE id=$1"#,
            )
            .bind(account.id)
            .bind(&account.name)
            .bind(&account.domain)
            .bind(account.transport.as_database_str())
            .bind(account.auth_type.as_database_str())
            .bind(&account.protected_configuration)
            .execute(&self.pool)
            .await
            .map_err(database_error)?;
            if result.rows_affected() == 0 {
                return Err(GitAccountError::NotFound);
            }
            Ok(account.clone())
        })
    }

    fn delete<'a>(&'a self, ids: &'a [Uuid]) -> BoxFuture<'a, Result<(), GitAccountError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            let existing = sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM gitaccounts WHERE id=ANY($1::uuid[]) FOR UPDATE",
            )
            .bind(ids)
            .fetch_all(&mut *transaction)
            .await
            .map_err(storage)?;
            let expected = ids
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>();
            if existing.len() != expected.len() {
                return Err(GitAccountError::NotFound);
            }
            sqlx::query("DELETE FROM gitaccounts WHERE id=ANY($1::uuid[])")
                .bind(ids)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            transaction.commit().await.map_err(storage)
        })
    }
}

fn map_account(row: sqlx::postgres::PgRow) -> Result<StoredGitAccount, GitAccountError> {
    let account = map_summary(&row)?;
    Ok(StoredGitAccount {
        id: account.id,
        audit: account.audit,
        name: account.name,
        domain: account.domain,
        transport: account.transport,
        auth_type: account.auth_type,
        protected_configuration: row.try_get("configuration").map_err(storage)?,
    })
}

fn map_summary(row: &sqlx::postgres::PgRow) -> Result<GitAccount, GitAccountError> {
    Ok(GitAccount {
        id: row.try_get("id").map_err(storage)?,
        audit: citadel_primitives::AuditMetadata {
            created_by_actor_id: citadel_primitives::ActorId::new(
                row.try_get("createdbyactorid").map_err(storage)?,
            ),
            created_at: row.try_get("createdat").map_err(storage)?,
        },
        name: row.try_get("name").map_err(storage)?,
        domain: row.try_get("domain").map_err(storage)?,
        transport: parse_transport(row.try_get("transport").map_err(storage)?)?,
        auth_type: parse_auth_type(row.try_get("authtype").map_err(storage)?)?,
    })
}

fn parse_transport(value: String) -> Result<GitTransport, GitAccountError> {
    match value.as_str() {
        "Http" => Ok(GitTransport::Http),
        "Https" => Ok(GitTransport::Https),
        "Ssh" => Ok(GitTransport::Ssh),
        _ => Err(GitAccountError::Storage(format!(
            "unknown Git transport '{value}'"
        ))),
    }
}

fn parse_auth_type(value: String) -> Result<GitAuthType, GitAccountError> {
    match value.as_str() {
        "Basic" => Ok(GitAuthType::Basic),
        "Token" => Ok(GitAuthType::Token),
        "SshKey" => Ok(GitAuthType::SshKey),
        _ => Err(GitAccountError::Storage(format!(
            "unknown Git authentication type '{value}'"
        ))),
    }
}

fn database_error(error: sqlx::Error) -> GitAccountError {
    if error
        .as_database_error()
        .and_then(sqlx::error::DatabaseError::code)
        .as_deref()
        == Some("23505")
    {
        GitAccountError::Conflict("A Git account with this name already exists.".to_owned())
    } else {
        storage(error)
    }
}

fn storage(error: sqlx::Error) -> GitAccountError {
    GitAccountError::Storage(error.to_string())
}
