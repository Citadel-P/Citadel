use citadel_contracts::citadel::swarm::v1::{
    CreateManagedSwarmServiceRequest, DeleteManagedSwarmServiceRequest,
    SwarmConfigReferenceSpecMessage, SwarmHealthCheckSpecMessage, SwarmMountSpecMessage,
    SwarmPortSpecMessage, SwarmResourceSpecMessage, SwarmRestartPolicySpecMessage,
    SwarmSecretReferenceSpecMessage, SwarmServiceMessage, SwarmServiceMutationSpecMessage,
    SwarmUpdatePolicySpecMessage, UpdateManagedSwarmServiceRequest,
};
use citadel_swarm_services::{
    MountKind, PortPublishMode, RuntimeServiceResult, SchedulingMode, ServiceOperationClaim,
    ServiceOperationKind, SwarmServiceError, SwarmServiceRuntime, SwarmServiceSpec,
};
use futures_util::future::BoxFuture;
use serde_json::{Map, Value, json};
use sqlx::{PgPool, Row};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::agent::AgentClient;
use crate::docker::{DockerClient, DockerError};

const MANAGED_LABEL: &str = "com.citadel.managed";
mod updates;
const SERVICE_LABEL: &str = "com.citadel.service-id";
const OPERATION_LABEL: &str = "com.citadel.operation-id";

#[derive(Clone)]
pub struct SwarmServiceRuntimeRouter {
    pool: PgPool,
    docker: DockerClient,
    agent: Option<AgentClient>,
    edge: crate::edge::EdgeRegistry,
    image_cache: std::sync::Arc<crate::image_digest_cache::ImageDigestCache>,
}

impl SwarmServiceRuntimeRouter {
    pub async fn inspect_service(
        &self,
        platform: Uuid,
        id: &str,
        cancel: &CancellationToken,
    ) -> Result<SwarmServiceMessage, SwarmServiceError> {
        let work = async {
            use citadel_platforms::PlatformRuntimePort;
            let pinned: (Option<String>, Value) =
                sqlx::query_as("SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1")
                    .bind(platform)
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(storage)?
                    .ok_or(SwarmServiceError::NotFound)?;
            let check = |info: &citadel_platforms::RuntimePlatformInfo| {
                if citadel_platforms::swarm_mutations::manager_matches(
                    info,
                    pinned.0.as_deref(),
                    &pinned.1,
                ) {
                    Ok(())
                } else {
                    Err(SwarmServiceError::Conflict(
                        "The connected Docker manager no longer belongs to this Swarm platform."
                            .into(),
                    ))
                }
            };
            if self.connector(platform).await? == "Local" {
                check(
                    &self
                        .docker
                        .get_info(cancel)
                        .await
                        .map_err(inspection_error)?,
                )?;
                crate::swarm_inventory::SwarmInventoryClient::Local(&self.docker)
                    .inspect_service(id, cancel)
                    .await
                    .map_err(inspection_error)
            } else {
                let agent = self.agent_for(platform).await?;
                let info = match &agent {
                    crate::agent_execution::AgentExecutionClient::Direct(client) => {
                        client.get_info(cancel).await
                    }
                    crate::agent_execution::AgentExecutionClient::Edge(session) => {
                        crate::edge::EdgeRuntime {
                            session: session.clone(),
                        }
                        .get_info(cancel)
                        .await
                    }
                }
                .map_err(inspection_error)?;
                check(&info)?;
                agent
                    .inspect_managed_swarm_service(id, cancel)
                    .await
                    .map_err(inspection_error)
            }
        };
        tokio::select! { biased;
            () = cancel.cancelled() => Err(SwarmServiceError::Cancelled),
            result = tokio::time::timeout(std::time::Duration::from_secs(30),work) =>
                result.unwrap_or_else(|_| Err(SwarmServiceError::Runtime("Service inspection timed out.".into()))),
        }
    }
    #[must_use]
    pub fn new(pool: PgPool, docker: DockerClient, agent: Option<AgentClient>) -> Self {
        Self {
            pool,
            docker,
            agent,
            edge: crate::edge::EdgeRegistry::default(),
            image_cache: Default::default(),
        }
    }

