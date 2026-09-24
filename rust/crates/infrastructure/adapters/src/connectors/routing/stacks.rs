#[cfg(test)]
use std::path::{Path, PathBuf};

use base64::Engine as _;
use citadel_platforms::{PlatformInventoryPort, PlatformResourceMutationPort};
use citadel_stacks::{
    ComposeProjectRuntimeService, StackAction, StackApplySource, StackDeletionClaim, StackDrift,
    StackDriftPolicy, StackError, StackImportClaim, StackImportKind, StackOperationClaim,
    StackOrchestrationMode, StackProgressItem, StackReconciliationAction,
    StackReconciliationActionType, StackReleaseStatus, StackRuntime, StackRuntimeContainer,
    StackRuntimeResult, StackRuntimeService, StackRuntimeSnapshot,
};
use futures_util::{FutureExt, future::BoxFuture};
use serde_json::Value;
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::connectors::agent::client::AgentClient;
use crate::connectors::agent::client::AgentContainerAction;
use crate::connectors::agent::client::AgentStackRegistry;
use crate::connectors::docker::DockerClient;
use crate::persistence::postgres::stacks::release_resources;
mod import;
mod update_scanner;
pub use update_scanner::StackUpdateRuntime;

use crate::external::stacks::run_docker;

#[derive(Clone)]
pub struct StackRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    agent: Option<AgentClient>,
    edge: crate::connectors::edge::EdgeRegistry,
    image_cache: std::sync::Arc<crate::connectors::registries::digest_cache::ImageDigestCache>,
}

impl StackRuntimeRouter {
    #[must_use]
    pub fn new(pool: PgPool, docker: DockerClient, agent: Option<AgentClient>) -> Self {
        Self {
            pool,
            docker,
            agent,
            edge: crate::connectors::edge::EdgeRegistry::default(),
            image_cache: Default::default(),
        }
    }

    pub fn with_image_cache(
        mut self,
        cache: std::sync::Arc<crate::connectors::registries::digest_cache::ImageDigestCache>,
    ) -> Self {
        self.image_cache = cache;
        self
    }

    #[must_use]
    pub fn with_edge(mut self, edge: crate::connectors::edge::EdgeRegistry) -> Self {
        self.edge = edge;
        self
    }

