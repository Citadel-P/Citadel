use super::*;
use crate::connectors::routing::containers::Runtime;
use citadel_backups::policies::read_models::{
    BackupPreviewKind, BackupPreviewResource, BackupSourcePreview, BackupVolumePreview,
};
use citadel_platforms::containers::{ContainerInspectionPort, ContainerTarget};
use citadel_primitives::{ActorId, ResourceType};
use serde_json::json;
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
                let containers = sqlx::query(
                    "SELECT id,dockercontainerid,dockernodeid FROM containers WHERE deploymentid=$1 LIMIT 2",
                )
                .bind(id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?;
                if containers.len() > 1 {
                    return Err(BackupError::Conflict(
                        "Deployment Container ownership is ambiguous.".into(),
                    ));
                }
                let mut mounts = Vec::new();
                if let (Some(runtime), Some(container)) = (&self.runtime, containers.first()) {
                    let target = ContainerTarget {
                        id: container.try_get("id").map_err(storage)?,
                        platform_id: platform.id,
                        docker_id: container.try_get("dockercontainerid").map_err(storage)?,
                        node_id: container.try_get("dockernodeid").map_err(storage)?,
                    };
                    let inspected = async {
                        match runtime.resolve(&target, cancellation).await? {
                            Runtime::Local(r) => {
                                r.inspection(&target.docker_id, cancellation).await
                            }
                            Runtime::Agent(r) => {
                                r.inspection(&target.docker_id, cancellation).await
                            }
                            Runtime::Edge(r) => r.inspection(&target.docker_id, cancellation).await,
                        }
                    }
                    .await;
                    match inspected {
                        Ok(mut document)=>{mounts=document.get_mut("mounts").and_then(Value::as_array_mut).map(std::mem::take).unwrap_or_default();
                            for mount in &mut mounts {if let Some(node)=&target.node_id {mount["dockerNodeId"]=Value::String(node.clone());}}
                        },
                        Err(_)=>warnings.push("The synchronized Container could not be inspected. Saved volume settings are used as a fallback.".into()),
                    }
                }
                for value in &mounts {
                    if field(value, "Type").is_some_and(|v| v.eq_ignore_ascii_case("volume"))
                        && let Some(name) = field(value, "Name").filter(|s| !s.is_empty())
                    {
                        let node = optional_string(value, "dockerNodeId");
                        volumes.insert(
                            (node.clone(), name.into()),
                            item(
                                name,
                                if declared.contains(name) {
                                    "DeclaredNamed"
                                } else {
                                    "AnonymousNamed"
                                },
                                false,
                                false,
                                node,
                                None,
                            ),
                        );
                    }
                }
                if volumes.is_empty() {
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
                let release: Uuid = row.try_get("releaseid").map_err(storage)?;
                let bindings=sqlx::query("SELECT volumename,isanonymous,isexternal FROM stackreleasevolumebindings WHERE stackreleaseid=$1 ORDER BY volumename").bind(release).fetch_all(&self.pool).await.map_err(storage)?;
                for binding in bindings {
                    let name: String = binding.try_get("volumename").map_err(storage)?;
                    let anonymous: bool = binding.try_get("isanonymous").map_err(storage)?;
                    let external: bool = binding.try_get("isexternal").map_err(storage)?;
                    volumes.insert(
                        (None, name.clone()),
                        item(
                            &name,
                            if anonymous {
                                "AnonymousNamed"
                            } else {
                                "DeclaredNamed"
                            },
                            external,
                            false,
                            None,
                            None,
                        ),
                    );
                }
                if volumes.is_empty() {
                    let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
                        .map_err(|e| BackupError::Storage(e.to_string()))?;
                    let compose = self.stack_compose_files(&spec, cancellation).await?;
                    let model = parse_compose(&compose)
                        .map_err(|e| BackupError::Validation(e.to_string()))?;
                    warnings.push("No applied volume bindings were found. Compose declarations are used as a fallback.".into());
                    for service in model.service_volumes.values() {
                        for reference in service {
                            let name = model.volume_names.get(reference).unwrap_or(reference);
                            let key = (None, name.clone());
                            if let Some(existing) = volumes.get_mut(&key) {
                                existing.is_shared = true;
                            } else {
                                volumes.insert(
                                    key,
                                    item(
                                        name,
                                        "DeclaredNamed",
                                        model.external_volumes.contains(reference),
                                        false,
                                        None,
                                        None,
                                    ),
                                );
                            }
                        }
                    }
                }
            }
            kind => {
                let plan = match kind {
                    BackupPreviewKind::Stack => {
                        self.plan_stack(&json!({"stackId":id}), None, cancellation)
                            .await?
                    }
                    BackupPreviewKind::SwarmService => {
                        self.plan_swarm_service(&json!({"swarmServiceId":id}), None, cancellation)
                            .await?
                    }
                    BackupPreviewKind::Deployment => unreachable!(),
                };
                warnings.extend(plan.warnings);
                for volume in plan.items {
                    let shared = warnings.iter().any(|warning| {
                        warning.contains(&format!(
                            "Volume {} on Node {} ",
                            volume.volume_name,
                            volume.docker_node_id.as_deref().unwrap_or_default()
                        ))
                    });
                    volumes.insert(
                        (volume.docker_node_id.clone(), volume.volume_name.clone()),
                        item(
                            &volume.volume_name,
                            "DeclaredNamed",
                            false,
                            shared,
                            volume.docker_node_id,
                            volume.node_hostname,
                        ),
                    );
                }
            }
        }
        if platform.status != "Online" {
            warnings.push("The source Platform is offline or unavailable.".into());
        }
        if volumes.is_empty() {
            warnings.push("No Docker named volumes were resolved for this resource.".into());
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
        // .NET treats Protected and Warning coverage as present: even a disabled
        // or never-run readable Volume policy is Warning. Only no policy or the
        // latest Failed run yields false. Never expose an inaccessible policy.
        let query = format!(
            "{} {}",
            crate::persistence::postgres::backups::AUTHORIZED_CTE,
            r#"
, policies AS (
SELECT p.id,COALESCE(p.source->>'VolumeName',p.source->>'volumeName') AS volumename,
COALESCE(p.source->>'DockerNodeId',p.source->>'dockerNodeId') AS dockernodeid
FROM backuppolicies p WHERE p.archivedat IS NULL AND p.source->>'$type'='DockerVolume'
AND COALESCE(p.source->>'PlatformId',p.source->>'platformId')=$5::text
AND COALESCE(p.source->>'VolumeName',p.source->>'volumeName')=ANY($6)
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
fn field<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value
        .get(key)
        .or_else(|| value.get(format!("{}{}", key[..1].to_lowercase(), &key[1..])))
        .and_then(Value::as_str)
}
