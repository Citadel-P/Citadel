use super::AuthorizationCache;
use sqlx::{PgPool, Postgres, Transaction};
use std::{collections::BTreeSet, sync::Arc};
use tokio::sync::OwnedRwLockWriteGuard;
use uuid::Uuid;

/// Resolve both before and after the mutation: removed members and cascading
/// role/team deletions must be captured while their old relationships exist.
pub(crate) enum Impact {
    Actors(Vec<Uuid>),
    Users(Vec<Uuid>),
    Teams(Vec<Uuid>),
    Roles(Vec<Uuid>),
    ServiceAccounts(Vec<Uuid>),
    Resources(i32, Vec<Uuid>),
}
impl Impact {
    async fn resolve(
        &self,
        tx: &mut Transaction<'_, Postgres>,
    ) -> Result<BTreeSet<Uuid>, sqlx::Error> {
        let (sql, ids) = match self {
            Self::Actors(ids) => ("SELECT unnest($1::uuid[])", ids),
            Self::Users(ids) => ("SELECT actorid FROM users WHERE id=ANY($1)", ids),
            Self::Teams(ids) => ("SELECT actorid FROM teams WHERE id=ANY($1)", ids),
            Self::Roles(ids) => ("SELECT actorid FROM actorroles WHERE roleid=ANY($1)", ids),
            Self::ServiceAccounts(ids) => {
                ("SELECT actorid FROM serviceaccounts WHERE id=ANY($1)", ids)
            }
            Self::Resources(kind, ids) => {
                let actors: Vec<Uuid> = sqlx::query_scalar("SELECT DISTINCT actorid FROM resourceaccesses WHERE resourcetype=$1 AND resourceid=ANY($2)")
                    .bind(kind).bind(ids).fetch_all(&mut **tx).await?;
                return expand(tx, actors).await;
            }
        };
        let actors = sqlx::query_scalar(sqlx::AssertSqlSafe(sql))
            .bind(ids)
            .fetch_all(&mut **tx)
            .await?;
        expand(tx, actors).await
    }
}
async fn expand(
    tx: &mut Transaction<'_, Postgres>,
    actors: Vec<Uuid>,
) -> Result<BTreeSet<Uuid>, sqlx::Error> {
    let members: Vec<Uuid> = sqlx::query_scalar("SELECT membership.memberactorid FROM actorteammemberships membership JOIN teams team ON team.id=membership.teamid WHERE team.actorid=ANY($1)")
        .bind(&actors).fetch_all(&mut **tx).await?;
    Ok(actors.into_iter().chain(members).collect())
}
pub(crate) struct Mutation {
    cache: Arc<AuthorizationCache>,
    _guard: OwnedRwLockWriteGuard<()>,
    affected: BTreeSet<Uuid>,
}
impl Mutation {
    /// Acquire before the SQL transaction (never while holding a pool connection).
    pub(crate) async fn enter(pool: &PgPool) -> Self {
        let cache = AuthorizationCache::attach(pool);
        let guard = cache.gate.clone().write_owned().await;
        Self {
            cache,
            _guard: guard,
            affected: BTreeSet::new(),
        }
    }
    pub(crate) async fn capture(
        &mut self,
        tx: &mut Transaction<'_, Postgres>,
        impact: &Impact,
    ) -> Result<(), sqlx::Error> {
        self.affected.extend(impact.resolve(tx).await?);
        Ok(())
    }
    pub(crate) async fn commit(
        mut self,
        mut tx: Transaction<'static, Postgres>,
        impact: Impact,
    ) -> Result<(), sqlx::Error> {
        self.capture(&mut tx, &impact).await?;
        // Once COMMIT starts it must finish publication even if the request is
        // cancelled. The gate remains held until commit + invalidation complete.
        tokio::spawn(async move {
            // Move the complete guard into the task; disjoint closure capture
            // must not leave the write guard in the cancellable caller future.
            let mutation = self;
            let result = tx.commit().await;
            if result.is_ok()
                || result.as_ref().is_err_and(|error| {
                    matches!(
                        error,
                        sqlx::Error::Io(_)
                            | sqlx::Error::Tls(_)
                            | sqlx::Error::Protocol(_)
                            | sqlx::Error::WorkerCrashed
                    )
                })
            {
                // A lost acknowledgement can mean COMMIT succeeded. Fail closed
                // in that uncertain case; explicit rollback never publishes.
                mutation.cache.invalidate(mutation.affected.iter().copied());
            }
            drop(mutation);
            result
        })
        .await
        .map_err(|e| sqlx::Error::Protocol(e.to_string()))?
    }
}