    async fn platform(&self, platform_id: Uuid) -> Result<PlatformTarget, StackError> {
        let row = sqlx::query("SELECT connectortype,address,status FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(StackError::NotFound)?;
        if row.try_get::<String, _>("status").map_err(storage)? != "Online" {
            return Err(StackError::Conflict(
                "The Stack Platform is offline.".to_owned(),
            ));
        }
        Ok(PlatformTarget {
            platform_id,
            connector: crate::persistence::postgres::platforms::classification::connector_kind(
                row.try_get("connectortype").map_err(storage)?,
            )
            .map_err(storage)?,
            address: row.try_get("address").map_err(storage)?,
        })
    }

    fn agent_for(
        &self,
        target: &PlatformTarget,
    ) -> Result<crate::connectors::agent::execution::AgentExecutionClient, StackError> {
        use crate::connectors::agent::execution::AgentExecutionClient;
        use crate::connectors::edge::EdgeTarget;
        if target.connector == citadel_platforms::ConnectorKind::EdgeAgent {
            return self
                .edge
                .get(&EdgeTarget::platform(target.platform_id))
                .map(AgentExecutionClient::Edge)
                .map_err(|_| {
                    StackError::Runtime("The Edge Agent is disconnected or unavailable.".into())
                });
        }
        let agent = self
            .agent
            .as_ref()
            .filter(|_| target.connector == citadel_platforms::ConnectorKind::Agent)
            .ok_or_else(|| {
                StackError::Runtime("The configured Agent transport is unavailable.".into())
            })?;
        let agent = agent
            .at_address(&target.address)
            .map_err(|error| StackError::Runtime(error.message))?;
        Ok(AgentExecutionClient::Direct(std::sync::Arc::new(agent)))
    }

    async fn apply_local(
        &self,
        claim: &StackOperationClaim,
        source: &StackApplySource,
        environment: &[String],
        registry: Option<&AgentStackRegistry>,
        cancellation: &CancellationToken,
        progress: Option<&citadel_stacks::StackProgress>,
    ) -> Result<StackRuntimeResult, StackError> {
        let common = claim.spec.common();
        crate::external::stacks::LocalStackApply {
            docker: "docker".into(),
            endpoint: self.docker.endpoint().clone(),
            project_name: claim.project_name.clone(),
            orchestration: orchestration(&claim.platform_type),
            destroy_before_deploy: common.destroy_before_deploy,
            pre_deploy: common.pre_deploy.clone(),
            post_deploy: common.post_deploy.clone(),
            service_names: claim.service_names.clone(),
            pull_images: true,
        }
        .apply(
            source,
            environment,
            registry.map(|registry| (registry.host.as_str(), registry.auth.as_str())),
            cancellation,
            progress,
        )
        .await
    }

    async fn snapshot(
        &self,
        platform_id: Uuid,
        project: &str,
        orchestration: StackOrchestrationMode,
    ) -> Result<StackRuntimeSnapshot, StackError> {
        match orchestration {
            StackOrchestrationMode::DockerCompose => {
                // Local drift needs current Docker state and Compose service labels;
                // the inventory projection can lag immediately after an apply.
                if self.platform(platform_id).await?.connector
                    == citadel_platforms::ConnectorKind::Local
                {
                    let containers = self
                        .docker
                        .list_containers(true)
                        .await
                        .map_err(runtime_io)?;
                    return Ok(local_compose_snapshot(containers, project));
                }
                let rows = sqlx::query("SELECT dockercontainerid,name,state FROM containers WHERE platformid=$1 AND stack=$2 AND NOT isswarmtask ORDER BY dockercontainerid")
                    .bind(platform_id).bind(project).fetch_all(&self.pool).await.map_err(storage)?;
                Ok(StackRuntimeSnapshot {
                    containers: rows
                        .into_iter()
                        .map(|row| {
                            Ok(StackRuntimeContainer {
                                docker_container_id: row
                                    .try_get("dockercontainerid")
                                    .map_err(storage)?,
                                service_name: row
                                    .try_get::<String, _>("name")
                                    .map_err(storage)?
                                    .trim_start_matches('/')
                                    .to_owned(),
                                state: row.try_get("state").map_err(storage)?,
                                health: None,
                            })
                        })
                        .collect::<Result<_, StackError>>()?,
                    services: Vec::new(),
                })
            }
            StackOrchestrationMode::DockerSwarm => {
                let rows = sqlx::query("SELECT dockerserviceid,name,versionindex,desiredtaskcount,runningtaskcount,updatestate,updatemessage FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY dockerserviceid")
                    .bind(platform_id).bind(project).fetch_all(&self.pool).await.map_err(storage)?;
                Ok(StackRuntimeSnapshot {
                    containers: Vec::new(),
                    services: rows
                        .into_iter()
                        .map(|row| {
                            Ok(StackRuntimeService {
                                docker_service_id: row
                                    .try_get("dockerserviceid")
                                    .map_err(storage)?,
                                name: row.try_get("name").map_err(storage)?,
                                version_index: row.try_get("versionindex").map_err(storage)?,
                                desired_tasks: row.try_get("desiredtaskcount").map_err(storage)?,
                                running_tasks: row.try_get("runningtaskcount").map_err(storage)?,
                                update_state: row.try_get("updatestate").map_err(storage)?,
                                update_message: row.try_get("updatemessage").map_err(storage)?,
                            })
                        })
                        .collect::<Result<_, StackError>>()?,
                })
            }
        }
    }

    async fn compose_container_ids(
        &self,
        platform_id: Uuid,
        project: &str,
    ) -> Result<Vec<String>, StackError> {
        sqlx::query_scalar(
            "SELECT dockercontainerid FROM containers WHERE platformid=$1 AND stack=$2 AND NOT isswarmtask ORDER BY dockercontainerid",
        )
        .bind(platform_id)
        .bind(project)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)
    }

    async fn swarm_service_ids(
        &self,
        platform_id: Uuid,
        project: &str,
    ) -> Result<Vec<String>, StackError> {
        sqlx::query_scalar(
            "SELECT dockerserviceid FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY dockerserviceid",
        )
        .bind(platform_id)
        .bind(project)
        .fetch_all(&self.pool)
        .await
        .map_err(storage)
    }

    async fn delete_compose_networks<F, Fut>(
        &self,
        networks: Vec<citadel_platforms::RuntimeNetworkSummary>,
        project: &str,
        delete: F,
    ) -> Result<(), StackError>
    where
        F: Fn(String) -> Fut,
        Fut: std::future::Future<Output = Result<(), citadel_platforms::RuntimeCapabilityError>>,
    {
        for network in networks.into_iter().filter(|network| {
            network
                .labels
                .get("com.docker.compose.project")
                .is_some_and(|value| value == project)
        }) {
            delete(network.id).await.map_err(agent_error)?;
        }
        Ok(())
    }
}

