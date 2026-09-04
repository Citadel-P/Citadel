use std::path::{Path, PathBuf};
use std::process::Stdio;

use citadel_platforms::{PlatformInventoryPort, PlatformResourceMutationPort};
use citadel_stacks::{
    ComposeProjectRuntimeService, StackAction, StackApplyEventType, StackDeletionClaim, StackDrift,
    StackDriftPolicy, StackError, StackImportClaim, StackImportKind, StackOperationClaim,
    StackOrchestrationMode, StackReconciliationAction, StackReconciliationActionType,
    StackReleaseStatus, StackRuntimeContainer, StackRuntimePort, StackRuntimeResult,
    StackRuntimeService, StackRuntimeSnapshot, StackStreamItem,
};
use futures_util::{FutureExt, future::BoxFuture};
use sha2::{Digest, Sha256};
use sqlx::{PgPool, Row};
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agent::{AgentClient, AgentContainerAction};
use crate::docker::DockerClient;

const MAX_PROCESS_OUTPUT_BYTES: usize = 256 * 1024;

#[derive(Clone)]
pub struct StackRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    agent: Option<AgentClient>,
}

impl StackRuntimeRouter {
    #[must_use]
    pub fn new(pool: PgPool, docker: DockerClient, agent: Option<AgentClient>) -> Self {
        Self {
            pool,
            docker,
            agent,
        }
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
            connector: row.try_get("connectortype").map_err(storage)?,
            address: row.try_get("address").map_err(storage)?,
        })
    }

    fn agent_for(&self, target: &PlatformTarget) -> Result<&AgentClient, StackError> {
        self.agent
            .as_ref()
            .filter(|agent| {
                agent.address().trim_end_matches('/') == target.address.trim_end_matches('/')
            })
            .ok_or_else(|| {
                StackError::Runtime("The configured Agent transport is unavailable.".to_owned())
            })
    }

    async fn apply_local(
        &self,
        claim: &StackOperationClaim,
        compose: &str,
        environment: &[String],
        cancellation: &CancellationToken,
    ) -> Result<StackRuntimeResult, StackError> {
        let root = temporary_run_root(claim.stack_id);
        tokio::fs::create_dir_all(&root).await.map_err(runtime_io)?;
        let compose_path = root.join("compose.yml");
        let env_path = root.join(".env");
        let result = async {
            tokio::fs::write(&compose_path, compose)
                .await
                .map_err(runtime_io)?;
            if !environment.is_empty() {
                let mut contents = environment.join("\n");
                contents.push('\n');
                tokio::fs::write(&env_path, contents)
                    .await
                    .map_err(runtime_io)?;
            }
            if claim.platform_type == "DockerSwarm" {
                let args = vec![
                    "stack".to_owned(),
                    "deploy".to_owned(),
                    "--detach=false".to_owned(),
                    "--prune".to_owned(),
                    "-c".to_owned(),
                    compose_path.display().to_string(),
                    claim.project_name.clone(),
                ];
                run_docker(&args, &root, environment, cancellation).await
            } else {
                if claim.spec.common().destroy_before_deploy {
                    let down =
                        compose_args(&compose_path, &env_path, &claim.project_name, &["down"]);
                    let down_result = run_docker(&down, &root, environment, cancellation).await?;
                    if down_result.status != StackReleaseStatus::Healthy {
                        return Ok(down_result);
                    }
                }
                let up = compose_args(
                    &compose_path,
                    &env_path,
                    &claim.project_name,
                    &["up", "-d", "--remove-orphans"],
                );
                run_docker(&up, &root, environment, cancellation).await
            }
        }
        .await;
        if let Err(error) = tokio::fs::remove_dir_all(&root).await {
            tracing::warn!(%error, path=%root.display(), "failed to clean transient Stack files");
        }
        result
    }

    async fn snapshot(
        &self,
        platform_id: Uuid,
        project: &str,
        orchestration: StackOrchestrationMode,
    ) -> Result<StackRuntimeSnapshot, StackError> {
        match orchestration {
            StackOrchestrationMode::DockerCompose => {
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

    async fn delete_compose_networks<T>(
        &self,
        transport: &T,
        project: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), StackError>
    where
        T: PlatformInventoryPort + PlatformResourceMutationPort,
    {
        let networks = transport
            .list_networks(cancellation)
            .await
            .map_err(agent_error)?;
        for network in networks.into_iter().filter(|network| {
            network
                .labels
                .get("com.docker.compose.project")
                .is_some_and(|value| value == project)
        }) {
            transport
                .delete_network(&network.id, cancellation)
                .await
                .map_err(agent_error)?;
        }
        Ok(())
    }
}

impl StackRuntimePort for StackRuntimeRouter {
    fn apply<'a>(
        &'a self,
        claim: &'a StackOperationClaim,
        compose: &'a str,
        environment: &'a [String],
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<StackRuntimeResult, StackError>> {
        async move {
            let target = self.platform(claim.platform_id).await?;
            if target.connector.eq_ignore_ascii_case("Local") {
                self.apply_local(claim, compose, environment, cancellation).await
            } else if target.connector.eq_ignore_ascii_case("Agent") {
                self.agent_for(&target)?.apply_stack(claim, compose, environment, cancellation).await.map_err(agent_error)
            } else {
                Err(StackError::Runtime("Edge Agent Stack mutations are not available until the inbound command transport migrates.".to_owned()))
            }
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
                    orchestration(&claim.platform_type)?,
                )
                .await?;
            let status = if claim.platform_type == "DockerSwarm" {
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
                    .map(|message| StackStreamItem::system(message.clone()))
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
            if claim.platform_type == "DockerSwarm"
                && target.connector.eq_ignore_ascii_case("Local")
            {
                let args = vec![
                    "stack".to_owned(),
                    "rm".to_owned(),
                    claim.project_name.clone(),
                ];
                let result = run_docker(&args, &std::env::temp_dir(), &[], cancellation).await?;
                return if result.status == StackReleaseStatus::Healthy {
                    Ok(())
                } else {
                    Err(StackError::RuntimeRejected(last_message(&result)))
                };
            }

            if claim.platform_type == "DockerSwarm" {
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
            if target.connector.eq_ignore_ascii_case("Local") {
                for id in container_ids {
                    self.docker
                        .delete_container(&id, false, true)
                        .await
                        .map_err(runtime_io)?;
                }
                self.delete_compose_networks(&self.docker, &claim.project_name, cancellation)
                    .await
            } else {
                let agent = self.agent_for(&target)?;
                for id in container_ids {
                    agent
                        .delete_container(&id, cancellation)
                        .await
                        .map_err(agent_error)?;
                }
                self.delete_compose_networks(agent, &claim.project_name, cancellation)
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
            if target.connector.eq_ignore_ascii_case("Local") {
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
                let result = run_docker(&args, &std::env::temp_dir(), &[], cancellation).await?;
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
                        let result = if target.connector.eq_ignore_ascii_case("Local") {
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
                    let result = if target.connector.eq_ignore_ascii_case("Local") {
                        let command = if agent_action == AgentContainerAction::Start {
                            "start"
                        } else {
                            "unpause"
                        };
                        run_docker(
                            &[command.to_owned(), container_id.clone()],
                            &std::env::temp_dir(),
                            &[],
                            cancellation,
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
        _cancellation: &'a CancellationToken,
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
            let (service_names, container_ids, container_names, services) = if import_kind == StackImportKind::SwarmStack {
                let rows=sqlx::query("SELECT dockerserviceid,name,image,desiredtaskcount,runningtaskcount,updatestate FROM swarmserviceprojections WHERE platformid=$1 AND dockerstacknamespace=$2 AND NOT isstale ORDER BY name")
                    .bind(platform_id).bind(project_name).fetch_all(&self.pool).await.map_err(storage)?;
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
                let rows=sqlx::query("SELECT dockercontainerid,name,dockerimageid,state FROM containers WHERE platformid=$1 AND stack=$2 AND NOT isswarmtask ORDER BY name,dockercontainerid")
                    .bind(platform_id).bind(project_name).fetch_all(&self.pool).await.map_err(storage)?;
                let ids=rows.iter().map(|row|row.try_get("dockercontainerid").map_err(storage)).collect::<Result<Vec<String>,_>>()?;
                let container_names=rows.iter().map(|row|row.try_get("name").map_err(storage)).collect::<Result<Vec<String>,_>>()?;
                let mut grouped=std::collections::BTreeMap::<String,(Option<String>,Vec<String>)>::new();
                for row in rows {
                    let name:String=row.try_get("name").map_err(storage)?;
                    let entry=grouped.entry(name.trim_start_matches('/').to_owned()).or_default();
                    entry.0=Some(row.try_get("dockerimageid").map_err(storage)?);
                    entry.1.push(row.try_get("state").map_err(storage)?);
                }
                let services=grouped.into_iter().map(|(name,(image,states))|ComposeProjectRuntimeService{name, image, container_count:states.len(), states}).collect::<Vec<_>>();
                let names=services.iter().map(|service|service.name.clone()).collect();
                (names,ids,container_names,services)
            };
            if service_names.is_empty() { return Err(StackError::NotFound); }
            let mut hash = Sha256::new(); hash.update(platform_id.as_bytes()); hash.update(project_name.as_bytes());
            for service in &services {
                hash.update(service.name.as_bytes()); hash.update([0]);
                hash.update(service.image.as_deref().unwrap_or_default().as_bytes()); hash.update([0]);
                hash.update(service.container_count.to_le_bytes());
                for state in &service.states { hash.update(state.as_bytes()); hash.update([0]); }
            }
            for id in &container_ids { hash.update(id.as_bytes()); hash.update([0]); }
            for name in &container_names { hash.update(name.as_bytes()); hash.update([0]); }
            let fingerprint = hash.finalize().iter().map(|byte| format!("{byte:02x}")).collect::<String>();
            Ok(StackImportClaim { platform_id, platform_name: platform.try_get("name").map_err(storage)?, project_name: project_name.to_owned(), import_kind, runtime_fingerprint: format!("sha256:{fingerprint}"), service_names, container_ids, container_names, services })
        }.boxed()
    }
}

fn temporary_run_root(stack_id: Uuid) -> PathBuf {
    std::env::temp_dir()
        .join("citadel-stack-runs")
        .join(format!("{}-{}", stack_id.simple(), Uuid::now_v7().simple()))
}

fn compose_args(compose: &Path, env: &Path, project: &str, tail: &[&str]) -> Vec<String> {
    let mut args = vec![
        "compose".to_owned(),
        "-p".to_owned(),
        project.to_owned(),
        "-f".to_owned(),
        compose.display().to_string(),
    ];
    if env.exists() {
        args.extend(["--env-file".to_owned(), env.display().to_string()]);
    }
    args.extend(tail.iter().map(|value| (*value).to_owned()));
    args
}

async fn run_docker(
    args: &[String],
    directory: &Path,
    environment: &[String],
    cancellation: &CancellationToken,
) -> Result<StackRuntimeResult, StackError> {
    let mut command = Command::new("docker");
    command
        .args(args)
        .current_dir(directory)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    for entry in environment {
        if let Some((name, value)) = entry.split_once('=') {
            command.env(name, value);
        }
    }
    let mut child = command.spawn().map_err(runtime_io)?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| StackError::Runtime("Docker stdout was not captured.".to_owned()))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| StackError::Runtime("Docker stderr was not captured.".to_owned()))?;
    let stdout_task = tokio::spawn(read_bounded(stdout));
    let stderr_task = tokio::spawn(read_bounded(stderr));
    let status = tokio::select! { biased; () = cancellation.cancelled() => { let _=child.kill().await; return Err(StackError::Cancelled); }, value=child.wait()=>value.map_err(runtime_io)? };
    let stdout = stdout_task
        .await
        .map_err(|error| StackError::Runtime(error.to_string()))??;
    let stderr = stderr_task
        .await
        .map_err(|error| StackError::Runtime(error.to_string()))??;
    let mut messages = lines(stdout, StackApplyEventType::StdOut);
    messages.extend(lines(stderr, StackApplyEventType::StdErr));
    messages.push(StackStreamItem {
        event_type: StackApplyEventType::CommandCompleted,
        message: None,
        exit_code: status.code(),
        stack_status: None,
        severity: None,
    });
    Ok(StackRuntimeResult {
        status: if status.success() {
            StackReleaseStatus::Healthy
        } else {
            StackReleaseStatus::Failed
        },
        messages,
    })
}

async fn read_bounded(
    mut reader: impl tokio::io::AsyncRead + Unpin,
) -> Result<Vec<u8>, StackError> {
    let mut output = Vec::new();
    let mut chunk = [0_u8; 8192];
    loop {
        let count = reader.read(&mut chunk).await.map_err(runtime_io)?;
        if count == 0 {
            break;
        }
        let remaining = MAX_PROCESS_OUTPUT_BYTES.saturating_sub(output.len());
        output.extend_from_slice(&chunk[..count.min(remaining)]);
    }
    Ok(output)
}

fn lines(bytes: Vec<u8>, event_type: StackApplyEventType) -> Vec<StackStreamItem> {
    String::from_utf8_lossy(&bytes)
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| StackStreamItem {
            event_type,
            message: Some(line.to_owned()),
            exit_code: None,
            stack_status: None,
            severity: None,
        })
        .collect()
}
fn last_message(result: &StackRuntimeResult) -> String {
    result
        .messages
        .iter()
        .rev()
        .find_map(|item| item.message.clone())
        .unwrap_or_else(|| "Docker rejected the Stack operation.".to_owned())
}
fn orchestration(value: &str) -> Result<StackOrchestrationMode, StackError> {
    match value {
        "Docker" => Ok(StackOrchestrationMode::DockerCompose),
        "DockerSwarm" => Ok(StackOrchestrationMode::DockerSwarm),
        _ => Err(StackError::Validation(
            "Stacks require a Docker or Docker Swarm Platform.".to_owned(),
        )),
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
    connector: String,
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
