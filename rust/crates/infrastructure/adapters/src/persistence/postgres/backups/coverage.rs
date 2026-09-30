//! Batched volume coverage with the same policy visibility rules as the Backup UI.
use super::*;

pub async fn volume_coverage(
    pool: &PgPool,
    actor: ActorId,
    administrator: bool,
    platform_id: Uuid,
    volumes: &[(String, Option<String>)],
) -> Result<Vec<summaries::VolumeBackupCoverage>, BackupError> {
    if volumes.is_empty() {
        return Ok(Vec::new());
    }
    let names = volumes.iter().map(|v| v.0.as_str()).collect::<Vec<_>>();
    let nodes = volumes.iter().map(|v| v.1.as_deref()).collect::<Vec<_>>();
    let query = format!(
        r#"{AUTHORIZED_CTE}, requested AS (
        SELECT DISTINCT $5::uuid AS platformid, volume, node
        FROM unnest($6::text[], $7::text[]) AS requested(volume, node)
    ), policies AS (
        SELECT r.*, p.id, p.enabled, p.backuprepositoryid
        FROM requested r JOIN backuppolicies p ON p.archivedat IS NULL
        AND p.source->>'$type'='DockerVolume'
        AND COALESCE(p.source->>'platformId',p.source->>'PlatformId')=r.platformid::text
        AND COALESCE(p.source->>'volumeName',p.source->>'VolumeName')=r.volume
        AND COALESCE(p.source->>'dockerNodeId',p.source->>'DockerNodeId') IS NOT DISTINCT FROM r.node
        WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
            SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
            WHERE access.resourcetype=$2 AND access.resourceid=p.id AND access.permissionlevel=ANY($3)
        ))
    )
    SELECT r.platformid, r.volume, r.node, summary.count,
        CASE WHEN summary.count=0 THEN 'Unprotected'
             WHEN latest.status='Failed' THEN 'Failed'
             WHEN summary.enabled=0 OR NOT summary.ready THEN 'Warning'
             WHEN latest.status IN ('TimedOut','Cancelled','Interrupted','SucceededWithWarnings') THEN 'Warning'
             WHEN success.at IS NOT NULL THEN 'Protected' ELSE 'Warning' END AS status,
        latest.id AS lastrunid, latest.status AS lastrunstatus, latest.at AS lastrunat,
        success.at AS lastsuccessfulrunat
    FROM requested r
    CROSS JOIN LATERAL (
        SELECT count(*)::integer AS count, count(*) FILTER (WHERE p.enabled)::integer AS enabled,
            COALESCE(bool_or(EXISTS (SELECT 1 FROM backuprepositoryvalidations v
              WHERE v.backuprepositoryid=p.backuprepositoryid AND v.location='Core'
              AND v.platformid IS NULL AND v.status='Ready')),false) AS ready
        FROM policies p WHERE p.platformid=r.platformid AND p.volume=r.volume AND p.node IS NOT DISTINCT FROM r.node
    ) summary
    LEFT JOIN LATERAL (
        SELECT b.id,b.status,COALESCE(b.completedat,b.queuedat) AS at FROM policies p
        JOIN backupruns b ON b.backuppolicyid=p.id
        WHERE p.platformid=r.platformid AND p.volume=r.volume AND p.node IS NOT DISTINCT FROM r.node
        ORDER BY b.queuedat DESC,b.id DESC LIMIT 1
    ) latest ON true
    LEFT JOIN LATERAL (
        SELECT COALESCE(b.completedat,b.queuedat) AS at FROM policies p
        JOIN backupruns b ON b.backuppolicyid=p.id
        WHERE p.platformid=r.platformid AND p.volume=r.volume AND p.node IS NOT DISTINCT FROM r.node
          AND b.status IN ('Succeeded','SucceededWithWarnings') AND b.snapshotavailability='Available'
        ORDER BY COALESCE(b.completedat,b.queuedat) DESC,b.id DESC LIMIT 1
    ) success ON true"#
    );
    sqlx::query(AssertSqlSafe(query.as_str()))
        .bind(actor.value())
        .bind(ResourceType::BackupPolicy as i32)
        .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
        .bind(administrator)
        .bind(platform_id)
        .bind(names)
        .bind(nodes)
        .fetch_all(pool)
        .await
        .map_err(storage)?
        .into_iter()
        .map(|r| {
            Ok(summaries::VolumeBackupCoverage {
                platform_id: r.try_get("platformid").map_err(storage)?,
                volume_name: r.try_get("volume").map_err(storage)?,
                docker_node_id: r.try_get("node").map_err(storage)?,
                status: r.try_get("status").map_err(storage)?,
                policy_count: r.try_get("count").map_err(storage)?,
                last_run_id: r.try_get("lastrunid").map_err(storage)?,
                last_run_status: r.try_get("lastrunstatus").map_err(storage)?,
                last_run_at: r.try_get("lastrunat").map_err(storage)?,
                last_successful_run_at: r.try_get("lastsuccessfulrunat").map_err(storage)?,
            })
        })
        .collect()
}