    pub fn with_image_cache(
        mut self,
        cache: std::sync::Arc<crate::image_digest_cache::ImageDigestCache>,
    ) -> Self {
        self.image_cache = cache;
        self
    }

    #[must_use]
    pub fn with_edge(mut self, edge: crate::edge::EdgeRegistry) -> Self {
        self.edge = edge;
        self
    }

    async fn agent_for(
        &self,
        platform_id: Uuid,
    ) -> Result<crate::agent_execution::AgentExecutionClient, SwarmServiceError> {
        use crate::{agent_execution::AgentExecutionClient, edge::EdgeTarget};
        let row = sqlx::query("SELECT connectortype,address FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(SwarmServiceError::NotFound)?;
        let connector: String = row.try_get("connectortype").map_err(storage)?;
        let address: String = row.try_get("address").map_err(storage)?;
        if connector == "EdgeAgent" {
            return self
                .edge
                .get(&EdgeTarget::platform(platform_id))
                .map(AgentExecutionClient::Edge)
                .map_err(|_| {
                    SwarmServiceError::Runtime(
                        "The Edge Agent is disconnected or unavailable.".into(),
                    )
                });
        }
        let agent = self
            .agent
            .as_ref()
            .filter(|_| connector == "Agent")
            .ok_or_else(|| {
                SwarmServiceError::Runtime("The configured Agent transport is unavailable.".into())
            })?;
        let agent = agent
            .at_address(&address)
            .map_err(|error| SwarmServiceError::Runtime(error.message))?;
        Ok(AgentExecutionClient::Direct(std::sync::Arc::new(agent)))
    }

    async fn connector(&self, platform_id: Uuid) -> Result<String, SwarmServiceError> {
        let row = sqlx::query("SELECT connectortype,status FROM platforms WHERE id=$1")
            .bind(platform_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?
            .ok_or(SwarmServiceError::NotFound)?;
        if row.try_get::<String, _>("status").map_err(storage)? != "Online" {
            return Err(SwarmServiceError::Conflict(
                "The Docker Swarm manager is not available.".to_owned(),
            ));
        }
        row.try_get("connectortype").map_err(storage)
    }

    async fn record_target(
        &self,
        claim: &ServiceOperationClaim,
        spec: &Value,
    ) -> Result<String, SwarmServiceError> {
        let hash = crate::swarm_service_inspection::runtime_hash(spec);
        let changed = sqlx::query("UPDATE swarmservices SET targetruntimehash=$3,expectedforceupdate=$4 WHERE id=$1 AND operationid=$2 AND operationstate='PendingAcceptance'")
            .bind(claim.id).bind(claim.operation_id).bind(&hash)
            .bind(spec.pointer("/TaskTemplate/ForceUpdate").and_then(Value::as_i64))
            .execute(&self.pool).await.map_err(storage)?.rows_affected();
        if changed != 1 {
            return Err(SwarmServiceError::Conflict(
                "The operation changed before dispatch.".into(),
            ));
        }
        Ok(hash)
    }

    async fn mutate_local(
        &self,
        claim: &ServiceOperationClaim,
        _kind: ServiceOperationKind,
        force_increment: i64,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeServiceResult, SwarmServiceError> {
        let existing=match &claim.docker_service_id {
            Some(id)=>Some(tokio::select!{biased;()=cancellation.cancelled()=>return Err(SwarmServiceError::Cancelled),result=self.docker.inspect_swarm_service(id)=>result}.map_err(runtime)?),
            None=>None,
        };
        let image = claim.spec.image.source_reference().ok_or_else(|| {
            SwarmServiceError::Validation("The Service image has not been resolved.".to_owned())
        })?;
        let mut spec = docker_spec(
            &claim.spec,
            &claim.docker_name,
            claim.id,
            claim.operation_id,
            image,
            force_increment,
            existing.as_ref().map(|value| &value.spec),
        );
        let target_hash = self.record_target(claim, &spec).await?;
        let response = if let Some(existing) = &existing {
            tokio::select!{biased;()=cancellation.cancelled()=>return Err(SwarmServiceError::Cancelled),result=self.docker.update_swarm_service(&existing.id,i64::try_from(existing.version.index).unwrap_or(i64::MAX),&spec)=>result}.map_err(runtime)?
        } else {
            tokio::select!{biased;()=cancellation.cancelled()=>return Err(SwarmServiceError::Cancelled),result=self.docker.create_swarm_service(&spec)=>result}.map_err(runtime)?
        };
        // Drop the request body, which can contain resolved secret values, before observation.
        spec = Value::Null;
        drop(spec);
        let service_id = response
            .get("ID")
            .or_else(|| response.get("Id"))
            .and_then(Value::as_str)
            .map(str::to_owned)
            .or_else(|| claim.docker_service_id.clone())
            .ok_or_else(|| {
                SwarmServiceError::Runtime(
                    "Docker did not return an ID for the Service mutation.".to_owned(),
                )
            })?;
        let warnings = response
            .get("Warnings")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect();
        let observed=tokio::select!{biased;()=cancellation.cancelled()=>return Err(SwarmServiceError::Cancelled),result=self.docker.inspect_swarm_service(&service_id)=>result}.map_err(runtime)?;
        let mut result = observed_result(observed, warnings, true);
        result.rollout_complete &= result.runtime_hash == target_hash;
        Ok(result)
    }

    async fn mutate_agent(
        &self,
        claim: &ServiceOperationClaim,
        force_increment: i32,
        cancellation: &CancellationToken,
    ) -> Result<RuntimeServiceResult, SwarmServiceError> {
        let agent = self.agent_for(claim.platform_id).await?;
        let image = claim.spec.image.source_reference().ok_or_else(|| {
            SwarmServiceError::Validation("The Service image has not been resolved.".to_owned())
        })?;
        let current = if let Some(id) = &claim.docker_service_id {
            let service = self
                .inspect_service(claim.platform_id, id, cancellation)
                .await?;
            Some(json!({"TaskTemplate":{"ForceUpdate":service.force_update}}))
        } else {
            None
        };
        let target_spec = docker_spec(
            &claim.spec,
            &claim.docker_name,
            claim.id,
            claim.operation_id,
            image,
            i64::from(force_increment),
            current.as_ref(),
        );
        let target_hash = self.record_target(claim, &target_spec).await?;
        let mut labels = std::collections::HashMap::from([
            (MANAGED_LABEL.to_owned(), "true".to_owned()),
            (SERVICE_LABEL.to_owned(), claim.id.to_string()),
            (OPERATION_LABEL.to_owned(), claim.operation_id.to_string()),
        ]);
        labels.extend(claim.spec.labels.clone());
        let spec = proto_spec(&claim.spec, image, force_increment);
        let response = if let Some(service_id) = &claim.docker_service_id {
            agent
                .update_managed_swarm_service(
                    UpdateManagedSwarmServiceRequest {
                        operation_id: claim.operation_id.to_string(),
                        service_id: service_id.clone(),
                        version_index: claim
                            .docker_version_index
                            .and_then(|value| u64::try_from(value).ok())
                            .unwrap_or_default(),
                        spec: Some(spec),
                        labels,
                        registry_auth: String::new(),
                    },
                    cancellation,
                )
                .await
        } else {
            agent
                .create_managed_swarm_service(
                    CreateManagedSwarmServiceRequest {
                        operation_id: claim.operation_id.to_string(),
                        docker_name: claim.docker_name.clone(),
                        spec: Some(spec),
                        labels,
                        registry_auth: String::new(),
                    },
                    cancellation,
                )
                .await
        }
        .map_err(agent_runtime)?;
        let observed = agent
            .inspect_managed_swarm_service(&response.service_id, cancellation)
            .await
            .map_err(agent_runtime)?;
        Ok(observed_agent_result(
            observed,
            target_hash,
            response.warnings,
            true,
        ))
    }
}

impl SwarmServiceRuntime for SwarmServiceRuntimeRouter {
    fn apply<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move {
            match self.connector(claim.platform_id).await?.as_str() {
                "Local" => {
                    self.mutate_local(claim, ServiceOperationKind::Apply, 0, cancellation)
                        .await
                }
                "Agent" | "EdgeAgent" => self.mutate_agent(claim, 0, cancellation).await,
                _ => Err(edge_unavailable()),
            }
        })
    }
    fn scale<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        _replicas: i32,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move {
            match self.connector(claim.platform_id).await?.as_str() {
                "Local" => {
                    self.mutate_local(claim, ServiceOperationKind::Scale, 0, cancellation)
                        .await
                }
                "Agent" | "EdgeAgent" => self.mutate_agent(claim, 0, cancellation).await,
                _ => Err(edge_unavailable()),
            }
        })
    }
    fn force_update<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeServiceResult, SwarmServiceError>> {
        Box::pin(async move {
            match self.connector(claim.platform_id).await?.as_str() {
                "Local" => {
                    self.mutate_local(claim, ServiceOperationKind::ForceUpdate, 1, cancellation)
                        .await
                }
                "Agent" | "EdgeAgent" => self.mutate_agent(claim, 1, cancellation).await,
                _ => Err(edge_unavailable()),
            }
        })
    }
    fn delete<'a>(
        &'a self,
        platform_id: Uuid,
        docker_service_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), SwarmServiceError>> {
        Box::pin(async move {
            match self.connector(platform_id).await?.as_str() {
                "Local" => {
                    let result = tokio::select! {biased;()=cancellation.cancelled()=>return Err(SwarmServiceError::Cancelled),result=self.docker.delete_swarm_service(docker_service_id)=>result};
                    match result {
                        Ok(()) => Ok(()),
                        Err(DockerError::Api { status, .. }) if status.as_u16() == 404 => Ok(()),
                        Err(error) => Err(runtime(error)),
                    }
                }
                "Agent" | "EdgeAgent" => {
                    let agent = self.agent_for(platform_id).await?;
                    agent
                        .delete_managed_swarm_service(
                            DeleteManagedSwarmServiceRequest {
                                operation_id: Uuid::now_v7().to_string(),
                                service_id: docker_service_id.to_owned(),
                            },
                            cancellation,
                        )
                        .await
                        .map_err(agent_runtime)
                }
                _ => Err(edge_unavailable()),
            }
        })
    }
    fn observe<'a>(
        &'a self,
        claim: &'a ServiceOperationClaim,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<RuntimeServiceResult>, SwarmServiceError>> {
        Box::pin(async move {
            let Some(id) = &claim.docker_service_id else {
                return Ok(None);
            };
            let observed = self
                .inspect_service(claim.platform_id, id, cancellation)
                .await?;
            let expected: Option<(Option<String>, Option<i64>, Option<i64>)> = sqlx::query_as("SELECT targetruntimehash,basedockerversion,expectedforceupdate FROM swarmservices WHERE id=$1 AND operationid=$2 AND operationstate IN ('PendingAcceptance','Accepted')")
                .bind(claim.id).bind(claim.operation_id).fetch_optional(&self.pool).await.map_err(storage)?;
            let Some((Some(hash), base, force)) = expected else {
                return Ok(None);
            };
            let accepted = observed
                .labels
                .get(OPERATION_LABEL)
                .and_then(|id| Uuid::parse_str(id).ok())
                == Some(claim.operation_id)
                && base.is_none_or(|base| observed.version_index > base as u64)
                && force.is_none_or(|force| observed.force_update >= force);
            if !accepted || hash != observed.runtime_hash {
                return Ok(None);
            }
            Ok(Some(observed_agent_result(observed, hash, vec![], true)))
        })
    }
}

