use super::{repository::PostgresInstanceRepository, repository::storage};
use crate::persistence::postgres::{
    activities::store::{insert_activity, invalid_activity},
    identity::users::repository::lock_identity_mutations,
};
use chrono::{DateTime, Utc};
use citadel_activities::{ActivityEvent, ActivityEventInfo, ActivityStatus};
use citadel_identity::ActorPrincipal;
use citadel_instance::{InstanceError, InstanceRestartRepository};
use futures_util::future::BoxFuture;
use uuid::Uuid;

impl InstanceRestartRepository for PostgresInstanceRepository {
    fn request<'a>(
        &'a self,
        principal: &'a ActorPrincipal,
        boot_id: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(), InstanceError>> {
        Box::pin(async move {
            let _fence = self.authorization.read_fence().await;
            let mut tx = self.pool.begin().await.map_err(storage)?;
            super::authorization::authorize(&mut tx, principal).await?;
            let initialized: bool = sqlx::query_scalar(
                "SELECT initializedat IS NOT NULL FROM instancesetupstates WHERE id=1",
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?;
            if !initialized {
                return Err(InstanceError::Conflict(
                    "Complete Citadel setup first.".into(),
                ));
            }
            if sqlx::query_scalar::<_, bool>(
                "SELECT EXISTS(SELECT 1 FROM instancerestarts WHERE id=1)",
            )
            .fetch_one(&mut *tx)
            .await
            .map_err(storage)?
            {
                return Ok(());
            }
            if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM instanceupdates WHERE ownsinstallation) OR EXISTS(SELECT 1 FROM agentupdateoperations WHERE NOT finished) OR EXISTS(SELECT 1 FROM updatescheduleruns WHERE NOT finished)").fetch_one(&mut *tx).await.map_err(storage)? {
                return Err(InstanceError::Conflict("Wait for the active update or recovery to finish before restarting Core.".into()));
            }
            let instance = crate::persistence::postgres::licensing::store::get_or_create_identity(
                &mut tx, now,
            )
            .await
            .map_err(storage)?;
            let event = ActivityEvent::new_instance_event(
                instance.instance_id,
                principal.actor_id,
                ActivityEventInfo::InstanceRestarted {
                    phase: "Requested".into(),
                    completed_at: None,
                },
                ActivityStatus::Information,
                now,
            )
            .map_err(|error| storage(invalid_activity(error)))?;
            insert_activity(&mut tx, &event).await.map_err(storage)?;
            sqlx::query("INSERT INTO instancerestarts(id,bootid,activityid) VALUES(1,$1,$2)")
                .bind(boot_id)
                .bind(event.id())
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            tx.commit().await.map_err(storage)
        })
    }
    fn pending(&self, boot_id: Uuid) -> BoxFuture<'_, Result<bool, InstanceError>> {
        Box::pin(async move {
            sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM instancerestarts WHERE id=1 AND bootid=$1)",
            )
            .bind(boot_id)
            .fetch_one(&self.pool)
            .await
            .map_err(storage)
        })
    }
    fn complete(&self, boot_id: Uuid) -> BoxFuture<'_, Result<(), InstanceError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            lock_identity_mutations(&mut tx).await.map_err(storage)?;
            let activity: Option<Uuid> = sqlx::query_scalar(
                "DELETE FROM instancerestarts WHERE id=1 AND bootid<>$1 RETURNING activityid",
            )
            .bind(boot_id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
            if let Some(id) = activity {
                sqlx::query("UPDATE activityevents SET status='Success', info=(info::jsonb || jsonb_build_object('Phase','Completed','CompletedAt',CURRENT_TIMESTAMP))::text WHERE id=$1 AND eventtype='InstanceRestarted'").bind(id).execute(&mut *tx).await.map_err(storage)?;
            }
            tx.commit().await.map_err(storage)
        })
    }
}
