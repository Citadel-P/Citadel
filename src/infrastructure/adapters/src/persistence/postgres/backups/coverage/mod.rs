//! Authorized, batched coverage inputs. No runtime calls or status decisions.
use super::*;
use citadel_backups::coverage::*;
pub mod changes;
mod inventory;
pub struct PostgresVolumeCoverageReader(pub PgPool);
impl CoverageReader for PostgresVolumeCoverageReader {
    fn snapshot<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platforms: &'a [Uuid],
    ) -> BoxFuture<'a, Result<CoverageSnapshot, BackupError>> {
        Box::pin(async move {
            if platforms.is_empty() {
                return Ok(CoverageSnapshot::default());
            }
            let mut snapshot = CoverageSnapshot::default();
            let query = format!(
                r#"{AUTHORIZED_CTE}
                SELECT p.*, (br.archivedat IS NULL AND br.status='Ready') AS ready
                FROM backuppolicies p JOIN backuprepositories br ON br.id=p.backuprepositoryid
                WHERE p.archivedat IS NULL
                AND (
                    COALESCE(p.source->>'platformId',p.source->>'PlatformId')=ANY(SELECT unnest($5::uuid[])::text)
                    OR EXISTS(SELECT 1 FROM deployments d WHERE d.platformid=ANY($5) AND d.id::text=COALESCE(p.source->>'deploymentId',p.source->>'DeploymentId'))
                    OR EXISTS(SELECT 1 FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=ANY($5) AND s.id::text=COALESCE(p.source->>'stackId',p.source->>'StackId'))
                    OR EXISTS(SELECT 1 FROM swarmservices s WHERE s.platformid=ANY($5) AND s.id::text=COALESCE(p.source->>'swarmServiceId',p.source->>'SwarmServiceId'))
                )
                AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
                    SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
                    WHERE access.resourcetype=$2 AND access.resourceid=p.id AND access.permissionlevel=ANY($3)))"#
            );
            let rows = sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor.value())
                .bind(ResourceType::BackupPolicy as i32)
                .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
                .bind(administrator)
                .bind(platforms)
                .fetch_all(&self.0)
                .await
                .map_err(storage)?;
            inventory::read(&self.0, platforms, &mut snapshot).await?;
            for r in rows {
                let source: sqlx::types::Json<spec::BackupSourceSpec> =
                    r.try_get("source").map_err(storage)?;
                let in_scope = match &source.0 {
                    spec::BackupSourceSpec::DockerVolume { platform_id, .. }
                    | spec::BackupSourceSpec::BindMount { platform_id, .. } => {
                        platforms.contains(platform_id)
                    }
                    source => source_identity(source).is_some_and(|(kind, id)| {
                        snapshot
                            .workloads
                            .iter()
                            .any(|w| w.kind == kind && w.id == id)
                    }),
                };
                if in_scope {
                    snapshot.policies.push(CoveragePolicy {
                        id: r.try_get("id").map_err(storage)?,
                        name: r.try_get("name").map_err(storage)?,
                        enabled: r.try_get("enabled").map_err(storage)?,
                        cron: r.try_get("cron").map_err(storage)?,
                        source: source.0,
                        repository_ready: r.try_get("ready").map_err(storage)?,
                        latest_run: None,
                    });
                }
            }
            let ids = snapshot.policies.iter().map(|p| p.id).collect::<Vec<_>>();
            let runs = sqlx::query("SELECT latest.* FROM unnest($1::uuid[]) policy_id CROSS JOIN LATERAL (SELECT * FROM backupruns WHERE backuppolicyid=policy_id ORDER BY queuedat DESC,id DESC LIMIT 1) latest")
                .bind(&ids).fetch_all(&self.0).await.map_err(storage)?.into_iter().map(map_run).collect::<Result<Vec<_>,_>>()?;
            let run_ids = runs.iter().map(|r| r.id).collect::<Vec<_>>();
            let mut items = sqlx::query("SELECT * FROM backuprunitems WHERE backuprunid=ANY($1)")
                .bind(&run_ids)
                .fetch_all(&self.0)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_run_item)
                .collect::<Result<Vec<_>, _>>()?;
            for mut run in runs {
                run.items = items
                    .extract_if(.., |i| i.backup_run_id == run.id)
                    .collect();
                if let Some(policy) = snapshot
                    .policies
                    .iter_mut()
                    .find(|p| p.id == run.backup_policy_id)
                {
                    policy.latest_run = Some(run);
                }
            }
            let rows=sqlx::query("SELECT i.platformid,i.volumename,i.hostpath,i.dockernodeid,MAX(COALESCE(i.completedat,r.completedat,r.queuedat)) AS at FROM backuprunitems i JOIN backupruns r ON r.id=i.backuprunid WHERE r.backuppolicyid=ANY($1) AND i.platformid=ANY($2) AND i.status='Succeeded' AND i.resticsnapshotid IS NOT NULL AND r.snapshotavailability='Available' GROUP BY i.platformid,i.volumename,i.hostpath,i.dockernodeid")
                .bind(&ids).bind(platforms).fetch_all(&self.0).await.map_err(storage)?;
            for r in rows {
                let host: Option<String> = r.try_get("hostpath").map_err(storage)?;
                snapshot.successes.push((
                    StorageTarget {
                        platform_id: r.try_get("platformid").map_err(storage)?,
                        name: host
                            .clone()
                            .unwrap_or(r.try_get("volumename").map_err(storage)?),
                        host_path: host,
                        docker_node_id: r.try_get("dockernodeid").map_err(storage)?,
                    },
                    r.try_get("at").map_err(storage)?,
                ));
            }
            Ok(snapshot)
        })
    }
}