impl StackRuntime for StackRuntimeRouter {
    fn apply<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        source: &'a StackApplySource,
        environment: &'a [String],
        cancellation: &'a CancellationToken,
        progress: Option<&'a citadel_stacks::StackProgress>,
    ) -> BoxFuture<'a, Result<StackRuntimeResult, StackError>> {
        async move {
            validate_stack_execution(claim)?;
            let target = self.platform(claim.platform_id).await?;
            let registry = self.stack_registry(claim.spec.common().registry_id).await?;
            let result = if target.connector == citadel_platforms::ConnectorKind::Local  {
                self.apply_local(claim, source, environment, registry.as_ref(), cancellation, progress).await
            } else if matches!(target.connector, citadel_platforms::ConnectorKind::Agent | citadel_platforms::ConnectorKind::EdgeAgent) {
                self.agent_for(&target)?.apply_stack(claim, source, environment, registry.as_ref(), cancellation, progress).await.map_err(agent_error)
            } else {
                Err(StackError::Runtime("Edge Agent Stack mutations are not available until the inbound command transport migrates.".to_owned()))
            }?;
            if claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm && result.status == StackReleaseStatus::Healthy {
                let runtime: Box<dyn PlatformInventoryPort> = if target.connector == citadel_platforms::ConnectorKind::Local {
                    Box::new(self.docker.clone())
                } else {
                    match self.agent_for(&target)? {
                        crate::connectors::agent::execution::AgentExecutionClient::Direct(agent) => Box::new((*agent).clone()),
                        crate::connectors::agent::execution::AgentExecutionClient::Edge(session) => Box::new(crate::connectors::edge::EdgeRuntime { session }),
                    }
                };
                tokio::time::timeout(std::time::Duration::from_secs(30), release_resources::capture(&self.pool, claim, runtime.as_ref(), cancellation))
                    .await.map_err(|_| StackError::Runtime("Timed out recording immutable Swarm release resources.".into()))??;
            }
            Ok(result)
        }.boxed()
    }

    fn observe<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<StackRuntimeResult>, StackError>> {
        async move {
            let snapshot = self
                .snapshot(
                    claim.platform_id,
                    &claim.project_name,
                    orchestration(&claim.platform_type),
                )
                .await?;
            let status = if claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm {
                if snapshot.services.is_empty() {
                    return Ok(None);
                }
                if snapshot.services.iter().any(|service| {
                    matches!(
                        service.update_state.to_ascii_lowercase().as_str(),
                        "paused" | "rollback_paused" | "rollbackcompleted" | "rollback_completed"
                    )
                }) {
                    StackReleaseStatus::Failed
                } else if snapshot
                    .services
                    .iter()
                    .all(|service| service.desired_tasks == service.running_tasks)
                {
                    StackReleaseStatus::Healthy
                } else {
                    StackReleaseStatus::Degraded
                }
            } else {
                if snapshot.containers.is_empty() {
                    return Ok(None);
                }
                if snapshot
                    .containers
                    .iter()
                    .all(|container| container.state.eq_ignore_ascii_case("Running"))
                {
                    StackReleaseStatus::Healthy
                } else {
                    StackReleaseStatus::Degraded
                }
            };
            let messages = if status == StackReleaseStatus::Failed {
                snapshot
                    .services
                    .iter()
                    .filter_map(|service| service.update_message.as_ref())
                    .map(|message| StackProgressItem::system(message.clone()))
                    .collect()
            } else {
                Vec::new()
            };
            Ok(Some(StackRuntimeResult { status, messages }))
        }
        .boxed()
    }

    fn delete<'a>(
        &'a self,
        claim: &'a StackDeletionClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), StackError>> {
        async move {
            let target = self.platform(claim.platform_id).await?;
            if claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm
                && (target.connector == citadel_platforms::ConnectorKind::Local)
            {
                let args = vec![
                    "stack".to_owned(),
                    "rm".to_owned(),
                    claim.project_name.clone(),
                ];
                let result = run_docker(
                    self.docker.endpoint(),
                    &args,
                    &std::env::temp_dir(),
                    &[],
                    cancellation,
                    None,
                )
                .await?;
                return if result.status == StackReleaseStatus::Healthy {
                    Ok(())
                } else {
                    Err(StackError::RuntimeRejected(last_message(&result)))
                };
            }

            if claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm {
                let service_ids = self
                    .swarm_service_ids(claim.platform_id, &claim.project_name)
                    .await?;
                let agent = self.agent_for(&target)?;
                for service_id in service_ids {
                    agent.delete_managed_swarm_service(
                        citadel_contracts::citadel::swarm::v1::DeleteManagedSwarmServiceRequest {
                            operation_id: Uuid::now_v7().to_string(),
                            service_id,
                        },
                        cancellation,
                    ).await.map_err(agent_error)?;
                }
                return Ok(());
            }

            let container_ids = self
                .compose_container_ids(claim.platform_id, &claim.project_name)
                .await?;
            if target.connector == citadel_platforms::ConnectorKind::Local {
                for id in container_ids {
                    self.docker
                        .delete_container(&id, false, true)
                        .await
                        .map_err(runtime_io)?;
                }
                let networks = PlatformInventoryPort::list_networks(&self.docker, cancellation)
                    .await
                    .map_err(agent_error)?;
                self.delete_compose_networks(networks, &claim.project_name, |id| async move {
                    PlatformResourceMutationPort::delete_network(&self.docker, &id, cancellation)
                        .await
                })
                .await
            } else {
                let agent = self.agent_for(&target)?;
                for id in container_ids {
                    agent
                        .delete_container(&id, cancellation)
                        .await
                        .map_err(agent_error)?;
                }
                let networks = agent
                    .list_networks(cancellation)
                    .await
                    .map_err(agent_error)?;
                let agent = &agent;
                self.delete_compose_networks(networks, &claim.project_name, |id| async move {
                    agent.delete_network(&id, cancellation).await
                })
                .await
            }
        }
        .boxed()
    }

    fn change_state<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        orchestration: StackOrchestrationMode,
        action: StackAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<String>, StackError>> {
        async move {
            if orchestration == StackOrchestrationMode::DockerSwarm {
                return Err(StackError::Validation(
                    "Container state actions are not available for Docker Swarm stacks.".to_owned(),
                ));
            }
            let target = self.platform(platform_id).await?;
            let ids = self
                .compose_container_ids(platform_id, project_name)
                .await?;
            if ids.is_empty() {
                return Err(StackError::NotFound);
            }
            if target.connector == citadel_platforms::ConnectorKind::Local {
                let command = match action {
                    StackAction::Start => "start",
                    StackAction::Stop => "stop",
                    StackAction::Pause => "pause",
                    StackAction::Resume => "unpause",
                    StackAction::Restart => "restart",
                };
                let mut args = Vec::with_capacity(ids.len() + 1);
                args.push(command.to_owned());
                args.extend(ids.iter().cloned());
                let result = run_docker(
                    self.docker.endpoint(),
                    &args,
                    &std::env::temp_dir(),
                    &[],
                    cancellation,
                    None,
                )
                .await?;
                if result.status == StackReleaseStatus::Healthy {
                    Ok(ids)
                } else {
                    Err(StackError::RuntimeRejected(last_message(&result)))
                }
            } else {
                let action = match action {
                    StackAction::Start => AgentContainerAction::Start,
                    StackAction::Stop => AgentContainerAction::Stop,
                    StackAction::Pause => AgentContainerAction::Pause,
                    StackAction::Resume => AgentContainerAction::Unpause,
                    StackAction::Restart => AgentContainerAction::Restart,
                };
                self.agent_for(&target)?
                    .change_containers_state(&ids, action, cancellation)
                    .await
                    .map_err(agent_error)?;
                Ok(ids)
            }
        }
        .boxed()
    }

    fn runtime_snapshot<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        orchestration: StackOrchestrationMode,
        _cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeSnapshot, StackError>> {
        async move {
            self.snapshot(platform_id, project_name, orchestration)
                .await
        }
        .boxed()
    }

    fn reconcile<'a>(
        &'a self,
        platform_id: Uuid,
        drifts: &'a [StackDrift],
        policy: &'a StackDriftPolicy,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<StackReconciliationAction>, StackError>> {
        async move {
            let target = self.platform(platform_id).await?;
            let mut actions = Vec::new();
            for drift in drifts {
                let selected = match drift {
                    StackDrift::ContainerStopped {
                        container_id,
                        service_name,
                    } if policy.auto_start_stopped_containers => Some((
                        container_id,
                        service_name,
                        StackReconciliationActionType::StartContainer,
                        AgentContainerAction::Start,
                    )),
                    StackDrift::ContainerPaused {
                        container_id,
                        service_name,
                    } if policy.auto_resume_paused_containers => Some((
                        container_id,
                        service_name,
                        StackReconciliationActionType::ResumeContainer,
                        AgentContainerAction::Unpause,
                    )),
                    StackDrift::ExtraContainer {
                        container_id,
                        service_name,
                    } if policy.remove_extra_containers => {
                        let result = if target.connector == citadel_platforms::ConnectorKind::Local
                        {
                            self.docker
                                .delete_container(container_id, false, true)
                                .await
                                .map_err(runtime_io)
                        } else {
                            self.agent_for(&target)?
                                .delete_container(container_id, cancellation)
                                .await
                                .map_err(agent_error)
                        };
                        actions.push(reconciliation_action(
                            container_id,
                            service_name,
                            StackReconciliationActionType::RemoveContainer,
                            result,
                        ));
                        None
                    }
                    _ => None,
                };
                if let Some((container_id, service_name, action_type, agent_action)) = selected {
                    let result = if target.connector == citadel_platforms::ConnectorKind::Local {
                        let command = if agent_action == AgentContainerAction::Start {
                            "start"
                        } else {
                            "unpause"
                        };
                        run_docker(
                            self.docker.endpoint(),
                            &[command.to_owned(), container_id.clone()],
                            &std::env::temp_dir(),
                            &[],
                            cancellation,
                            None,
                        )
                        .await
                        .and_then(|result| {
                            if result.status == StackReleaseStatus::Healthy {
                                Ok(())
                            } else {
                                Err(StackError::RuntimeRejected(last_message(&result)))
                            }
                        })
                    } else {
                        self.agent_for(&target)?
                            .change_containers_state(
                                std::slice::from_ref(container_id),
                                agent_action,
                                cancellation,
                            )
                            .await
                            .map_err(agent_error)
                    };
                    actions.push(reconciliation_action(
                        container_id,
                        service_name,
                        action_type,
                        result,
                    ));
                }
            }
            Ok(actions)
        }
        .boxed()
    }

    fn import_claim<'a>(
        &'a self,
        platform_id: Uuid,
        project_name: &'a str,
        import_kind: Option<StackImportKind>,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackImportClaim, StackError>> {
        async move {
            let platform = sqlx::query("SELECT name,platformdescriptor FROM platforms WHERE id=$1 AND status='Online'")
                .bind(platform_id).fetch_optional(&self.pool).await.map_err(storage)?.ok_or(StackError::NotFound)?;
            let descriptor: serde_json::Value = platform.try_get("platformdescriptor").map_err(storage)?;
            let swarm = descriptor.get("$type").and_then(serde_json::Value::as_str) == Some("DockerSwarm");
            let import_kind = import_kind.unwrap_or(if swarm {
                StackImportKind::SwarmStack
            } else {
                StackImportKind::ComposeProject
            });
            if import_kind == StackImportKind::SwarmStack && !swarm {
                return Err(StackError::Validation(
                    "Docker Stack import requires a Docker Swarm platform.".to_owned(),
                ));
            }
            let orphaned_owner_id;
            let mut ownership_fingerprint = String::new();
            let (service_names, container_ids, container_names, services) = if import_kind == StackImportKind::SwarmStack {
                let rows=sqlx::query("SELECT dockerserviceid,name,image,desiredtaskcount,runningtaskcount,updatestate,labels,ownership,stackid,swarmserviceid FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY name")
                    .bind(platform_id).bind(project_name).fetch_all(&self.pool).await.map_err(storage)?;
                let mut owners = Vec::new();
                for row in &rows {
                    if row.try_get::<Option<Uuid>, _>("stackid").map_err(storage)?.is_some()
                        || row.try_get::<Option<Uuid>, _>("swarmserviceid").map_err(storage)?.is_some()
                        || !matches!(row.try_get::<String, _>("ownership").map_err(storage)?.as_str(), "DockerStackExternal" | "Unmanaged") {
                        return Err(StackError::Conflict("A Service is already managed or has conflicting ownership.".into()));
                    }
                    let labels = serde_json::from_value(row.try_get::<Value, _>("labels").map_err(storage)?).map_err(runtime_io)?;
                    owners.push(crate::connectors::containers::ownership::owner(&labels).map_err(|message| StackError::Conflict(message.into()))?);
                    ownership_fingerprint.push_str(&serde_json::to_string(&labels).map_err(runtime_io)?);
                }
                orphaned_owner_id = self.orphaned_stack_owner(&owners).await?;
                let prefix = format!("{project_name}_");
                let services=rows.iter().map(|row| {
                    let docker_name: String = row.try_get("name").map_err(storage)?;
                    Ok(ComposeProjectRuntimeService {
                        name: docker_name.strip_prefix(&prefix).unwrap_or(&docker_name).to_owned(),
                        image: Some(row.try_get("image").map_err(storage)?),
                        container_count: usize::try_from(row.try_get::<i32,_>("desiredtaskcount").map_err(storage)?).unwrap_or_default(),
                        states: vec![
                            row.try_get::<String,_>("updatestate").map_err(storage)?,
                            format!("running:{}", row.try_get::<i32,_>("runningtaskcount").map_err(storage)?),
                        ],
                    })
                }).collect::<Result<Vec<_>,StackError>>()?;
                let names=services.iter().map(|service|service.name.clone()).collect::<Vec<_>>();
                (names,Vec::new(),Vec::new(),services)
            } else {
                let claim = self.compose_import(platform_id, project_name, cancellation).await?;
                orphaned_owner_id = claim.orphaned_owner_id;
                ownership_fingerprint = claim.runtime_fingerprint;
                (claim.service_names, claim.container_ids, claim.container_names, claim.services)
            };
            if service_names.is_empty() { return Err(StackError::NotFound); }
            let mut hash = Sha256::new(); hash.update(platform_id.as_bytes()); hash.update(project_name.as_bytes());
            hash.update(ownership_fingerprint.as_bytes());
            for service in &services {
                hash.update(service.name.as_bytes()); hash.update([0]);
                hash.update(service.image.as_deref().unwrap_or_default().as_bytes()); hash.update([0]);
                hash.update(service.container_count.to_le_bytes());
                for state in &service.states { hash.update(state.as_bytes()); hash.update([0]); }
            }
            for id in &container_ids { hash.update(id.as_bytes()); hash.update([0]); }
            for name in &container_names { hash.update(name.as_bytes()); hash.update([0]); }
            let fingerprint = hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
            Ok(StackImportClaim { orphaned_owner_id, platform_id, platform_name: platform.try_get("name").map_err(storage)?, project_name: project_name.to_owned(), import_kind, runtime_fingerprint: format!("sha256:{fingerprint}"), service_names, container_ids, container_names, services })
        }.boxed()
    }
}

