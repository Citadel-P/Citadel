use citadel_backups::spec::BackupSourceSpec;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use citadel_backups::{
    BackupClaim, BackupError, BackupSourceItem, BackupSourcePlan, BackupSourcePlanner,
    CitadelSystemBackupBuilder,
};
use citadel_deployments::DeploymentSpec;
use citadel_git::GitRepositoryExecutionService;
use citadel_stacks::{StackSpec, parse_compose};
use citadel_swarm_services::{MountKind, SwarmServiceSpec};
use futures_util::future::BoxFuture;
use serde_json::Value;
use sqlx::{PgPool, Row};
use uuid::Uuid;
mod preview;

#[derive(Clone)]
pub struct PostgresBackupSourcePlanner {
    runtime: Option<crate::connectors::routing::containers::ContainerRuntimeRouter>,
    pool: PgPool,
    system_builder: Option<Arc<dyn CitadelSystemBackupBuilder>>,
    git_execution: Option<Arc<GitRepositoryExecutionService>>,
}

impl PostgresBackupSourcePlanner {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            runtime: None,
            pool,
            system_builder: None,
            git_execution: None,
        }
    }
    #[must_use]
    pub fn with_runtime(
        mut self,
        runtime: crate::connectors::routing::containers::ContainerRuntimeRouter,
    ) -> Self {
        self.runtime = Some(runtime);
        self
    }

    #[must_use]
    pub fn with_system_builder(mut self, builder: Arc<dyn CitadelSystemBackupBuilder>) -> Self {
        self.system_builder = Some(builder);
        self
    }

    #[must_use]
    pub fn with_git_execution(mut self, execution: Arc<GitRepositoryExecutionService>) -> Self {
        self.git_execution = Some(execution);
        self
    }

    async fn stack_compose_files(
        &self,
        spec: &StackSpec,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<Vec<String>, BackupError> {
        match spec {
            StackSpec::WebEditor { compose_file, .. } => Ok(vec![compose_file.clone()]),
            StackSpec::Git {
                git_repo_id,
                branch,
                commit_sha,
                compose_paths,
                ..
            } => {
                let execution = self.git_execution.as_ref().ok_or_else(|| {
                    BackupError::Validation(
                        "Git-backed Stack source materialization is unavailable.".into(),
                    )
                })?;
                let revision = commit_sha.as_deref().unwrap_or(branch);
                let commit = execution
                    .resolve_commit(*git_repo_id, Some(revision), cancellation)
                    .await
                    .map_err(|error| BackupError::Validation(error.to_string()))?;
                let mut total = 0_usize;
                let mut files = Vec::with_capacity(compose_paths.len());
                for path in compose_paths {
                    let file = execution
                        .read_file(*git_repo_id, Some(&commit), path, cancellation)
                        .await
                        .map_err(|error| BackupError::Validation(error.to_string()))?;
                    let content = file.content.ok_or_else(|| {
                        BackupError::Validation(file.preview_unavailable_reason.unwrap_or_else(
                            || format!("Git Compose file '{path}' is unavailable."),
                        ))
                    })?;
                    total = total.saturating_add(content.len());
                    if total > 2 * 1024 * 1024 {
                        return Err(BackupError::Validation(
                            "Git Stack Compose files exceed the 2 MiB planning limit.".into(),
                        ));
                    }
                    files.push(content);
                }
                Ok(files)
            }
        }
    }

    async fn plan_volume(&self, claim: &BackupClaim) -> Result<BackupSourcePlan, BackupError> {
        let BackupSourceSpec::DockerVolume {
            platform_id,
            volume_name,
            docker_node_id,
            ..
        } = &claim.run.source_snapshot
        else {
            return Err(BackupError::Validation(
                "Expected a Docker Volume source.".into(),
            ));
        };
        let platform_id = *platform_id;
        let volume_name = volume_name.trim().to_owned();
        let docker_node_id = docker_node_id
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(str::to_owned);
        let platform = self.platform(platform_id).await?;
        validate_node_target(&platform.kind, docker_node_id.as_deref())?;
        validate_repository(&platform.kind, &claim.repository.repository_type)?;
        Ok(BackupSourcePlan {
            display_name: format!("Docker volume {volume_name}"),
            items: vec![BackupSourceItem::new(
                platform_id,
                volume_name,
                docker_node_id,
                None,
            )],
            warnings: vec![],
            local_directory: None,
        })
    }

    async fn plan_deployment(
        &self,
        deployment_id: Uuid,
        repository_type: Option<&str>,
        preview: bool,
    ) -> Result<BackupSourcePlan, BackupError> {
        let row = sqlx::query(
            "SELECT d.name,d.platformid,d.spec,p.platformdescriptor,p.status FROM deployments d JOIN platforms p ON p.id=d.platformid WHERE d.id=$1",
        )
        .bind(deployment_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or(BackupError::NotFound)?;
        let platform = platform_from_row(&row)?;
        if platform.kind == "DockerSwarm" && !preview {
            return Err(BackupError::Validation(
                "A Deployment on Docker Swarm requires an exact Task placement before its local Volumes can be backed up.".into(),
            ));
        }
        if let Some(repository) = repository_type {
            validate_repository(&platform.kind, repository)?;
        }
        if !preview {
            require_online(&platform)?;
        }
        let spec = DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
            .map_err(|error| BackupError::Storage(error.to_string()))?;
        let volumes = spec
            .volumes
            .unwrap_or_default()
            .into_iter()
            .filter_map(|mount| named_deployment_volume(&mount))
            .collect::<BTreeSet<_>>();
        if volumes.is_empty() && !preview {
            return Err(BackupError::Validation(
                "Deployment has no Docker named Volumes to back up.".into(),
            ));
        }
        Ok(BackupSourcePlan {
            display_name: format!(
                "Deployment {} ({} volume{})",
                row.try_get::<String, _>("name").map_err(storage)?,
                volumes.len(),
                plural(volumes.len())
            ),
            items: volumes
                .into_iter()
                .map(|volume| BackupSourceItem::new(platform.id, volume, None, None))
                .collect(),
            warnings: if preview && platform.status != citadel_primitives::PlatformStatus::Online {
                vec!["The Deployment Platform is offline. Saved volume settings are used as a fallback.".into()]
            } else {
                vec![]
            },
            local_directory: None,
        })
    }

    async fn plan_stack(
        &self,
        stack_id: Uuid,
        repository_type: Option<&str>,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BackupSourcePlan, BackupError> {
        let row = sqlx::query(
            "SELECT s.name,r.id AS releaseid,r.platformid,r.spec,p.platformdescriptor,p.status FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid JOIN platforms p ON p.id=r.platformid WHERE s.id=$1",
        )
        .bind(stack_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or(BackupError::NotFound)?;
        let platform = platform_from_row(&row)?;
        require_online(&platform)?;
        if let Some(repository) = repository_type {
            validate_repository(&platform.kind, repository)?;
        }
        let name: String = row.try_get("name").map_err(storage)?;
        let release_id: Uuid = row.try_get("releaseid").map_err(storage)?;
        if platform.kind != "DockerSwarm" {
            let volumes = sqlx::query_scalar::<_, String>(
                "SELECT volumename FROM stackreleasevolumebindings WHERE stackreleaseid=$1 ORDER BY volumename",
            )
            .bind(release_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?;
            if volumes.is_empty() {
                return Err(BackupError::Validation(
                    "Stack has no applied Docker Volume bindings to back up.".into(),
                ));
            }
            return Ok(BackupSourcePlan {
                display_name: format!(
                    "Stack {name} ({} volume{})",
                    volumes.len(),
                    plural(volumes.len())
                ),
                items: volumes
                    .into_iter()
                    .map(|volume| BackupSourceItem::new(platform.id, volume, None, None))
                    .collect(),
                warnings: vec![],
                local_directory: None,
            });
        }

        let spec = StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
            .map_err(|error| BackupError::Storage(error.to_string()))?;
        let compose_files = self.stack_compose_files(&spec, cancellation).await?;
        let model = parse_compose(&compose_files)
            .map_err(|error| BackupError::Validation(error.to_string()))?;
        let namespace = sqlx::query_scalar::<_, String>(
            "SELECT namespace FROM stackswarmnamespacereservations WHERE stackid=$1",
        )
        .bind(stack_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| {
            BackupError::Validation("Swarm Stack has no applied namespace reservation.".into())
        })?;
        let services = sqlx::query(
            "SELECT dockerserviceid,name,desiredtaskcount,runningtaskcount,isstale FROM swarmserviceprojections WHERE platformid=$1 AND stackid=$2 ORDER BY name",
        )
        .bind(platform.id)
        .bind(stack_id)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)?;
        if services.is_empty() {
            return Err(BackupError::Validation(
                "Swarm Stack has no current Service projections.".into(),
            ));
        }
        let mut items = BTreeMap::<(String, String), BackupSourceItem>::new();
        let mut counts = BTreeMap::<(String, String), usize>::new();
        for service in services {
            let docker_name: String = service.try_get("name").map_err(storage)?;
            let compose_name = docker_name
                .strip_prefix(&format!("{namespace}_"))
                .unwrap_or(&docker_name);
            let volume_refs = model
                .service_volumes
                .get(compose_name)
                .cloned()
                .unwrap_or_default();
            if volume_refs.is_empty() && self.runtime.is_none() {
                continue;
            }
            validate_service_projection(&service, &docker_name)?;
            let service_id: String = service.try_get("dockerserviceid").map_err(storage)?;
            let tasks = current_tasks(&self.pool, platform.id, &service_id).await?;
            let desired: i32 = service.try_get("desiredtaskcount").map_err(storage)?;
            if tasks.len() != usize::try_from(desired).unwrap_or(usize::MAX) {
                return Err(BackupError::Validation(format!(
                    "Service '{docker_name}' Task placement is not stable."
                )));
            }
            for task in tasks {
                if self.runtime.is_some() {
                    for volume in self.task_volumes(platform.id, &task, cancellation).await? {
                        insert_task_volume(&mut items, &mut counts, platform.id, &task, volume)?;
                    }
                    continue;
                }
                for compose_volume in &volume_refs {
                    let physical = physical_swarm_volume(&model, &namespace, compose_volume);
                    insert_task_volume(&mut items, &mut counts, platform.id, &task, physical)?;
                }
            }
        }
        if items.is_empty() && repository_type.is_none() {
            return Ok(empty_preview_plan(format!("Stack {name}")));
        }
        finish_swarm_plan(format!("Stack {name}"), items, counts)
    }

    async fn plan_swarm_service(
        &self,
        id: Uuid,
        repository_type: Option<&str>,
        cancellation: &tokio_util::sync::CancellationToken,
    ) -> Result<BackupSourcePlan, BackupError> {
        let row = sqlx::query(
            "SELECT s.name,s.platformid,s.spec,s.dockerserviceid,p.platformdescriptor,p.status,sp.desiredtaskcount,sp.runningtaskcount,sp.isstale FROM swarmservices s JOIN platforms p ON p.id=s.platformid LEFT JOIN swarmserviceprojections sp ON sp.platformid=s.platformid AND (sp.swarmserviceid=s.id OR sp.dockerserviceid=s.dockerserviceid) WHERE s.id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or(BackupError::NotFound)?;
        let platform = platform_from_row(&row)?;
        if platform.kind != "DockerSwarm" {
            return Err(BackupError::Validation(
                "The selected Service is not on a Docker Swarm Platform.".into(),
            ));
        }
        require_online(&platform)?;
        if let Some(repository) = repository_type {
            validate_repository(&platform.kind, repository)?;
        }
        let name: String = row.try_get("name").map_err(storage)?;
        validate_service_projection(&row, &name)?;
        let docker_service_id = row
            .try_get::<Option<String>, _>("dockerserviceid")
            .map_err(storage)?
            .filter(|value| !value.is_empty())
            .ok_or_else(|| BackupError::Validation("Swarm Service has not been applied.".into()))?;
        let spec = SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)
            .map_err(|error| BackupError::Storage(error.to_string()))?;
        let volumes = spec
            .mounts
            .into_iter()
            .filter(|mount| mount.kind == MountKind::Volume && !mount.source.trim().is_empty())
            .map(|mount| mount.source)
            .collect::<BTreeSet<_>>();
        if volumes.is_empty() && self.runtime.is_none() {
            if repository_type.is_none() {
                return Ok(empty_preview_plan(format!("Swarm Service {name}")));
            }
            return Err(BackupError::Validation(
                "Swarm Service has no supported local named Volumes to back up.".into(),
            ));
        }
        let tasks = current_tasks(&self.pool, platform.id, &docker_service_id).await?;
        let desired: i32 = row.try_get("desiredtaskcount").map_err(storage)?;
        if tasks.len() != usize::try_from(desired).unwrap_or(usize::MAX) {
            return Err(BackupError::Validation(format!(
                "Service '{name}' Task placement is not stable."
            )));
        }
        let mut items = BTreeMap::<(String, String), BackupSourceItem>::new();
        let mut counts = BTreeMap::<(String, String), usize>::new();
        for task in tasks {
            if self.runtime.is_some() {
                for volume in self.task_volumes(platform.id, &task, cancellation).await? {
                    insert_task_volume(&mut items, &mut counts, platform.id, &task, volume)?;
                }
                continue;
            }
            for volume in &volumes {
                insert_task_volume(&mut items, &mut counts, platform.id, &task, volume.clone())?;
            }
        }
        if items.is_empty() && repository_type.is_none() {
            return Ok(empty_preview_plan(format!("Swarm Service {name}")));
        }
        finish_swarm_plan(format!("Swarm Service {name}"), items, counts)
    }

    async fn task_volumes(
        &self,
        platform_id: Uuid,
        task: &TaskPlacement,
        cancel: &tokio_util::sync::CancellationToken,
    ) -> Result<BTreeSet<String>, BackupError> {
        use crate::connectors::routing::containers::Runtime;
        use citadel_platforms::containers::{ContainerInspectionPort, ContainerTarget};
        let router = self
            .runtime
            .as_ref()
            .ok_or_else(|| BackupError::Validation("Node inspection is unavailable.".into()))?;
        let docker_id = task
            .container_id
            .as_ref()
            .filter(|id| !id.is_empty())
            .ok_or_else(|| {
                BackupError::Validation("Current Task has no Container to inspect.".into())
            })?;
        let target = ContainerTarget {
            id: Uuid::nil(),
            platform_id,
            docker_id: docker_id.clone(),
            node_id: Some(task.docker_node_id.clone()),
        };
        let runtime = router
            .resolve(&target, cancel)
            .await
            .map_err(|e| BackupError::Validation(e.message))?;
        let inspected = match runtime {
            Runtime::Local(r) => r.inspection(docker_id, cancel).await,
            Runtime::Agent(r) => r.inspection(docker_id, cancel).await,
            Runtime::Edge(r) => r.inspection(docker_id, cancel).await,
        }
        .map_err(|e| BackupError::Validation(e.message))?;
        Ok(inspected
            .get("mounts")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|m| m.get("type").and_then(Value::as_str) == Some("volume"))
            .filter_map(|m| m.get("name").and_then(Value::as_str))
            .filter(|name| !name.is_empty())
            .map(str::to_owned)
            .collect())
    }

    async fn platform(&self, id: Uuid) -> Result<Platform, BackupError> {
        let row = sqlx::query("SELECT id,platformdescriptor,status FROM platforms WHERE id=$1")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(BackupError::NotFound)?;
        platform_from_row(&row)
    }
}

impl BackupSourcePlanner for PostgresBackupSourcePlanner {
    fn preview<'a>(
        &'a self,
        kind: citadel_backups::policies::read_models::BackupPreviewKind,
        id: Uuid,
        actor: citadel_primitives::ActorId,
        administrator: bool,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<
        'a,
        Result<citadel_backups::policies::read_models::BackupSourcePreview, BackupError>,
    > {
        Box::pin(self.preview_source(kind, id, actor, administrator, cancellation))
    }
    fn validate_source<'a>(
        &'a self,
        source: &'a BackupSourceSpec,
        repository: &'a citadel_backups::BackupRepository,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<(), BackupError>> {
        Box::pin(async move {
            match source {
                BackupSourceSpec::DockerVolume {
                    platform_id,
                    docker_node_id,
                    ..
                } => {
                    let platform = self.platform(*platform_id).await?;
                    validate_node_target(&platform.kind, docker_node_id.as_deref())?;
                    validate_repository(&platform.kind, &repository.repository_type)?;
                }
                BackupSourceSpec::Deployment { deployment_id } => {
                    self.plan_deployment(*deployment_id, Some(&repository.repository_type), false)
                        .await?;
                }
                BackupSourceSpec::Stack { stack_id } => {
                    self.plan_stack(*stack_id, Some(&repository.repository_type), cancellation)
                        .await?;
                }
                BackupSourceSpec::SwarmService { swarm_service_id } => {
                    self.plan_swarm_service(
                        *swarm_service_id,
                        Some(&repository.repository_type),
                        cancellation,
                    )
                    .await?;
                }
                BackupSourceSpec::CitadelSystem {} => {}
            }
            Ok(())
        })
    }
    fn plan<'a>(
        &'a self,
        claim: &'a BackupClaim,
        cancellation: &'a tokio_util::sync::CancellationToken,
    ) -> BoxFuture<'a, Result<BackupSourcePlan, BackupError>> {
        Box::pin(async move {
            match &claim.run.source_snapshot {
                BackupSourceSpec::DockerVolume { .. } => self.plan_volume(claim).await,
                BackupSourceSpec::Deployment { deployment_id } => {
                    self.plan_deployment(
                        *deployment_id,
                        Some(&claim.repository.repository_type),
                        false,
                    )
                    .await
                }
                BackupSourceSpec::Stack { stack_id } => {
                    self.plan_stack(
                        *stack_id,
                        Some(&claim.repository.repository_type),
                        cancellation,
                    )
                    .await
                }
                BackupSourceSpec::SwarmService { swarm_service_id } => {
                    self.plan_swarm_service(
                        *swarm_service_id,
                        Some(&claim.repository.repository_type),
                        cancellation,
                    )
                    .await
                }
                BackupSourceSpec::CitadelSystem {} => {
                    let builder = self.system_builder.as_ref().ok_or_else(|| {
                        BackupError::Validation(
                            "Citadel system recovery bundle creation is unavailable.".into(),
                        )
                    })?;
                    let directory = builder.build(claim.run.id, cancellation).await?;
                    Ok(BackupSourcePlan {
                        display_name: "Citadel recovery bundle".into(),
                        items: vec![],
                        warnings: vec![],
                        local_directory: Some(directory),
                    })
                }
            }
        })
    }
}

