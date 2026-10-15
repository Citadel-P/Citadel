//! Atomic copy reviews, snapshot pins, child restores and workload finalization.
use super::identity::authentication::store::PostgresIdentityStore;
use super::identity::authorization_cache::{AuthorizationCache, ReadFence};
use citadel_identity::IdentityStore;
use citadel_primitives::{
    ActorId, PermissionLevel, PermissionRequirement, ResourceType, SpecificPermission,
};
use citadel_resource_copies::*;
use futures_util::future::BoxFuture;
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{AssertSqlSafe, PgPool, Postgres, Row, Transaction};
use std::sync::Arc;
use uuid::Uuid;
mod access;
pub mod changes;
mod current;
mod destinations;
mod finalize;
mod operations;
mod sources;
pub mod storage;
use access::*;

pub struct PostgresCopyRepository {
    pool: PgPool,
    authorization: Arc<AuthorizationCache>,
}
impl PostgresCopyRepository {
    pub fn new(pool: PgPool) -> Self {
        Self {
            authorization: AuthorizationCache::attach(&pool),
            pool,
        }
    }
}
fn storage_error(error: impl std::fmt::Display) -> CopyError {
    CopyError::Storage(error.to_string())
}
fn conflict(message: &str) -> CopyError {
    CopyError::Conflict(message.into())
}

impl CopyRepository for PostgresCopyRepository {
    fn targets(
        &self,
        actor: ActorId,
        kind: CopyKind,
        source: Uuid,
    ) -> BoxFuture<'_, Result<CopyTargets, CopyError>> {
        Box::pin(self.list_targets(actor, kind, source))
    }
    fn prepare_destination<'a>(
        &'a self,
        actor: ActorId,
        plan: &'a mut CopyPlan,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(self.prepare_target(actor, plan))
    }

    fn current_data_allowed(
        &self,
        actor: ActorId,
        platform: Uuid,
    ) -> BoxFuture<'_, Result<bool, CopyError>> {
        Box::pin(async move {
            let access = self.access(actor).await?;
            let mut tx = self.pool.begin().await.map_err(storage_error)?;
            match self
                .require(
                    &mut tx,
                    &access,
                    ResourceType::Platform,
                    platform,
                    PermissionLevel::Read,
                    Some(SpecificPermission::Download),
                )
                .await
            {
                Ok(()) => Ok(true),
                Err(CopyError::NotFound) => Ok(false),
                Err(error) => Err(error),
            }
        })
    }
    fn recover_current(&self) -> BoxFuture<'_, Result<(), CopyError>> {
        Box::pin(self.recover_current_jobs())
    }
    fn claim_current(
        &self,
        actor: ActorId,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<(CopyPlan, CopyRestore)>, CopyError>> {
        Box::pin(self.claim_current_job(actor, id))
    }
    fn finish_current(
        &self,
        actor: ActorId,
        id: Uuid,
        volume: Uuid,
        result: Result<(), String>,
    ) -> BoxFuture<'_, Result<(), CopyError>> {
        Box::pin(self.finish_current_job(actor, id, volume, result))
    }

    fn source(
        &self,
        actor: ActorId,
        kind: CopyKind,
        id: Uuid,
    ) -> BoxFuture<'_, Result<CopySource, CopyError>> {
        Box::pin(async move {
            let access = self.access(actor).await?;
            let mut tx = self.pool.begin().await.map_err(storage_error)?;
            self.load_source(&mut tx, &access, kind, id).await
        })
    }
    fn snapshots<'a>(
        &'a self,
        actor: ActorId,
        source: &'a CopySource,
    ) -> BoxFuture<'a, Result<Vec<CopySnapshot>, CopyError>> {
        Box::pin(self.list_snapshots(actor, source))
    }
    fn save_review<'a>(
        &'a self,
        actor: ActorId,
        plan: &'a CopyPlan,
    ) -> BoxFuture<'a, Result<(), CopyError>> {
        Box::pin(self.save(actor, plan))
    }
    fn start(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        Box::pin(self.start_copy(actor, id))
    }
    fn get(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        Box::pin(self.get_copy(actor, id))
    }
    fn cleanup(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        Box::pin(self.cleanup_copy(actor, id))
    }
    fn finish_cleanup(
        &self,
        actor: ActorId,
        id: Uuid,
        result: Result<(), String>,
    ) -> BoxFuture<'_, Result<(), CopyError>> {
        Box::pin(async move {
            let (status, message) = match result {
                Ok(()) => ("Cleaned", "Destination volumes removed.".to_owned()),
                Err(error) => ("Failed", error),
            };
            let mut tx = self.pool.begin().await.map_err(storage_error)?;
            let changed = sqlx::query("UPDATE resourcecopies SET status=$3,message=$4,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND actorid=$2 AND status='Cleaning'").bind(id).bind(actor.value()).bind(status).bind(message).execute(&mut *tx).await.map_err(storage_error)?.rows_affected();
            if changed > 0 {
                changes::publish(&mut tx, id).await.map_err(storage_error)?;
            }
            tx.commit().await.map_err(storage_error)?;
            Ok(())
        })
    }
    fn cancel(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        Box::pin(self.cancel_copy(actor, id))
    }
    fn pending(&self) -> BoxFuture<'_, Result<Vec<(ActorId, Uuid)>, CopyError>> {
        Box::pin(async move {
            // Expired reviews have no child restores and can be removed in
            // bounded batches. Started operation receipts remain durable.
            sqlx::query("DELETE FROM resourcecopies WHERE id IN (SELECT id FROM resourcecopies WHERE status='Review' AND createdat<CURRENT_TIMESTAMP-INTERVAL '1 day' ORDER BY createdat LIMIT 100)").execute(&self.pool).await.map_err(storage_error)?;
            let rows = sqlx::query_as::<_,(Uuid,Uuid)>("SELECT actorid,id FROM resourcecopies WHERE status IN ('Restoring','Cancelling','Cleaning') ORDER BY updatedat,id LIMIT 32").fetch_all(&self.pool).await.map_err(storage_error)?;
            Ok(rows
                .into_iter()
                .map(|(actor, id)| (ActorId::new(actor), id))
                .collect())
        })
    }
    fn plan(&self, actor: ActorId, id: Uuid) -> BoxFuture<'_, Result<CopyPlan, CopyError>> {
        Box::pin(async move {
            let access = self.access(actor).await?;
            let mut tx = self.pool.begin().await.map_err(storage_error)?;
            let (plan, _) = operations::owned_plan(&mut tx, actor, id).await?;
            self.require(
                &mut tx,
                &access,
                ResourceType::Platform,
                plan.target_platform(),
                PermissionLevel::Write,
                None,
            )
            .await?;
            Ok(plan)
        })
    }
    fn advance(
        &self,
        actor: ActorId,
        id: Uuid,
        storage_result: Option<Result<(), String>>,
    ) -> BoxFuture<'_, Result<CopyProgress, CopyError>> {
        Box::pin(self.advance_copy(actor, id, storage_result))
    }
}