impl StackRuntimeRouter {
    async fn stack_registry(
        &self,
        registry_id: Option<Uuid>,
    ) -> Result<Option<AgentStackRegistry>, StackError> {
        let Some(registry_id) = registry_id else {
            return Ok(None);
        };
        let row = sqlx::query(
            "SELECT name,registryhost,configuration,status FROM registries WHERE id=$1",
        )
        .bind(registry_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(storage)?
        .ok_or_else(|| StackError::Validation("The selected Registry was not found.".to_owned()))?;
        let status: String = row.try_get("status").map_err(storage)?;
        if status.eq_ignore_ascii_case("Disabled") {
            return Err(StackError::Validation(
                "The selected Registry is disabled.".to_owned(),
            ));
        }
        let configuration: Value = row.try_get("configuration").map_err(storage)?;
        let Some((username, password)) = registry_user_password(&configuration)? else {
            return Ok(None);
        };
        Ok(Some(AgentStackRegistry {
            name: row.try_get("name").map_err(storage)?,
            host: row.try_get("registryhost").map_err(storage)?,
            auth: zeroize::Zeroizing::new(
                base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}")),
            ),
        }))
    }
}

fn registry_user_password(configuration: &Value) -> Result<Option<(String, String)>, StackError> {
    let kind = configuration
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let string = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| configuration.get(*name).and_then(Value::as_str))
    };
    let boolean = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| configuration.get(*name).and_then(Value::as_bool))
    };
    let values = match kind {
        "DockerHub" => string(&["UserName", "userName", "username"]).zip(string(&["PAT", "pat"])),
        "GitHub" if boolean(&["GhcrAuthEnabled", "ghcrAuthEnabled"]) == Some(true) => {
            string(&["NameSpace", "nameSpace"]).zip(string(&["PAT", "pat"]))
        }
        "Custom" if boolean(&["AuthEnabled", "authEnabled"]) == Some(true) => {
            string(&["UserName", "userName", "username"]).zip(string(&["Password", "password"]))
        }
        "GitHub" | "Custom" | "" => None,
        _ => {
            return Err(StackError::Validation(
                "This Registry type is not available for Stack Apply in the Rust server yet."
                    .to_owned(),
            ));
        }
    };
    Ok(values
        .filter(|(username, password)| !username.is_empty() && !password.is_empty())
        .map(|(username, password)| (username.to_owned(), password.to_owned())))
}

