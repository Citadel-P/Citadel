use std::sync::Arc;

use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    PlatformInventoryPort, RuntimeCapabilityError, RuntimeInventorySnapshot, RuntimeSwarmInfo,
    jobs::{InventoryCollectionTarget, collect_inventory_from_info},
};

pub const LOCAL_DOCKER_ADDRESS: &str = "http://localhost.docker";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum PlatformType {
    #[default]
    Docker,
    DockerSwarm,
    Kubernetes,
}

impl PlatformType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Docker => "Docker",
            Self::DockerSwarm => "DockerSwarm",
            Self::Kubernetes => "Kubernetes",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum PlatformConnectorType {
    #[default]
    Unknown,
    Local,
    Agent,
    EdgeAgent,
}

impl PlatformConnectorType {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "Unknown",
            Self::Local => "Local",
            Self::Agent => "Agent",
            Self::EdgeAgent => "EdgeAgent",
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreatePlatformInput {
    pub name: String,
    pub address: Option<String>,
    pub description: Option<String>,
    #[serde(rename = "type")]
    #[serde(default)]
    pub platform_type: PlatformType,
    #[serde(default)]
    pub connector_type: PlatformConnectorType,
    #[serde(default = "default_prune_historical_swarm_task_containers")]
    pub prune_historical_swarm_task_containers: bool,
    #[serde(default)]
    pub tag_ids: Vec<Uuid>,
}

const fn default_prune_historical_swarm_task_containers() -> bool {
    true
}

#[derive(Debug, Clone)]
pub struct PlatformRegistration {
    pub id: Uuid,
    pub name: String,
    pub address: String,
    pub description: Option<String>,
    pub connector_type: PlatformConnectorType,
    pub prune_historical_swarm_task_containers: bool,
    pub tag_ids: Vec<Uuid>,
    pub descriptor: Value,
    pub cluster_id: Option<String>,
    pub snapshot: RuntimeInventorySnapshot,
}

#[derive(Debug, thiserror::Error)]
pub enum PlatformRegistrationError {
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error("{0}")]
    Runtime(#[source] RuntimeCapabilityError),
    #[error("{0}")]
    Storage(String),
}

impl From<RuntimeCapabilityError> for PlatformRegistrationError {
    fn from(error: RuntimeCapabilityError) -> Self {
        Self::Runtime(error)
    }
}

pub trait PlatformRegistrationRuntime: Send + Sync {
    fn inventory_for<'a>(
        &'a self,
        connector_type: PlatformConnectorType,
        address: &'a str,
    ) -> Result<&'a dyn PlatformInventoryPort, PlatformRegistrationError>;
}

pub trait PlatformRegistrationStore: Send + Sync {
    fn create_pending_edge<'a>(
        &'a self,
        _actor_id: ActorId,
        _id: Uuid,
        _input: &'a CreatePlatformInput,
    ) -> BoxFuture<'a, Result<Uuid, PlatformRegistrationError>> {
        Box::pin(async {
            Err(PlatformRegistrationError::Storage(
                "Pending Edge Platform persistence is not configured.".into(),
            ))
        })
    }
    fn ensure_available<'a>(
        &'a self,
        name: &'a str,
        address: &'a str,
    ) -> BoxFuture<'a, Result<(), PlatformRegistrationError>>;

    fn create<'a>(
        &'a self,
        actor_id: ActorId,
        registration: &'a PlatformRegistration,
    ) -> BoxFuture<'a, Result<Uuid, PlatformRegistrationError>>;
}

#[derive(Clone)]
pub struct PlatformRegistrationService {
    store: Arc<dyn PlatformRegistrationStore>,
    runtime: Arc<dyn PlatformRegistrationRuntime>,
}

impl PlatformRegistrationService {
    #[must_use]
    pub fn new(
        store: Arc<dyn PlatformRegistrationStore>,
        runtime: Arc<dyn PlatformRegistrationRuntime>,
    ) -> Self {
        Self { store, runtime }
    }