#[derive(Debug)]
struct Platform {
    id: Uuid,
    kind: String,
    status: citadel_primitives::PlatformStatus,
}

#[derive(Debug)]
struct TaskPlacement {
    container_id: Option<String>,
    docker_node_id: String,
    node_hostname: String,
}

async fn current_tasks(
    pool: &PgPool,
    platform_id: Uuid,
    service_id: &str,
) -> Result<Vec<TaskPlacement>, BackupError> {
    let rows=sqlx::query("SELECT dockernodeid,nodehostname,state,desiredstate,isstale,dockercontainerid FROM swarmtaskprojections WHERE platformid=$1 AND dockerserviceid=$2 AND lower(desiredstate)='running' ORDER BY dockertaskid LIMIT 1001")
        .bind(platform_id)
        .bind(service_id)
        .fetch_all(pool)
        .await
        .map_err(storage)?;
    if rows.len() > 1000 {
        return Err(BackupError::Validation(
            "Backup source exceeds the 1000 current Task limit.".into(),
        ));
    }
    rows.into_iter()
        .map(|row| {
            let stale: bool = row.try_get("isstale").map_err(storage)?;
            let state: String = row.try_get("state").map_err(storage)?;
            if stale || !state.eq_ignore_ascii_case("running") {
                return Err(BackupError::Validation(
                    "A current Swarm Task is unavailable or stale.".into(),
                ));
            }
            let docker_node_id: String = row.try_get("dockernodeid").map_err(storage)?;
            if docker_node_id.trim().is_empty() {
                return Err(BackupError::Validation(
                    "A current Swarm Task has no Node placement.".into(),
                ));
            }
            Ok(TaskPlacement {
                container_id: row.try_get("dockercontainerid").map_err(storage)?,
                docker_node_id,
                node_hostname: row.try_get("nodehostname").map_err(storage)?,
            })
        })
        .collect()
}

