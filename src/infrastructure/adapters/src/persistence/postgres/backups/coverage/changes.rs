//! Commit-fenced notifications follow the existing connection-event pattern.
use super::*;
const CHANNEL: &str = "citadel_backup_coverage";

pub(crate) async fn publish(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    policy: Option<Uuid>,
    repository: Option<Uuid>,
) -> Result<(), BackupError> {
    sqlx::query(r#"SELECT pg_notify('citadel_backup_coverage',platform::text) FROM (
        SELECT DISTINCT COALESCE(
            NULLIF(COALESCE(p.source->>'platformId',p.source->>'PlatformId'),'')::uuid,
            d.platformid,r.platformid,s.platformid) AS platform
        FROM backuppolicies p
        LEFT JOIN deployments d ON d.id::text=COALESCE(p.source->>'deploymentId',p.source->>'DeploymentId')
        LEFT JOIN stacks st ON st.id::text=COALESCE(p.source->>'stackId',p.source->>'StackId')
        LEFT JOIN stackreleases r ON r.id=st.currentstackreleaseid
        LEFT JOIN swarmservices s ON s.id::text=COALESCE(p.source->>'swarmServiceId',p.source->>'SwarmServiceId')
        WHERE p.id=$1 OR p.backuprepositoryid=$2
        UNION
        SELECT item.platformid FROM backuprunitems item
        JOIN backupruns run ON run.id=item.backuprunid
        WHERE run.backuppolicyid=$1 OR run.backuprepositoryid=$2
    ) affected WHERE platform IS NOT NULL"#).bind(policy).bind(repository).execute(&mut **tx).await.map_err(storage)?;
    Ok(())
}
use citadel_runtime::notifications::{NotificationSource, RuntimeNotification};
use std::{collections::BTreeSet, time::Duration};

pub struct PostgresCoverageChanges {
    source: Box<dyn NotificationSource>,
    pool: PgPool,
    pending: BTreeSet<Uuid>,
}
impl PostgresCoverageChanges {
    pub fn from_notifications(pool: &PgPool, source: Box<dyn NotificationSource>) -> Self {
        Self {
            source,
            pool: pool.clone(),
            pending: BTreeSet::new(),
        }
    }
    pub async fn connect(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let source =
            super::super::super::notifications::PostgresNotifications::connect(pool, CHANNEL)
                .await?;
        Ok(Self::from_notifications(pool, Box::new(source)))
    }
    async fn collect(&mut self) -> Result<bool, BackupError> {
        // Keep IDs across cancellation, and bound both the batch and its latency.
        loop {
            match self.source.recv().await.map_err(storage)? {
                RuntimeNotification::BackupCoverage(id) => {
                    self.pending.insert(id);
                    break;
                }
                RuntimeNotification::Resync => {
                    self.pending.clear();
                    return Ok(false);
                }
                _ => {}
            }
        }
        let deadline = tokio::time::Instant::now() + Duration::from_millis(50);
        while self.pending.len() < 256 {
            tokio::select! {
                _ = tokio::time::sleep_until(deadline) => break,
                event = self.source.recv() => match event.map_err(storage)? {
                    RuntimeNotification::BackupCoverage(id) => { self.pending.insert(id); }
                    RuntimeNotification::Resync => { self.pending.clear(); return Ok(false); }
                    _ => {},
                },
            }
        }
        Ok(true)
    }
}
impl CoverageChangeSource for PostgresCoverageChanges {
    fn next(&mut self) -> BoxFuture<'_, Result<Option<CoverageInvalidation>, BackupError>> {
        Box::pin(async move {
            if self.pending.is_empty() && !self.collect().await? {
                return Ok(None);
            }
            let platform = *self.pending.first().expect("collected coverage platform");
            let rows=sqlx::query("SELECT id,'Deployment' AS kind FROM deployments WHERE platformid=$1 UNION ALL SELECT s.id,'Stack' FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$1 UNION ALL SELECT id,'SwarmService' FROM swarmservices WHERE platformid=$1 UNION ALL SELECT id,'Container' FROM containers WHERE platformid=$1").bind(platform).fetch_all(&self.pool).await.map_err(storage)?;
            let mut change = CoverageInvalidation {
                platform_id: platform,
                ..Default::default()
            };
            for row in rows {
                let ids = match row.try_get::<&str, _>("kind").map_err(storage)? {
                    "Deployment" => &mut change.deployment_ids,
                    "Stack" => &mut change.stack_ids,
                    "SwarmService" => &mut change.service_ids,
                    _ => &mut change.container_ids,
                };
                ids.push(row.try_get("id").map_err(storage)?);
            }
            self.pending.remove(&platform);
            Ok(Some(change))
        })
    }
}

#[cfg(test)]
mod tests;