    pub async fn create(
        &self,
        actor_id: ActorId,
        input: CreatePlatformInput,
        cancellation: &CancellationToken,
    ) -> Result<Uuid, PlatformRegistrationError> {
        let input = validate(input)?;
        let id = Uuid::now_v7();
        if input.connector_type == PlatformConnectorType::EdgeAgent {
            return self.store.create_pending_edge(actor_id, id, &input).await;
        }
        let address = match input.connector_type {
            PlatformConnectorType::Unknown => input.address.clone().unwrap_or_default(),
            PlatformConnectorType::Local => LOCAL_DOCKER_ADDRESS.to_owned(),
            PlatformConnectorType::Agent => validate_agent_address(input.address.as_deref())?,
            PlatformConnectorType::EdgeAgent => {
                unreachable!("Edge enrollment creates a pending Platform")
            }
        };
        self.store.ensure_available(&input.name, &address).await?;
        if input.connector_type == PlatformConnectorType::Unknown {
            return Err(PlatformRegistrationError::Validation(
                "A supported Platform connector type is required.".to_owned(),
            ));
        }
        let runtime = self.runtime.inventory_for(input.connector_type, &address)?;
        let mut info = runtime.get_info(cancellation).await?;
        validate_platform_type(input.platform_type, info.swarm.as_ref())?;
        normalize_daemon_id(&mut info)?;
        let mut snapshot = collect_inventory_from_info(
            runtime,
            &InventoryCollectionTarget {
                platform_id: id,
                platform_type: input.platform_type.as_str().to_owned(),
            },
            info,
            cancellation,
        )
        .await?;
        prune_historical_swarm_task_containers(&input, &mut snapshot);
        let swarm = snapshot.info.swarm.as_ref();
        let descriptor = descriptor(input.platform_type, &snapshot, swarm);
        let registration = PlatformRegistration {
            id,
            name: input.name,
            address,
            description: input.description,
            connector_type: input.connector_type,
            prune_historical_swarm_task_containers: input.prune_historical_swarm_task_containers,
            tag_ids: input.tag_ids,
            descriptor,
            cluster_id: swarm.and_then(|value| value.cluster_id.clone()),
            snapshot,
        };
        self.store.create(actor_id, &registration).await
    }
}

fn normalize_daemon_id(
    info: &mut crate::RuntimePlatformInfo,
) -> Result<(), PlatformRegistrationError> {
    if info.daemon_id.trim().is_empty() {
        return Err(PlatformRegistrationError::Runtime(
            RuntimeCapabilityError::new(
                crate::RuntimeErrorKind::Remote,
                "Docker engine did not report a daemon id.",
                false,
            ),
        ));
    }
    info.daemon_id = info.daemon_id.trim().to_owned();
    Ok(())
}

fn prune_historical_swarm_task_containers(
    input: &CreatePlatformInput,
    snapshot: &mut RuntimeInventorySnapshot,
) {
    if input.platform_type != PlatformType::DockerSwarm
        || !input.prune_historical_swarm_task_containers
    {
        return;
    }
    snapshot.containers.retain(|container| {
        !container.is_swarm_task
            || matches!(
                container.state.to_ascii_lowercase().as_str(),
                "running" | "restarting" | "paused"
            )
    });
}

fn validate_agent_address(address: Option<&str>) -> Result<String, PlatformRegistrationError> {
    let address = address.filter(|value| !value.is_empty()).ok_or_else(|| {
        PlatformRegistrationError::Validation(
            "Agent Address is required for an Agent Platform.".to_owned(),
        )
    })?;
    if address.len() > 2048
        || address.chars().any(char::is_control)
        || !(address.starts_with("https://") || address.starts_with("http://"))
    {
        return Err(PlatformRegistrationError::Validation(
            "Agent Address must be a valid HTTP or HTTPS address.".to_owned(),
        ));
    }
    Ok(address.to_owned())
}

