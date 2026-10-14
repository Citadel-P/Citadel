use super::*;
use citadel_backups::policies::read_models::{
    BackupPreviewKind, BackupPreviewResource, BackupSourcePreview, BackupVolumePreview,
};
use citadel_primitives::{ActorId, ResourceType};
use sqlx::AssertSqlSafe;

impl PostgresBackupSourcePlanner {
    pub(super) async fn preview_source(
        &self,
        kind: BackupPreviewKind,
        id: Uuid,
        actor: ActorId,
        administrator: bool,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BackupSourcePreview, BackupError> {
        let query = match kind {
            BackupPreviewKind::Deployment => {
                "SELECT d.name,d.platformid,d.spec,p.name AS platformname,p.status,p.platformdescriptor FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE d.id=$1"
            }
            BackupPreviewKind::Stack => {
                "SELECT s.name,r.platformid,r.id AS releaseid,r.spec,p.name AS platformname,p.status,p.platformdescriptor FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1"
            }
            BackupPreviewKind::SwarmService => {
                "SELECT s.name,s.platformid,s.spec,p.name AS platformname,p.status,p.platformdescriptor FROM swarmservices s JOIN platforms p ON p.id=s.platformid WHERE s.id=$1"
            }
        };
        let row = sqlx::query(AssertSqlSafe(query))
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)?;
        let platform = platform_from_row(&row)?;
        let name: String = row.try_get("name").map_err(storage)?;
        let resource = match kind {
            BackupPreviewKind::Deployment => BackupPreviewResource::Deployment { id, name },
            BackupPreviewKind::Stack => BackupPreviewResource::Stack { id, name },
            BackupPreviewKind::SwarmService => BackupPreviewResource::SwarmService { id, name },
        };
        let mut warnings = Vec::new();
        let mut volumes = BTreeMap::<(Option<String>, String), BackupVolumePreview>::new();
        match kind {
            BackupPreviewKind::Deployment => {
                let spec =
                    DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
                        .map_err(|e| BackupError::Storage(e.to_string()))?;
                let declared = spec
                    .volumes
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|m| named_deployment_volume(m))
                    .collect::<BTreeSet<_>>();
                let discovery = self
                    .workload_mounts(kind, id, platform.id, false, cancellation)
                    .await?;
                warnings.extend(discovery.warnings);
                for storage in discovery.items {
                    let name = storage.host_path.as_deref().unwrap_or(&storage.volume_name);
                    let storage_kind = if storage.host_path.is_some() {
                        "BindMount"
                    } else if declared.contains(name) {
                        "DeclaredNamed"
                    } else {
                        "AnonymousNamed"
                    };
                    volumes.insert(
                        (storage.docker_node_id.clone(), name.to_owned()),
                        item(
                            name,
                            storage_kind,
                            false,
                            false,
                            storage.docker_node_id,
                            storage.node_hostname,
                        ),
                    );
                }
                if !volumes.values().any(|v| v.kind != "BindMount") {
                    warnings.push("No synchronized volume mounts were found. Deployment volume settings are used as a fallback.".into());
                    for name in declared {
                        volumes.insert(
                            (None, name.clone()),
                            item(&name, "DeclaredNamed", false, false, None, None),
                        );
                    }
                }
            }
            BackupPreviewKind::Stack if platform.kind != "DockerSwarm" => {
                let name: String = row.try_get("name").map_err(storage)?;
                let plan = self
                    .standalone_stack_storage(id, platform.id, &name, false, cancellation)
                    .await?;
                warnings.extend(plan.warnings);
                for storage in plan.items {
                    let name = storage.host_path.as_deref().unwrap_or(&storage.volume_name);
                    volumes.insert(
                        (storage.docker_node_id.clone(), name.to_owned()),
                        item(
                            name,
                            if storage.host_path.is_some() {
                                "BindMount"
                            } else {
                                "DockerVolume"
                            },
                            false,
                            false,
                            storage.docker_node_id,
                            storage.node_hostname,
                        ),
                    );
                }
            }
            kind => {
                let plan = match kind {
                    BackupPreviewKind::Stack => self.plan_stack(id, None, cancellation).await?,
                    BackupPreviewKind::SwarmService => {
                        self.plan_swarm_service(id, None, cancellation).await?
                    }
                    BackupPreviewKind::Deployment => unreachable!(),
                };
                warnings.extend(plan.warnings);
                for volume in plan.items {
                    let source_name = volume.host_path.as_deref().unwrap_or(&volume.volume_name);
                    let source_kind = if volume.host_path.is_some() {
                        "BindMount"
                    } else {
                        "DeclaredNamed"
                    };
                    let shared = warnings.iter().any(|warning| {
                        warning.contains(&format!(
                            "Volume {} on Node {} ",
                            volume.volume_name,
                            volume.docker_node_id.as_deref().unwrap_or_default()
                        ))
                    });
                    volumes.insert(
                        (volume.docker_node_id.clone(), source_name.to_owned()),
                        item(
                            source_name,
                            source_kind,
                            false,
                            shared,
                            volume.docker_node_id,
                            volume.node_hostname,
                        ),
                    );
                }
            }
        }
        if platform.status != citadel_primitives::PlatformStatus::Online {
            warnings.push("The source Platform is offline or unavailable.".into());
        }
        if volumes.is_empty() {
            let warning = "No supported volumes or bind mounts were resolved for this resource.";
            if !warnings.iter().any(|existing| existing == warning) {
                warnings.push(warning.into());
            }
        }
        let mut volumes = volumes.into_values().collect::<Vec<_>>();
        self.add_coverage(actor, administrator, platform.id, &mut volumes)
            .await?;
        Ok(BackupSourcePreview {
            resource,
            platform_id: platform.id,
            platform_name: row.try_get("platformname").map_err(storage)?,
            platform_status: platform.status,
            volumes,
            warnings,
        })
    }

    async fn add_coverage(
        &self,
        actor: ActorId,
        administrator: bool,
        platform: Uuid,
        volumes: &mut [BackupVolumePreview],
    ) -> Result<(), BackupError> {
        if volumes.is_empty() {
            return Ok(());
        }
        let names = volumes.iter().map(|v| v.name.clone()).collect::<Vec<_>>();
        // Protected and Warning coverage count as present: even a disabled
        // or never-run readable Volume policy is Warning. Only no policy or the
        // latest Failed run yields false. Never expose an inaccessible policy.
        let query = format!(
            "{} {}",
            crate::persistence::postgres::backups::AUTHORIZED_CTE,
            r#"
, policies AS (
SELECT p.id,COALESCE(p.source->>'VolumeName',p.source->>'volumeName',p.source->>'HostPath',p.source->>'hostPath') AS volumename,
COALESCE(p.source->>'DockerNodeId',p.source->>'dockerNodeId') AS dockernodeid
FROM backuppolicies p WHERE p.archivedat IS NULL AND p.source->>'$type' IN ('DockerVolume','BindMount')
AND COALESCE(p.source->>'PlatformId',p.source->>'platformId')=$5::text
AND COALESCE(p.source->>'VolumeName',p.source->>'volumeName',p.source->>'HostPath',p.source->>'hostPath')=ANY($6)
AND ($4 OR (SELECT allowed FROM global_access) OR EXISTS(
 SELECT 1 FROM actor_scope scope JOIN resourceaccesses access ON access.actorid=scope.actorid
 WHERE access.resourcetype=$2 AND access.resourceid=p.id AND access.permissionlevel = ANY($3)))
), latest AS (
SELECT DISTINCT ON (p.volumename,p.dockernodeid) p.volumename,p.dockernodeid,r.status
FROM policies p LEFT JOIN backupruns r ON r.backuppolicyid=p.id
ORDER BY p.volumename,p.dockernodeid,r.queuedat DESC NULLS LAST,r.id DESC
)
SELECT volumename,dockernodeid FROM latest WHERE status IS DISTINCT FROM 'Failed'"#
        );
        let covered = sqlx::query(AssertSqlSafe(query.as_str()))
            .bind(actor.value())
            .bind(ResourceType::BackupPolicy as i32)
            .bind(citadel_primitives::PermissionLevel::Read.accepted_database_levels())
            .bind(administrator)
            .bind(platform.to_string())
            .bind(names)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
        for row in covered {
            let name: String = row.try_get("volumename").map_err(storage)?;
            let node: Option<String> = row.try_get("dockernodeid").map_err(storage)?;
            for volume in volumes
                .iter_mut()
                .filter(|v| v.name == name && v.docker_node_id == node)
            {
                volume.has_backup_coverage = true;
            }
        }
        Ok(())
    }
}

fn item(
    name: &str,
    kind: &str,
    is_external: bool,
    is_shared: bool,
    docker_node_id: Option<String>,
    node_hostname: Option<String>,
) -> BackupVolumePreview {
    BackupVolumePreview {
        name: name.into(),
        kind: if is_external { "ExternalNamed" } else { kind }.into(),
        is_external,
        is_shared,
        has_backup_coverage: false,
        docker_node_id,
        node_hostname,
    }
}