fn insert_task_volume(
    items: &mut BTreeMap<(String, String), BackupSourceItem>,
    counts: &mut BTreeMap<(String, String), usize>,
    platform_id: Uuid,
    task: &TaskPlacement,
    volume: String,
) -> Result<(), BackupError> {
    if volume.trim().is_empty() {
        return Err(BackupError::Validation(
            "A Swarm Volume name is empty.".into(),
        ));
    }
    let key = (task.docker_node_id.clone(), volume.clone());
    if items.len() >= 1000 && !items.contains_key(&key) {
        return Err(BackupError::Validation(
            "Backup source exceeds the 1000 resolved Volume limit.".into(),
        ));
    }
    *counts.entry(key.clone()).or_default() += 1;
    items.entry(key).or_insert_with(|| {
        BackupSourceItem::new(
            platform_id,
            volume,
            Some(task.docker_node_id.clone()),
            Some(task.node_hostname.clone()),
        )
    });
    Ok(())
}

fn finish_swarm_plan(
    display: String,
    items: BTreeMap<(String, String), BackupSourceItem>,
    counts: BTreeMap<(String, String), usize>,
) -> Result<BackupSourcePlan, BackupError> {
    if items.is_empty() {
        return Err(BackupError::Validation(format!(
            "{display} has no resolved Docker named Volumes to back up."
        )));
    }
    let warnings = counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|((node, volume), _)| {
            format!("Volume {volume} on Node {node} is mounted by more than one current Task.")
        })
        .collect();
    let items = items.into_values().collect::<Vec<_>>();
    Ok(BackupSourcePlan {
        display_name: format!("{display} ({} volume{})", items.len(), plural(items.len())),
        items,
        warnings,
        local_directory: None,
    })
}