fn validate(
    mut input: CreatePlatformInput,
) -> Result<CreatePlatformInput, PlatformRegistrationError> {
    input.name = input.name.trim().to_owned();
    if !(3..=64).contains(&input.name.len())
        || !input
            .name
            .bytes()
            .all(|value| value.is_ascii_alphanumeric() || matches!(value, b'-' | b'_'))
    {
        return Err(PlatformRegistrationError::Validation(
            "Name must contain 3 to 64 letters, numbers, hyphens, or underscores.".to_owned(),
        ));
    }
    input.description = input
        .description
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if input
        .description
        .as_ref()
        .is_some_and(|value| value.chars().count() > 600)
    {
        return Err(PlatformRegistrationError::Validation(
            "Description cannot exceed 600 characters.".to_owned(),
        ));
    }
    if input.tag_ids.len() > 100 {
        return Err(PlatformRegistrationError::Validation(
            "At most 100 Tags may be assigned to a Platform.".to_owned(),
        ));
    }
    if input.platform_type == PlatformType::Kubernetes {
        return Err(PlatformRegistrationError::Validation(
            "Currently, only Docker Standalone and Docker Swarm platform types are supported."
                .to_owned(),
        ));
    }
    input.tag_ids.sort_unstable();
    input.tag_ids.dedup();
    if let Some(address) = input.address.as_mut() {
        *address = address.trim().trim_end_matches('/').to_owned();
    }
    Ok(input)
}

fn validate_platform_type(
    platform_type: PlatformType,
    swarm: Option<&RuntimeSwarmInfo>,
) -> Result<Option<&RuntimeSwarmInfo>, PlatformRegistrationError> {
    match (platform_type, swarm) {
        (PlatformType::Docker, Some(value))
            if value.local_node_state.eq_ignore_ascii_case("active") =>
        {
            Err(PlatformRegistrationError::Validation(
                "This Docker Engine is an active Swarm member. Register it as Docker Swarm instead."
                    .to_owned(),
            ))
        }
        (PlatformType::Docker, _) => Ok(None),
        (PlatformType::DockerSwarm, None) => Err(PlatformRegistrationError::Validation(
            "This Docker Engine is not an active Swarm member.".to_owned(),
        )),
        (PlatformType::DockerSwarm, Some(value))
            if !value.local_node_state.eq_ignore_ascii_case("active") =>
        {
            Err(PlatformRegistrationError::Validation(
                "This Docker Engine is not an active Swarm member.".to_owned(),
            ))
        }
        (PlatformType::DockerSwarm, Some(value)) if !value.control_available => {
            Err(PlatformRegistrationError::Validation(
                "The selected Docker Engine is a Swarm worker. Connect Citadel to a manager node."
                    .to_owned(),
            ))
        }
        (PlatformType::DockerSwarm, Some(value))
            if value.cluster_id.as_deref().is_none_or(str::is_empty) =>
        {
            Err(PlatformRegistrationError::Validation(
                "The Swarm manager did not report a cluster id.".to_owned(),
            ))
        }
        (PlatformType::DockerSwarm, Some(value)) => Ok(Some(value)),
        (PlatformType::Kubernetes, _) => unreachable!("unsupported types are rejected first"),
    }
}