fn docker_spec(
    spec: &SwarmServiceSpec,
    docker_name: &str,
    service_id: Uuid,
    operation_id: Uuid,
    image: &str,
    force_increment: i64,
    current: Option<&Value>,
) -> Value {
    let mut labels = spec.labels.clone();
    labels.insert(MANAGED_LABEL.to_owned(), "true".to_owned());
    labels.insert(SERVICE_LABEL.to_owned(), service_id.to_string());
    labels.insert(OPERATION_LABEL.to_owned(), operation_id.to_string());
    let current_force = current
        .and_then(|value| value.pointer("/TaskTemplate/ForceUpdate"))
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let container = json!({"Image":image,"Command":spec.command,"Args":spec.arguments,"Env":spec.environment,"User":spec.user,"Dir":spec.working_directory,"StopGracePeriod":spec.stop_grace_period_nanoseconds,
        "Healthcheck":spec.health_check.as_ref().map(|value|json!({"Test":value.test,"Interval":value.interval_nanoseconds,"Timeout":value.timeout_nanoseconds,"Retries":value.retries,"StartPeriod":value.start_period_nanoseconds})),
        "Mounts":spec.mounts.iter().map(|value|json!({"Type":match value.kind{MountKind::Bind=>"bind",MountKind::Tmpfs=>"tmpfs",MountKind::Volume=>"volume"},"Source":value.source,"Target":value.target,"ReadOnly":value.read_only})).collect::<Vec<_>>(),
        "Secrets":spec.secrets.iter().map(|value|json!({"SecretID":value.secret_id,"SecretName":value.secret_name,"File":{"Name":value.target_name,"UID":"0","GID":"0","Mode":292}})).collect::<Vec<_>>(),
        "Configs":spec.configs.iter().map(|value|json!({"ConfigID":value.config_id,"ConfigName":value.config_name,"File":{"Name":value.target_name,"UID":"0","GID":"0","Mode":292}})).collect::<Vec<_>>()});
    let resources=spec.resources.as_ref().map(|value|json!({"Limits":{"NanoCPUs":value.limit_nano_cpus,"MemoryBytes":value.limit_memory_bytes},"Reservations":{"NanoCPUs":value.reservation_nano_cpus,"MemoryBytes":value.reservation_memory_bytes}}));
    let restart=spec.restart_policy.as_ref().map(|value|json!({"Condition":match value.condition{citadel_swarm_services::RestartCondition::None=>"none",citadel_swarm_services::RestartCondition::OnFailure=>"on-failure",citadel_swarm_services::RestartCondition::Any=>"any"},"Delay":value.delay_nanoseconds,"MaxAttempts":value.maximum_attempts,"Window":value.window_nanoseconds}));
    let task = json!({"ContainerSpec":container,"ForceUpdate":current_force+force_increment,"Networks":spec.network_ids.iter().map(|id|json!({"Target":id})).collect::<Vec<_>>(),"Resources":resources,"Placement":(!spec.placement_constraints.is_empty()).then(||json!({"Constraints":spec.placement_constraints})),"RestartPolicy":restart});
    let mode = match spec.scheduling_mode {
        SchedulingMode::Global => json!({"Global":{}}),
        SchedulingMode::Replicated => json!({"Replicated":{"Replicas":spec.replicas}}),
    };
    let update=spec.update_policy.as_ref().map(|value|json!({"Parallelism":value.parallelism,"Delay":value.delay_nanoseconds,"Order":match value.order{citadel_swarm_services::UpdateOrder::StartFirst=>"start-first",_=>"stop-first"},"FailureAction":match value.failure_action{citadel_swarm_services::UpdateFailureAction::Continue=>"continue",citadel_swarm_services::UpdateFailureAction::Rollback=>"rollback",_=>"pause"}}));
    let endpoint = json!({"Mode":"vip","Ports":spec.ports.iter().map(|value|json!({"TargetPort":value.target_port,"PublishedPort":value.published_port,"Protocol":value.protocol.to_ascii_lowercase(),"PublishMode":match value.publish_mode{PortPublishMode::Host=>"host",PortPublishMode::Ingress=>"ingress"}})).collect::<Vec<_>>()});
    let mut result = Map::new();
    result.insert("Name".to_owned(), json!(docker_name));
    result.insert("Labels".to_owned(), json!(labels));
    result.insert("TaskTemplate".to_owned(), task);
    result.insert("Mode".to_owned(), mode);
    result.insert("UpdateConfig".to_owned(), json!(update));
    result.insert("EndpointSpec".to_owned(), endpoint);
    if let Some(value) = current.and_then(|value| value.get("RollbackConfig")) {
        result.insert("RollbackConfig".to_owned(), value.clone());
    }
    Value::Object(result)
}