fn empty_preview_plan(display_name: String) -> BackupSourcePlan {
    BackupSourcePlan {
        display_name,
        items: vec![],
        warnings: vec!["No supported Docker named volumes were resolved for this resource.".into()],
        local_directory: None,
    }
}

fn platform_from_row(row: &sqlx::postgres::PgRow) -> Result<Platform, BackupError> {
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    Ok(Platform {
        id: row
            .try_get("platformid")
            .or_else(|_| row.try_get("id"))
            .map_err(storage)?,
        kind: discriminator(&descriptor)?.to_owned(),
        status: row
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(storage)?,
    })
}

fn validate_service_projection(row: &sqlx::postgres::PgRow, name: &str) -> Result<(), BackupError> {
    let stale = row
        .try_get::<Option<bool>, _>("isstale")
        .map_err(storage)?
        .unwrap_or(true);
    let desired = row
        .try_get::<Option<i32>, _>("desiredtaskcount")
        .map_err(storage)?
        .unwrap_or_default();
    let running = row
        .try_get::<Option<i32>, _>("runningtaskcount")
        .map_err(storage)?
        .unwrap_or_default();
    if stale || desired <= 0 || running != desired {
        return Err(BackupError::Validation(format!(
            "Service '{name}' does not have a stable set of running Tasks."
        )));
    }
    Ok(())
}