fn validate_stack_execution(claim: &StackOperationClaim) -> Result<(), StackError> {
    let common = claim.spec.common();
    if claim.platform_type == citadel_platforms::PlatformKind::DockerSwarm
        && (common.destroy_before_deploy
            || common.pre_deploy.is_some()
            || common.post_deploy.is_some())
    {
        return Err(StackError::Validation(
            "Swarm Stack deploy does not support destroy-before-deploy or pre/post commands."
                .to_owned(),
        ));
    }
    Ok(())
}

fn local_compose_snapshot(
    containers: Vec<crate::connectors::docker::projection::ContainerSummary>,
    project: &str,
) -> StackRuntimeSnapshot {
    let mut containers = containers
        .into_iter()
        .filter(|container| {
            container
                .labels
                .get("com.docker.compose.project")
                .is_some_and(|value| value == project)
                && !container.labels.contains_key("com.docker.swarm.task.id")
                && !container
                    .labels
                    .get("com.docker.compose.oneoff")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"))
        })
        .map(|container| StackRuntimeContainer {
            service_name: container
                .labels
                .get("com.docker.compose.service")
                .cloned()
                .unwrap_or_else(|| {
                    container
                        .names
                        .first()
                        .map(|name| name.trim_start_matches('/').to_owned())
                        .unwrap_or_default()
                }),
            docker_container_id: container.id,
            state: container.state,
            health: container
                .status
                .to_ascii_lowercase()
                .contains("(unhealthy)")
                .then(|| "unhealthy".into()),
        })
        .collect::<Vec<_>>();
    containers.sort_by(|a, b| a.docker_container_id.cmp(&b.docker_container_id));
    StackRuntimeSnapshot {
        containers,
        services: vec![],
    }
}

