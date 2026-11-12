//! Authorized, batched coverage inputs. No runtime calls or status decisions.
use super::super::*;
use super::inventory;
use citadel_backups::coverage::*;
pub struct PostgresVolumeCoverageReader(pub PgPool);
impl CoverageReader for PostgresVolumeCoverageReader {
    fn snapshot<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        platforms: &'a [Uuid],
        selection: Option<&'a CoverageSelection>,
    ) -> BoxFuture<'a, Result<CoverageSnapshot, BackupError>> {
        Box::pin(async move {
            if platforms.is_empty() {
                return Ok(CoverageSnapshot::default());
            }
            let mut snapshot = CoverageSnapshot::default();
            let query = format!(
                r#"{AUTHORIZED_CTE}
                SELECT p.id,p.name,p.enabled,p.cron,p.source, (br.archivedat IS NULL AND br.status='Ready') AS ready
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
            let mut inventory_selection = selection.cloned().unwrap_or_default();
            for row in &rows {
                let source: sqlx::types::Json<spec::BackupSourceSpec> =
                    row.try_get("source").map_err(storage)?;
                if let Some((kind, id)) = source_identity(&source.0) {
                    let policy = CoverageSelection::workload(kind, id);
                    inventory_selection.deployments.extend(policy.deployments);
                    inventory_selection.stacks.extend(policy.stacks);
                    inventory_selection.services.extend(policy.services);
                }
            }
            for ids in [
                &mut inventory_selection.deployments,
                &mut inventory_selection.stacks,
                &mut inventory_selection.services,
            ] {
                ids.sort_unstable();
                ids.dedup();
            }
            inventory::read(
                &self.0,
                platforms,
                selection.map(|_| &inventory_selection),
                &mut snapshot,
            )
            .await?;
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
            let targets = selected_targets(&snapshot, selection);
            let mut relevant = std::collections::BTreeSet::new();
            for platform in platforms {
                let local: Vec<_> = targets
                    .iter()
                    .filter(|t| t.target.platform_id == *platform)
                    .cloned()
                    .collect();
                relevant.extend(
                    snapshot
                        .evaluate(*platform, &local, None, true, true)
                        .policy_ids,
                );
            }
            for policy in &snapshot.policies {
                if source_identity(&policy.source).is_some_and(|(kind, id)| {
                    selection.is_none_or(|s| s.contains_workload(kind, id))
                }) {
                    relevant.insert(policy.id);
                }
            }
            if selection.is_none() {
                relevant.extend(snapshot.policies.iter().map(|p| p.id));
            }
            let ids = relevant.into_iter().collect::<Vec<_>>();
            let runs = sqlx::query("SELECT latest.* FROM unnest($1::uuid[]) policy_id CROSS JOIN LATERAL (SELECT * FROM backupruns WHERE backuppolicyid=policy_id ORDER BY queuedat DESC,id DESC LIMIT 1) latest")
                .bind(&ids).fetch_all(&self.0).await.map_err(storage)?.into_iter().map(map_run).collect::<Result<Vec<_>,_>>()?;
            let run_ids = runs.iter().map(|r| r.id).collect::<Vec<_>>();
            let target_platforms: Vec<_> = targets.iter().map(|t| t.target.platform_id).collect();
            let names: Vec<_> = targets.iter().map(|t| t.target.name.as_str()).collect();
            let paths: Vec<_> = targets
                .iter()
                .map(|t| t.target.host_path.as_deref())
                .collect();
            let nodes: Vec<_> = targets
                .iter()
                .map(|t| t.target.docker_node_id.as_deref())
                .collect();
            let items = sqlx::query("SELECT i.* FROM backuprunitems i WHERE backuprunid=ANY($1) AND ($6 OR EXISTS(SELECT 1 FROM unnest($2::uuid[],$3::text[],$4::text[],$5::text[]) t(platformid,name,hostpath,nodeid) WHERE t.platformid=i.platformid AND t.hostpath IS NOT DISTINCT FROM i.hostpath AND t.nodeid IS NOT DISTINCT FROM i.dockernodeid AND (t.hostpath IS NOT NULL OR t.name=i.volumename)))")
                .bind(&run_ids).bind(&target_platforms).bind(&names).bind(&paths).bind(&nodes).bind(selection.is_none())
                .fetch_all(&self.0)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_run_item)
                .collect::<Result<Vec<_>, _>>()?;
            let mut by_run = std::collections::HashMap::<Uuid, Vec<BackupRunItem>>::new();
            for item in items {
                by_run.entry(item.backup_run_id).or_default().push(item);
            }
            let policy_index = snapshot
                .policies
                .iter()
                .enumerate()
                .map(|(i, p)| (p.id, i))
                .collect::<std::collections::HashMap<_, _>>();
            for mut run in runs {
                run.items = by_run.remove(&run.id).unwrap_or_default();
                if let Some(&index) = policy_index.get(&run.backup_policy_id) {
                    snapshot.policies[index].latest_run = Some(run);
                }
            }
            let ids: Vec<_> = snapshot.policies.iter().map(|p| p.id).collect();
            let rows=sqlx::query("SELECT i.platformid,i.volumename,i.hostpath,i.dockernodeid,MAX(COALESCE(i.completedat,r.completedat,r.queuedat)) AS at FROM backuprunitems i JOIN backupruns r ON r.id=i.backuprunid WHERE r.backuppolicyid=ANY($1) AND i.platformid=ANY($2) AND i.status='Succeeded' AND i.resticsnapshotid IS NOT NULL AND r.snapshotavailability='Available' AND ($7 OR EXISTS(SELECT 1 FROM unnest($3::uuid[],$4::text[],$5::text[],$6::text[]) t(platformid,name,hostpath,nodeid) WHERE t.platformid=i.platformid AND t.hostpath IS NOT DISTINCT FROM i.hostpath AND t.nodeid IS NOT DISTINCT FROM i.dockernodeid AND (t.hostpath IS NOT NULL OR t.name=i.volumename))) GROUP BY i.platformid,i.volumename,i.hostpath,i.dockernodeid")
                .bind(&ids).bind(platforms).bind(&target_platforms).bind(&names).bind(&paths).bind(&nodes).bind(selection.is_none()).fetch_all(&self.0).await.map_err(storage)?;
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
    let selection = CoverageSelection {
        volumes: targets.iter().map(|t| t.target.clone()).collect(),
        ..Default::default()
    };
    let snapshot = PostgresVolumeCoverageReader(pool.clone())
        .snapshot(actor, administrator, &[platform], Some(&selection))
        .await?;
    Ok(targets
        .iter()
        .map(|target| {
            let mut summary = snapshot.evaluate(
                platform,
                std::slice::from_ref(target),
                None,
                true,
                scheduling_available,
            );
            let v = summary.volumes.remove(0);
            let policies = snapshot
                .policies
                .iter()
                .filter(|p| v.policy_ids.contains(&p.id))
                .collect::<Vec<_>>();
            let latest = policies
                .iter()
                .copied()
                .filter_map(|p| p.latest_run.as_ref())
                .max_by_key(|r| (r.queued_at, r.id));
            summaries::VolumeBackupCoverage {
                platform_id: platform,
                volume_name: v.target.name,
                docker_node_id: v.target.docker_node_id,
                status: v.status.unwrap_or(BackupCoverageStatus::Warning),
                policy_count: v.policy_ids.len() as i32,
                policies: policies
                    .into_iter()
                    .map(|p| summaries::VolumeCoveragePolicy {
                        id: p.id,
                        name: p.name.clone(),
                        enabled: p.enabled,
                        schedule_configured: p.scheduled(),
                    })
                    .collect(),
                reasons: summary.reasons,
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

fn selected_targets(
    snapshot: &CoverageSnapshot,
    selection: Option<&CoverageSelection>,
) -> Vec<ObservedStorage> {
    let mut targets = std::collections::BTreeMap::new();
    if let Some(selected) = selection {
        for target in &selected.volumes {
            targets.insert(target.clone(), false);
        }
    }
    for workload in &snapshot.workloads {
        if selection.is_none_or(|s| s.contains_workload(workload.kind, workload.id)) {
            for v in &workload.volumes {
                targets.insert(v.target.clone(), v.included);
            }
        }
    }
    for container in &snapshot.containers {
        if selection.is_none_or(|s| {
            s.containers.contains(&container.id)
                || s.container_keys.iter().any(|(p, n, id)| {
                    *p == container.platform_id
                        && *n == container.docker_node_id
                        && *id == container.docker_id
                })
        }) {
            for v in &container.volumes {
                targets.insert(v.target.clone(), v.included);
            }
        }
    }
    targets
        .into_iter()
        .map(|(target, included)| ObservedStorage { target, included })
        .collect()
}