fn require_online(platform: &Platform) -> Result<(), BackupError> {
    if platform.status == citadel_primitives::PlatformStatus::Online {
        Ok(())
    } else {
        Err(BackupError::Validation(
            "The backup source Platform is offline.".into(),
        ))
    }
}

fn validate_node_target(kind: &str, node: Option<&str>) -> Result<(), BackupError> {
    match (kind, node.filter(|value| !value.trim().is_empty())) {
        ("DockerSwarm", None) => Err(BackupError::Validation(
            "Docker Swarm Volume backup requires an explicit Node.".into(),
        )),
        ("DockerStandalone", Some(_)) => Err(BackupError::Validation(
            "Docker Standalone Volume backup cannot target a Swarm Node.".into(),
        )),
        _ => Ok(()),
    }
}

fn validate_repository(kind: &str, repository_type: &str) -> Result<(), BackupError> {
    if kind == "DockerSwarm" && repository_type != "S3Compatible" {
        return Err(BackupError::Validation(
            "Docker Swarm backups require an S3-compatible Backup Repository.".into(),
        ));
    }
    Ok(())
}

fn named_deployment_volume(mount: &str) -> Option<String> {
    let source = mount.split(':').next()?.trim();
    if source.is_empty()
        || source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with('~')
        || source.contains('\\')
    {
        None
    } else {
        Some(source.to_owned())
    }
}

