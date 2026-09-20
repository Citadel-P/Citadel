use chrono::Utc;
use citadel_activities::{ActivityEvent, ActivityEventInfo, PlatformActivitySnapshot};
use citadel_platforms::deletion::{PlatformDeletionError, PlatformDeletionRepository};
use citadel_primitives::{ActorId, ResourceType};
use futures_util::{FutureExt, future::BoxFuture};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::activity_store::insert_activity;

pub struct PostgresPlatformDeletionRepository {
    pool: PgPool,
}

impl PostgresPlatformDeletionRepository {
    pub const fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PlatformDeletionRepository for PostgresPlatformDeletionRepository {
    fn delete<'a>(
        &'a self,
        actor: ActorId,
        ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<(), PlatformDeletionError>> {
        async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            // Serialize with registration and Edge enrollment; row locks also protect
            // against concurrent workload FK inserts while validating the whole batch.
            sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
                .execute(&mut *tx).await.map_err(storage)?;
            // Match Edge inventory's binding-before-platform lock order. All
            // revocations roll back too if any selected Platform cannot be deleted.
            sqlx::query("UPDATE edgeagentenrollments SET revokedatutc = CURRENT_TIMESTAMP WHERE platformid = ANY($1) AND revokedatutc IS NULL")
                .bind(ids).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("UPDATE edgeagentbindings SET revokedatutc = CURRENT_TIMESTAMP, revocationreason = 'Platform deleted', connectionstatus = 'Revoked', updatedatutc = CURRENT_TIMESTAMP WHERE platformid = ANY($1) AND revokedatutc IS NULL")
                .bind(ids).execute(&mut *tx).await.map_err(storage)?;
            let rows = sqlx::query("SELECT id, name, address, description, status, connectortype, networkcount, volumecount, imagecount::bigint AS imagecount, cpucount::bigint AS cpucount, memtotal, serverversion, agentversion, platformdescriptor FROM platforms WHERE id = ANY($1) ORDER BY id FOR UPDATE")
                .bind(ids).fetch_all(&mut *tx).await.map_err(storage)?;
            if rows.len() != ids.len() {
                return Err(PlatformDeletionError::NotFound);
            }
            for row in rows {
                let platform = PlatformActivitySnapshot {
                    id: row.try_get("id").map_err(storage)?,
                    name: row.try_get("name").map_err(storage)?,
                    address: row.try_get("address").map_err(storage)?,
                    description: row.try_get("description").map_err(storage)?,
                    status: row.try_get("status").map_err(storage)?,
                    connector_type: row.try_get("connectortype").map_err(storage)?,
                    network_count: row.try_get("networkcount").map_err(storage)?,
                    volume_count: row.try_get("volumecount").map_err(storage)?,
                    image_count: row.try_get("imagecount").map_err(storage)?,
                    cpu_count: row.try_get("cpucount").map_err(storage)?,
                    mem_total: row.try_get("memtotal").map_err(storage)?,
                    server_version: row.try_get("serverversion").map_err(storage)?,
                    agent_version: row.try_get("agentversion").map_err(storage)?,
                    platform_descriptor: row.try_get("platformdescriptor").map_err(storage)?,
                };
                let event = ActivityEvent::new_platform_event(platform.id, platform.name.clone(), actor,
                    ActivityEventInfo::PlatformDeleted { platform }, Utc::now())
                    .map_err(|error| PlatformDeletionError::Storage(error.to_string()))?;
                insert_activity(&mut tx, &event).await
                    .map_err(|error| PlatformDeletionError::Storage(error.to_string()))?;
            }
            sqlx::query("DELETE FROM resourcetags WHERE resourcetype = 'Platform' AND resourceid = ANY($1)")
                .bind(ids).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("DELETE FROM resourceaccesses WHERE resourcetype = $1 AND resourceid = ANY($2)")
                .bind(ResourceType::Platform as i32).bind(ids).execute(&mut *tx).await.map_err(storage)?;
            // Restrictive FKs are the authoritative, race-safe dependency check.
            // Inventory/backup items cascade; activity platform links become null.
            sqlx::query("DELETE FROM platforms WHERE id = ANY($1)")
                .bind(ids).execute(&mut *tx).await.map_err(storage)?;
            sqlx::query("SELECT pg_notify('citadel_platform_targets','')").execute(&mut *tx).await.map_err(storage)?;
            tx.commit().await.map_err(storage)
        }.boxed()
    }
}

fn storage(error: sqlx::Error) -> PlatformDeletionError {
    if error.as_database_error().is_some_and(|error| {
        // PostgreSQL reports ON DELETE RESTRICT as restrict_violation;
        // SQLx's foreign-key helper only covers foreign_key_violation.
        error.is_foreign_key_violation() || error.code().as_deref() == Some("23001")
    }) {
        PlatformDeletionError::InUse
    } else {
        PlatformDeletionError::Storage(error.to_string())
    }
}