fn proto_spec(
    spec: &SwarmServiceSpec,
    image: &str,
    force_update: i32,
) -> SwarmServiceMutationSpecMessage {
    SwarmServiceMutationSpecMessage {
        image: image.to_owned(),
        scheduling_mode: match spec.scheduling_mode {
            SchedulingMode::Replicated => "Replicated",
            SchedulingMode::Global => "Global",
        }
        .to_owned(),
        replicas: spec.replicas,
        command: spec.command.clone(),
        arguments: spec.arguments.clone(),
        environment: spec.environment.clone(),
        user: spec.user.clone().unwrap_or_default(),
        working_directory: spec.working_directory.clone().unwrap_or_default(),
        health_check: spec
            .health_check
            .as_ref()
            .map(|value| SwarmHealthCheckSpecMessage {
                test: value.test.clone(),
                interval_nanoseconds: value.interval_nanoseconds,
                timeout_nanoseconds: value.timeout_nanoseconds,
                retries: value.retries,
                start_period_nanoseconds: value.start_period_nanoseconds,
            }),
        stop_grace_period_nanoseconds: spec.stop_grace_period_nanoseconds,
        ports: spec
            .ports
            .iter()
            .map(|value| SwarmPortSpecMessage {
                target_port: value.target_port,
                published_port: value.published_port,
                protocol: value.protocol.clone(),
                publish_mode: match value.publish_mode {
                    PortPublishMode::Ingress => "Ingress",
                    PortPublishMode::Host => "Host",
                }
                .to_owned(),
            })
            .collect(),
        network_ids: spec.network_ids.clone(),
        mounts: spec
            .mounts
            .iter()
            .map(|value| SwarmMountSpecMessage {
                kind: match value.kind {
                    MountKind::Volume => "Volume",
                    MountKind::Bind => "Bind",
                    MountKind::Tmpfs => "Tmpfs",
                }
                .to_owned(),
                source: value.source.clone(),
                target: value.target.clone(),
                read_only: value.read_only,
            })
            .collect(),
        secrets: spec
            .secrets
            .iter()
            .map(|value| SwarmSecretReferenceSpecMessage {
                id: value.secret_id.clone(),
                name: value.secret_name.clone(),
                target_name: value.target_name.clone(),
            })
            .collect(),
        configs: spec
            .configs
            .iter()
            .map(|value| SwarmConfigReferenceSpecMessage {
                id: value.config_id.clone(),
                name: value.config_name.clone(),
                target_name: value.target_name.clone(),
            })
            .collect(),
        resources: spec
            .resources
            .as_ref()
            .map(|value| SwarmResourceSpecMessage {
                limit_nano_cpus: value.limit_nano_cpus,
                limit_memory_bytes: value.limit_memory_bytes,
                reservation_nano_cpus: value.reservation_nano_cpus,
                reservation_memory_bytes: value.reservation_memory_bytes,
            }),
        placement_constraints: spec.placement_constraints.clone(),
        restart_policy: spec
            .restart_policy
            .as_ref()
            .map(|value| SwarmRestartPolicySpecMessage {
                condition: format!("{:?}", value.condition),
                delay_nanoseconds: value.delay_nanoseconds,
                maximum_attempts: value.maximum_attempts,
                window_nanoseconds: value.window_nanoseconds,
            }),
        update_policy: spec
            .update_policy
            .as_ref()
            .map(|value| SwarmUpdatePolicySpecMessage {
                parallelism: value.parallelism,
                delay_nanoseconds: value.delay_nanoseconds,
                order: format!("{:?}", value.order),
                failure_action: format!("{:?}", value.failure_action),
            }),
        force_update,
    }
}

