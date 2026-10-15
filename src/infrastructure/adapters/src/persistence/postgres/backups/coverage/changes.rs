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
pub struct PostgresCoverageChanges {
    listener: sqlx::postgres::PgListener,
    pool: PgPool,
}
impl PostgresCoverageChanges {
    pub async fn connect(pool: &PgPool) -> Result<Self, sqlx::Error> {
        let dedicated = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_lazy_with((*pool.connect_options()).clone().application_name(CHANNEL));
        let mut listener = sqlx::postgres::PgListener::connect_with(&dedicated).await?;
        listener.listen(CHANNEL).await?;
        Ok(Self {
            listener,
            pool: pool.clone(),
        })
    }
}
impl CoverageChangeSource for PostgresCoverageChanges {
    fn next(&mut self) -> BoxFuture<'_, Result<Option<CoverageInvalidation>, BackupError>> {
        Box::pin(async move {
            let Some(event) = self.listener.try_recv().await.map_err(storage)? else {
                return Ok(None);
            };
            let platform: Uuid = event.payload().parse().map_err(storage)?;
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
            Ok(Some(change))
        })
    }
}