fn descriptor(
    platform_type: PlatformType,
    snapshot: &RuntimeInventorySnapshot,
    swarm: Option<&RuntimeSwarmInfo>,
) -> Value {
    let info = &snapshot.info;
    let mut value = json!({
        "$type": platform_type.as_str(),
        "daemonId": info.daemon_id.trim(),
        "containerCount": snapshot.containers.len(),
        "containersRunning": snapshot.containers.iter().filter(|value| matches!(value.state.as_str(), "running" | "restarting")).count(),
        "containersPaused": snapshot.containers.iter().filter(|value| value.state == "paused").count(),
        "containersStopped": snapshot.containers.iter().filter(|value| !matches!(value.state.as_str(), "running" | "restarting" | "paused")).count(),
        "operatingSystem": info.operating_system,
        "osType": info.os_type,
        "architecture": info.architecture,
        "apiVersion": info.api_version,
        "minimumApiVersion": info.minimum_api_version,
    });
    if let Some(swarm) = swarm {
        let object = value.as_object_mut().expect("descriptor is an object");
        object.insert("nodeID".to_owned(), json!(swarm.node_id));
        object.insert("nodeAddr".to_owned(), json!(swarm.node_addr));
        object.insert("localNodeState".to_owned(), json!(swarm.local_node_state));
        object.insert(
            "controlAvailable".to_owned(),
            json!(swarm.control_available),
        );
        object.insert("nodes".to_owned(), json!(swarm.nodes));
        object.insert("managers".to_owned(), json!(swarm.managers));
        object.insert("clusterId".to_owned(), json!(swarm.cluster_id));
        object.insert(
            "clusterCreatedAt".to_owned(),
            json!(swarm.cluster_created_at),
        );
        object.insert("error".to_owned(), json!(swarm.error));
        object.insert("remoteManagers".to_owned(), json!(swarm.remote_managers));
        object.insert(
            "serviceCount".to_owned(),
            json!(
                snapshot
                    .swarm
                    .as_ref()
                    .map_or(0, |value| value.services.len())
            ),
        );
        object.insert(
            "runningTaskCount".to_owned(),
            json!(snapshot.swarm.as_ref().map_or(0, |value| {
                value
                    .tasks
                    .iter()
                    .filter(|task| task.state.eq_ignore_ascii_case("running"))
                    .count()
            })),
        );
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_type_against_the_discovered_daemon() {
        let swarm = RuntimeSwarmInfo {
            local_node_state: "active".to_owned(),
            control_available: true,
            cluster_id: Some("cluster-1".to_owned()),
            ..RuntimeSwarmInfo::default()
        };
        assert!(matches!(
            validate_platform_type(PlatformType::Docker, Some(&swarm)),
            Err(PlatformRegistrationError::Validation(message)) if message.contains("active Swarm")
        ));
        assert!(validate_platform_type(PlatformType::DockerSwarm, Some(&swarm)).is_ok());
    }

    #[test]
    fn rejects_invalid_names_before_opening_a_runtime() {
        let result = validate(CreatePlatformInput {
            name: "bad name".to_owned(),
            address: None,
            description: None,
            platform_type: PlatformType::Docker,
            connector_type: PlatformConnectorType::Local,
            prune_historical_swarm_task_containers: true,
            tag_ids: Vec::new(),
        });
        assert!(matches!(
            result,
            Err(PlatformRegistrationError::Validation(_))
        ));
    }

    #[test]
    fn swarm_registration_requires_an_active_manager_with_cluster_identity() {
        assert!(matches!(
            validate_platform_type(PlatformType::DockerSwarm, None),
            Err(PlatformRegistrationError::Validation(message)) if message.contains("not an active Swarm")
        ));
        let worker = RuntimeSwarmInfo {
            local_node_state: "active".to_owned(),
            control_available: false,
            cluster_id: Some("cluster-1".to_owned()),
            ..RuntimeSwarmInfo::default()
        };
        assert!(matches!(
            validate_platform_type(PlatformType::DockerSwarm, Some(&worker)),
            Err(PlatformRegistrationError::Validation(message)) if message.contains("manager node")
        ));
        let missing_cluster = RuntimeSwarmInfo {
            local_node_state: "active".to_owned(),
            control_available: true,
            ..RuntimeSwarmInfo::default()
        };
        assert!(matches!(
            validate_platform_type(PlatformType::DockerSwarm, Some(&missing_cluster)),
            Err(PlatformRegistrationError::Validation(message)) if message.contains("cluster id")
        ));
    }

    #[test]
    fn agent_registration_requires_a_bounded_http_address() {
        assert!(validate_agent_address(Some("https://agent.example.test:5001")).is_ok());
        assert!(matches!(
            validate_agent_address(Some("")),
            Err(PlatformRegistrationError::Validation(_))
        ));
        assert!(matches!(
            validate_agent_address(Some("file:///var/run/docker.sock")),
            Err(PlatformRegistrationError::Validation(_))
        ));
    }

    #[test]
    fn unsupported_platform_types_are_validation_failures() {
        let result = validate(CreatePlatformInput {
            name: "kubernetes".to_owned(),
            address: None,
            description: None,
            platform_type: PlatformType::Kubernetes,
            connector_type: PlatformConnectorType::Local,
            prune_historical_swarm_task_containers: true,
            tag_ids: Vec::new(),
        });
        assert!(matches!(
            result,
            Err(PlatformRegistrationError::Validation(message)) if message.contains("only Docker")
        ));
    }

    #[test]
    fn missing_daemon_identity_is_a_remote_runtime_failure() {
        let mut info = crate::RuntimePlatformInfo {
            daemon_id: " \t".to_owned(),
            ..crate::RuntimePlatformInfo::default()
        };

        assert!(matches!(
            normalize_daemon_id(&mut info),
            Err(PlatformRegistrationError::Runtime(RuntimeCapabilityError {
                kind: crate::RuntimeErrorKind::Remote,
                ..
            }))
        ));

        info.daemon_id = "  daemon-1  ".to_owned();
        normalize_daemon_id(&mut info).unwrap();
        assert_eq!(info.daemon_id, "daemon-1");
    }

    #[test]
    fn historical_swarm_task_pruning_respects_the_platform_setting() {
        let containers = vec![
            container("current", "running", true),
            container("restarting", "restarting", true),
            container("paused", "paused", true),
            container("historical", "exited", true),
            container("ordinary-stopped", "exited", false),
        ];
        let mut pruned = snapshot(containers.clone());
        let mut retained = snapshot(containers);
        let mut input = swarm_input(true);

        prune_historical_swarm_task_containers(&input, &mut pruned);
        assert_eq!(
            pruned
                .containers
                .iter()
                .map(|value| value.id.as_str())
                .collect::<Vec<_>>(),
            ["current", "restarting", "paused", "ordinary-stopped"]
        );

        input.prune_historical_swarm_task_containers = false;
        prune_historical_swarm_task_containers(&input, &mut retained);
        assert_eq!(retained.containers.len(), 5);
    }

    fn swarm_input(prune: bool) -> CreatePlatformInput {
        CreatePlatformInput {
            name: "swarm-platform".to_owned(),
            address: Some("https://agent.example.test".to_owned()),
            description: None,
            platform_type: PlatformType::DockerSwarm,
            connector_type: PlatformConnectorType::Agent,
            prune_historical_swarm_task_containers: prune,
            tag_ids: Vec::new(),
        }
    }

    fn container(id: &str, state: &str, is_swarm_task: bool) -> crate::RuntimeContainerSummary {
        crate::RuntimeContainerSummary {
            id: id.to_owned(),
            name: id.to_owned(),
            image: "nginx:latest".to_owned(),
            image_id: "sha256:image".to_owned(),
            created: 1,
            state: state.to_owned(),
            status: state.to_owned(),
            labels: Default::default(),
            ports: json!([]),
            stack: None,
            is_system: false,
            system_role: None,
            has_citadel_ownership_labels: false,
            is_swarm_task,
        }
    }

    fn snapshot(containers: Vec<crate::RuntimeContainerSummary>) -> RuntimeInventorySnapshot {
        RuntimeInventorySnapshot {
            platform_id: Uuid::now_v7(),
            info: crate::RuntimePlatformInfo::default(),
            containers,
            images: Vec::new(),
            networks: Vec::new(),
            volumes: Vec::new(),
            swarm: None,
            observed_at: chrono::Utc::now(),
        }
    }
}