fn observed_result(
    service: crate::docker::projection::SwarmService,
    warnings: Vec<String>,
    accepted: bool,
) -> RuntimeServiceResult {
    let update_state = service
        .update_status
        .get("State")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let rollout_error = matches!(
        update_state.to_ascii_lowercase().as_str(),
        "paused" | "rollback_paused" | "rollback_completed"
    )
    .then(|| {
        service
            .update_status
            .get("Message")
            .and_then(Value::as_str)
            .unwrap_or("Docker paused the Service rollout.")
            .to_owned()
    });
    let desired = service
        .service_status
        .get("DesiredTasks")
        .and_then(Value::as_i64)
        .or_else(|| {
            service
                .spec
                .pointer("/Mode/Replicated/Replicas")
                .and_then(Value::as_i64)
        })
        .unwrap_or(0);
    let running = service
        .service_status
        .get("RunningTasks")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    RuntimeServiceResult {
        docker_service_id: service.id,
        version_index: i64::try_from(service.version.index).unwrap_or(i64::MAX),
        accepted,
        rollout_complete: rollout_error.is_none()
            && citadel_platforms::jobs::rollout_complete_state(update_state)
            && running >= desired,
        rollout_error,
        runtime_hash: crate::swarm_service_inspection::runtime_hash(&service.spec),
        applied_digest: service
            .spec
            .pointer("/TaskTemplate/ContainerSpec/Image")
            .and_then(Value::as_str)
            .and_then(extract_digest),
        warnings,
    }
}
fn observed_agent_result(
    service: SwarmServiceMessage,
    runtime_hash: String,
    warnings: Vec<String>,
    accepted: bool,
) -> RuntimeServiceResult {
    let rollout_error = matches!(
        service.update_state.to_ascii_lowercase().as_str(),
        "paused" | "rollback_paused" | "rollback_completed"
    )
    .then(|| {
        if service.update_message.is_empty() {
            "Docker paused the Service rollout.".to_owned()
        } else {
            service.update_message
        }
    });
    RuntimeServiceResult {
        docker_service_id: service.id,
        version_index: i64::try_from(service.version_index).unwrap_or(i64::MAX),
        accepted,
        rollout_complete: rollout_error.is_none()
            && citadel_platforms::jobs::rollout_complete_state(&service.update_state)
            && service.running_task_count >= service.desired_task_count
            && !runtime_hash.is_empty()
            && service.runtime_hash == runtime_hash,
        rollout_error,
        runtime_hash: service.runtime_hash,
        applied_digest: extract_digest(&service.image),
        warnings,
    }
}
fn extract_digest(value: &str) -> Option<String> {
    value
        .split_once('@')
        .map(|(_, digest)| digest.to_owned())
        .filter(|value| value.starts_with("sha256:"))
}
fn edge_unavailable() -> SwarmServiceError {
    SwarmServiceError::Runtime(
        "Edge Agent mutations are not available during this migration phase.".to_owned(),
    )
}
fn inspection_error(error: citadel_platforms::RuntimeCapabilityError) -> SwarmServiceError {
    use citadel_platforms::RuntimeErrorKind;
    match error.kind {
        RuntimeErrorKind::NotFound => SwarmServiceError::NotFound,
        RuntimeErrorKind::Conflict => SwarmServiceError::Conflict(error.message),
        RuntimeErrorKind::InvalidRequest => SwarmServiceError::Validation(error.message),
        RuntimeErrorKind::PermissionDenied => SwarmServiceError::Forbidden,
        RuntimeErrorKind::Cancelled => SwarmServiceError::Cancelled,
        _ => SwarmServiceError::Runtime(error.message),
    }
}
fn agent_runtime(error: citadel_platforms::RuntimeCapabilityError) -> SwarmServiceError {
    if matches!(
        error.kind,
        citadel_platforms::RuntimeErrorKind::InvalidRequest
            | citadel_platforms::RuntimeErrorKind::Conflict
            | citadel_platforms::RuntimeErrorKind::Authentication
    ) {
        SwarmServiceError::RuntimeRejected(error.to_string())
    } else {
        SwarmServiceError::Runtime(error.to_string())
    }
}
fn runtime(error: DockerError) -> SwarmServiceError {
    match error {
        DockerError::Api { status, message } if status.is_client_error() => {
            SwarmServiceError::RuntimeRejected(format!("Docker returned HTTP {status}: {message}"))
        }
        error => SwarmServiceError::Runtime(error.to_string()),
    }
}
fn storage(error: impl std::fmt::Display) -> SwarmServiceError {
    SwarmServiceError::Storage(error.to_string())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use citadel_swarm_services::{
        RestartCondition, SwarmServiceConfigReference, SwarmServiceImageInfo,
        SwarmServiceRestartPolicy, SwarmServiceSecretReference, UpdateBehavior,
    };

    use super::*;

    #[test]
    fn docker_spec_uses_valid_file_defaults_and_restart_condition() {
        let mut spec = minimal_spec();
        spec.secrets.push(SwarmServiceSecretReference {
            secret_id: "secret-id".to_owned(),
            secret_name: "secret".to_owned(),
            target_name: "secret.txt".to_owned(),
        });
        spec.configs.push(SwarmServiceConfigReference {
            config_id: "config-id".to_owned(),
            config_name: "config".to_owned(),
            target_name: "config.txt".to_owned(),
        });
        spec.restart_policy = Some(SwarmServiceRestartPolicy {
            condition: RestartCondition::OnFailure,
            delay_nanoseconds: None,
            maximum_attempts: None,
            window_nanoseconds: None,
        });

        let payload = docker_spec(
            &spec,
            "fixture",
            Uuid::now_v7(),
            Uuid::now_v7(),
            "redis:7-alpine",
            0,
            None,
        );

        assert_eq!(
            payload.pointer("/TaskTemplate/RestartPolicy/Condition"),
            Some(&json!("on-failure"))
        );
        for path in [
            "/TaskTemplate/ContainerSpec/Secrets/0/File",
            "/TaskTemplate/ContainerSpec/Configs/0/File",
        ] {
            assert_eq!(payload.pointer(&format!("{path}/UID")), Some(&json!("0")));
            assert_eq!(payload.pointer(&format!("{path}/GID")), Some(&json!("0")));
            assert_eq!(payload.pointer(&format!("{path}/Mode")), Some(&json!(292)));
        }
    }

    fn minimal_spec() -> SwarmServiceSpec {
        SwarmServiceSpec {
            image: SwarmServiceImageInfo::External {
                registry_id: Uuid::now_v7(),
                image_tag: "redis:7-alpine".to_owned(),
                resolved_digest: None,
            },
            update_behavior: UpdateBehavior::Disabled,
            scheduling_mode: SchedulingMode::Replicated,
            replicas: Some(1),
            command: Vec::new(),
            arguments: Vec::new(),
            environment: Vec::new(),
            labels: BTreeMap::new(),
            user: None,
            working_directory: None,
            health_check: None,
            stop_grace_period_nanoseconds: None,
            ports: Vec::new(),
            network_ids: Vec::new(),
            mounts: Vec::new(),
            secrets: Vec::new(),
            configs: Vec::new(),
            resources: None,
            placement_constraints: Vec::new(),
            restart_policy: None,
            update_policy: None,
            webhook: None,
        }
    }
}