pub async fn volume_coverage(
    pool: &PgPool,
    actor: ActorId,
    administrator: bool,
    platform: Uuid,
    volumes: &[(String, Option<String>)],
    scheduling_available: bool,
) -> Result<Vec<summaries::VolumeBackupCoverage>, BackupError> {
    let snapshot = PostgresVolumeCoverageReader(pool.clone())
        .snapshot(actor, administrator, &[platform])
        .await?;
    let targets = volumes
        .iter()
        .map(|(name, node)| ObservedStorage {
            target: StorageTarget {
                platform_id: platform,
                name: name.clone(),
                docker_node_id: node.clone(),
                host_path: None,
            },
            included: false,
        })
        .collect::<Vec<_>>();
    let summary = snapshot.evaluate(platform, &targets, None, true, scheduling_available);
    Ok(summary
        .volumes
        .into_iter()
        .map(|v| {
            let latest = snapshot
                .policies
                .iter()
                .filter(|p| v.policy_ids.contains(&p.id))
                .filter_map(|p| p.latest_run.as_ref())
                .max_by_key(|r| (r.queued_at, r.id));
            summaries::VolumeBackupCoverage {
                platform_id: platform,
                volume_name: v.target.name,
                docker_node_id: v.target.docker_node_id,
                status: v.status.unwrap_or(BackupCoverageStatus::Warning),
                policy_count: v.policy_ids.len() as i32,
                last_run_id: latest.map(|r| r.id),
                last_run_status: latest.map(|r| r.status),
                last_run_at: latest.map(|r| r.completed_at.unwrap_or(r.queued_at)),
                last_successful_run_at: v.last_successful_run_at,
            }
        })
        .collect())
}
impl summaries::VolumeCoverageReader for PostgresVolumeCoverageReader {
    fn volume_coverage<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        volumes: &'a [(String, Option<String>)],
        scheduling_available: bool,
    ) -> BoxFuture<'a, Result<Vec<summaries::VolumeBackupCoverage>, BackupError>> {
        Box::pin(volume_coverage(
            &self.0,
            actor,
            administrator,
            platform,
            volumes,
            scheduling_available,
        ))
    }
}