fn last_message(result: &StackRuntimeResult) -> String {
    result
        .messages
        .iter()
        .rev()
        .find_map(|item| item.message.clone())
        .unwrap_or_else(|| "Docker rejected the Stack operation.".to_owned())
}
fn orchestration(value: &citadel_platforms::PlatformKind) -> StackOrchestrationMode {
    match value {
        citadel_platforms::PlatformKind::Docker => StackOrchestrationMode::DockerCompose,
        citadel_platforms::PlatformKind::DockerSwarm => StackOrchestrationMode::DockerSwarm,
    }
}

fn runtime_io(error: impl std::fmt::Display) -> StackError {
    StackError::Runtime(error.to_string())
}
fn storage(error: impl std::fmt::Display) -> StackError {
    StackError::Storage(error.to_string())
}
fn agent_error(error: impl std::fmt::Display) -> StackError {
    StackError::Runtime(error.to_string())
}
struct PlatformTarget {
    platform_id: Uuid,
    connector: citadel_platforms::ConnectorKind,
    address: String,
}

fn reconciliation_action(
    container_id: &str,
    service_name: &str,
    action: StackReconciliationActionType,
    result: Result<(), StackError>,
) -> StackReconciliationAction {
    StackReconciliationAction {
        container_id: container_id.to_owned(),
        service_name: service_name.to_owned(),
        action,
        succeeded: result.is_ok(),
        error_message: result.err().map(|error| error.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::external::stacks::*;
    use citadel_stacks::{StackApplyEventType, StackCommand};
    use std::process::Stdio;
    use std::time::Duration;
    use tokio::process::Command;

    #[test]
    fn transported_stack_paths_cannot_escape_the_staging_root() {
        let root = Path::new("/tmp/citadel-stack-source");
        assert!(resolve_staged_path(root, "compose.yml").is_ok());
        assert!(resolve_staged_path(root, "apps/web/compose.yml").is_ok());
        assert!(resolve_staged_path(root, "../compose.yml").is_err());
        assert!(resolve_staged_path(root, "/compose.yml").is_err());
        assert!(resolve_staged_path(root, "").is_err());
    }

    #[test]
    fn compose_arguments_preserve_source_file_and_environment_order() {
        let args = compose_args(
            &[PathBuf::from("base.yml"), PathBuf::from("prod.yml")],
            Some(Path::new("citadel.labels.yml")),
            &[PathBuf::from("repo.env"), PathBuf::from("resolved.env")],
            "demo",
            &["up", "-d"],
        );
        assert_eq!(
            args,
            [
                "compose",
                "-p",
                "demo",
                "-f",
                "base.yml",
                "-f",
                "prod.yml",
                "-f",
                "citadel.labels.yml",
                "--env-file",
                "repo.env",
                "--env-file",
                "resolved.env",
                "up",
                "-d",
            ]
        );
    }

    #[test]
    fn swarm_stack_rejects_compose_only_execution_options() {
        let claim = StackOperationClaim {
            stack_id: Uuid::now_v7(),
            release_id: Uuid::now_v7(),
            platform_id: Uuid::now_v7(),
            name: "demo".to_owned(),
            project_name: "demo".to_owned(),
            platform_type: citadel_platforms::PlatformKind::DockerSwarm,
            spec: citadel_stacks::StackSpec::WebEditor {
                compose_file: "services: {}".to_owned(),
                update_behavior: citadel_stacks::StackUpdateBehavior::Disabled,
                common: citadel_stacks::StackSpecCommon {
                    destroy_before_deploy: false,
                    pre_deploy: Some(StackCommand {
                        commands: vec!["echo unsupported".to_owned()],
                        path: ".".to_owned(),
                    }),
                    ..Default::default()
                },
            },
            row_version: 1,
            actor_id: Uuid::nil(),
            operation: "Apply".to_owned(),
            service_names: Vec::new(),
        };

        assert!(matches!(
            validate_stack_execution(&claim),
            Err(StackError::Validation(message)) if message.contains("pre/post")
        ));
    }

    // ProcessRunner/StackService yield stdout and stderr before command exit.
    #[cfg(unix)]
    #[tokio::test]
    async fn process_streams_both_pipes_before_exit_and_preserves_failure() {
        let root = std::env::temp_dir().join(format!("citadel-stack-stream-{}", Uuid::now_v7()));
        tokio::fs::create_dir_all(&root).await.unwrap();
        let cancellation = CancellationToken::new();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        let progress = citadel_stacks::StackProgress::new(
            sender,
            vec!["private-token".into()],
            cancellation.clone(),
        );
        let mut command = Command::new("/bin/sh");
        command.arg("-c").arg("printf 'stdout private-token\\n'; printf 'stderr ready\\r' >&2; while [ ! -f release ]; do sleep 0.02; done; printf 'last line'; exit 7")
            .current_dir(&root).stdout(Stdio::piped()).stderr(Stdio::piped());
        let process = collect_process(command, &cancellation, Some(&progress));
        let observe = async {
            let first = receiver.recv().await.unwrap();
            let second = receiver.recv().await.unwrap();
            assert!(
                [&first, &second]
                    .iter()
                    .any(|item| item.event_type == StackApplyEventType::StdOut)
            );
            assert!(
                [&first, &second]
                    .iter()
                    .any(|item| item.event_type == StackApplyEventType::StdErr)
            );
            assert!(
                [&first, &second].iter().all(|item| !item
                    .message
                    .as_deref()
                    .unwrap()
                    .contains("private-token"))
            );
            // The process cannot finish until progress is received by the caller.
            tokio::fs::write(root.join("release"), b"").await.unwrap();
            assert_eq!(
                receiver.recv().await.unwrap().message.as_deref(),
                Some("last line")
            );
            assert_eq!(receiver.recv().await.unwrap().exit_code, Some(7));
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(process, observe)
        })
        .await
        .unwrap();
        assert_eq!(result.unwrap().status, StackReleaseStatus::Failed);
        tokio::fs::remove_dir_all(root).await.unwrap();
    }

    #[tokio::test]
    async fn secrets_split_across_pipe_reads_are_redacted_before_streaming() {
        use tokio::io::AsyncWriteExt;
        let (mut writer, reader) = tokio::io::duplex(4);
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let progress = citadel_stacks::StackProgress::new(
            sender,
            vec!["private-token".into()],
            CancellationToken::new(),
        );
        let read = read_process_output(reader, StackApplyEventType::StdOut, Some(&progress));
        let write = async {
            writer.write_all(b"value private-token\n").await.unwrap();
            drop(writer);
        };
        let (result, ()) =
            tokio::time::timeout(Duration::from_secs(5), async { tokio::join!(read, write) })
                .await
                .unwrap();
        result.unwrap();
        let text = receiver.recv().await.unwrap().message.unwrap();
        assert!(text.starts_with("value "));
        assert!(!text.contains("private-token"));
    }

    #[tokio::test]
    async fn output_streams_past_retention_limit_and_discards_oversized_lines() {
        let cancellation = CancellationToken::new();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(2);
        let progress = citadel_stacks::StackProgress::new(sender, vec![], cancellation);
        let mut bytes = vec![b'x'; MAX_PROCESS_OUTPUT_BYTES + 1];
        bytes.extend_from_slice(b"\n");
        for _ in 0..300 {
            bytes.extend_from_slice(format!("{}\n", "y".repeat(1024)).as_bytes());
        }
        bytes.extend_from_slice(b"last");
        let read = read_process_output(
            bytes.as_slice(),
            StackApplyEventType::StdErr,
            Some(&progress),
        );
        let observe = async {
            for _ in 0..300 {
                assert_eq!(
                    receiver.recv().await.unwrap().message.unwrap(),
                    "y".repeat(1024)
                );
            }
            assert_eq!(
                receiver.recv().await.unwrap().message.as_deref(),
                Some("last")
            );
        };
        let (retained, ()) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(read, observe)
        })
        .await
        .unwrap();
        let retained = retained.unwrap();
        assert!(retained.len() < 300);
        assert!(
            retained
                .iter()
                .map(|item| item.message.as_ref().unwrap().len() + 64)
                .sum::<usize>()
                <= MAX_PROCESS_OUTPUT_BYTES
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn cancellation_interrupts_a_process_with_a_full_progress_queue() {
        let cancellation = CancellationToken::new();
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let progress = citadel_stacks::StackProgress::new(sender, vec![], cancellation.clone());
        let mut command = Command::new("/bin/sh");
        command
            .arg("-c")
            .arg("while :; do printf 'progress\\n'; done")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        let process = collect_process(command, &cancellation, Some(&progress));
        let cancel = async {
            receiver.recv().await.unwrap();
            cancellation.cancel();
        };
        let (result, ()) = tokio::time::timeout(Duration::from_secs(5), async {
            tokio::join!(process, cancel)
        })
        .await
        .unwrap();
        assert!(matches!(result, Err(StackError::Cancelled)));
    }

    #[test]
    fn local_drift_uses_compose_service_labels_and_current_state() {
        let container = |project: &str, service: &str, state: &str| {
            crate::connectors::docker::projection::ContainerSummary {
                id: service.into(),
                names: vec!["/custom-container-name".into()],
                state: state.into(),
                labels: [
                    ("com.docker.compose.project".into(), project.into()),
                    ("com.docker.compose.service".into(), service.into()),
                ]
                .into(),
                ..Default::default()
            }
        };
        let snapshot = local_compose_snapshot(
            vec![
                container("beszel", "agent", "running"),
                container("other", "other", "running"),
                container("beszel", "web", "exited"),
            ],
            "beszel",
        );
        assert_eq!(snapshot.containers.len(), 2);
        assert_eq!(snapshot.containers[1].service_name, "web");
        assert_eq!(snapshot.containers[1].state, "exited");
    }

    #[test]
    fn shell_commands_are_passed_as_one_argument_to_the_platform_shell() {
        let (_, arguments) = shell_invocation("printf '%s' \"$TOKEN\"");
        assert_eq!(
            arguments.last().map(String::as_str),
            Some("printf '%s' \"$TOKEN\"")
        );
    }

    #[test]
    fn stack_registry_credentials_follow_the_existing_registry_contract() {
        let credentials = registry_user_password(&serde_json::json!({
            "$type": "DockerHub",
            "UserName": "citadel",
            "PAT": "secret"
        }))
        .unwrap()
        .unwrap();
        assert_eq!(credentials, ("citadel".to_owned(), "secret".to_owned()));
        assert_eq!(
            registry_user_password(&serde_json::json!({
                "$type": "Custom",
                "AuthEnabled": false,
                "UserName": "ignored",
                "Password": "ignored"
            }))
            .unwrap(),
            None
        );
    }

    #[test]
    fn docker_configuration_is_a_global_cli_option() {
        let mut arguments = vec!["stack".to_owned(), "deploy".to_owned()];
        prepend_docker_config(&mut arguments, Some(Path::new("/tmp/docker-config")));
        assert_eq!(
            arguments,
            ["--config", "/tmp/docker-config", "stack", "deploy"]
        );
    }

    #[tokio::test]
    async fn stack_commands_use_the_staged_directory_and_resolved_environment() {
        let root = std::env::temp_dir().join(format!(
            "citadel-stack-command-test-{}",
            Uuid::now_v7().simple()
        ));
        tokio::fs::create_dir_all(&root).await.unwrap();
        let command = if cfg!(windows) {
            "echo %CITADEL_TEST_VALUE%>marker.txt"
        } else {
            "printf %s \"$CITADEL_TEST_VALUE\" > marker.txt"
        };
        let result = run_stack_commands(
            &StackCommand {
                commands: vec![command.to_owned()],
                path: ".".to_owned(),
            },
            &root,
            &["CITADEL_TEST_VALUE=resolved".to_owned()],
            &CancellationToken::new(),
            None,
        )
        .await
        .unwrap();
        assert_eq!(result.status, StackReleaseStatus::Healthy);
        assert_eq!(
            tokio::fs::read_to_string(root.join("marker.txt"))
                .await
                .unwrap()
                .trim(),
            "resolved"
        );
        tokio::fs::remove_dir_all(root).await.unwrap();
    }
}