fn physical_swarm_volume(
    model: &citadel_stacks::ComposeModel,
    namespace: &str,
    compose_name: &str,
) -> String {
    if model.external_volumes.contains(compose_name)
        || model
            .volume_names
            .get(compose_name)
            .is_some_and(|name| name != compose_name)
    {
        model
            .volume_names
            .get(compose_name)
            .cloned()
            .unwrap_or_else(|| compose_name.to_owned())
    } else {
        format!("{namespace}_{compose_name}")
    }
}

fn discriminator(value: &Value) -> Result<&str, BackupError> {
    value
        .get("$type")
        .or_else(|| value.get("Type"))
        .and_then(Value::as_str)
        .ok_or_else(|| BackupError::Validation("Backup source type is missing.".into()))
}

fn optional_string(value: &Value, key: &str) -> Option<String> {
    let alternate = pascal(key);
    value
        .get(key)
        .or_else(|| value.get(&alternate))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_owned)
}

fn pascal(value: &str) -> String {
    let mut chars = value.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

const fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

fn storage(error: impl std::fmt::Display) -> BackupError {
    BackupError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deployment_mount_parser_excludes_bind_and_anonymous_mounts() {
        assert_eq!(
            named_deployment_volume("data:/var/lib/app"),
            Some("data".into())
        );
        assert_eq!(named_deployment_volume("./data:/var/lib/app"), None);
        assert_eq!(named_deployment_volume("/var/data:/var/lib/app"), None);
        assert_eq!(named_deployment_volume("/var/lib/app"), None);
    }

    #[test]
    fn swarm_volume_identity_respects_namespace_and_explicit_names() {
        let model = parse_compose(&["services:\n  api:\n    image: nginx\n    volumes:\n      - data:/data\n      - shared:/shared\nvolumes:\n  data: {}\n  shared:\n    external: true\n    name: global-shared\n".into()]).unwrap();
        assert_eq!(physical_swarm_volume(&model, "demo", "data"), "demo_data");
        assert_eq!(
            physical_swarm_volume(&model, "demo", "shared"),
            "global-shared"
        );
    }
}
