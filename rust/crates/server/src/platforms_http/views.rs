use chrono::{DateTime, Utc};

use serde::Serialize;

use serde_json::Value;

use uuid::Uuid;

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_view_logs: bool,
    pub can_inspect: bool,
    pub can_open_terminal: bool,
    pub can_pull: bool,
    pub can_manage_node_agents: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_inspect: bool,
    pub can_pull: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_inspect: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_inspect: bool,
    pub can_browse: bool,
    pub can_download: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStatView {
    pub created: i64,
    pub tx_bytes: f64,
    pub rx_bytes: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_used_bytes: Option<i64>,
    pub disk_total_bytes: Option<i64>,
    pub disk_usage: Option<f64>,
}

impl From<citadel_platforms::PlatformStatSnapshot> for PlatformStatView {
    fn from(value: citadel_platforms::PlatformStatSnapshot) -> Self {
        Self {
            created: value.created,
            tx_bytes: value.tx_bytes,
            rx_bytes: value.rx_bytes,
            cpu_usage: value.cpu_usage,
            memory_usage: value.memory_usage,
            disk_used_bytes: value.disk_used_bytes,
            disk_total_bytes: value.disk_total_bytes,
            disk_usage: value.disk_usage,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct PlatformView {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub address: String,
    pub network_count: i32,
    pub volume_count: i32,
    pub image_count: i64,
    pub cpu_count: i64,
    pub mem_total: i64,
    pub agent_version: Option<String>,
    pub server_version: Option<String>,
    #[serde(rename = "type")]
    pub platform_type: String,
    pub status: String,
    pub connector_type: String,
    pub deployment_count: i64,
    pub stack_count: i64,
    pub deployment_status_counts: WorkloadStatusCounts,
    pub stack_status_counts: WorkloadStatusCounts,
    pub swarm_service_status_counts: WorkloadStatusCounts,
    pub stats: Option<Vec<PlatformStatView>>,
    pub platform_descriptor: Value,
    pub cluster_id: Option<String>,
    pub prune_historical_swarm_task_containers: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::PlatformDetails> for PlatformView {
    fn from(value: citadel_platforms::PlatformDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            description: value.description,
            address: value.address,
            network_count: value.network_count,
            volume_count: value.volume_count,
            image_count: value.image_count,
            cpu_count: value.cpu_count,
            mem_total: value.mem_total,
            agent_version: value.agent_version,
            server_version: value.server_version,
            platform_type: value.platform_type,
            status: value.status,
            connector_type: value.connector_type,
            deployment_count: value.deployment_count,
            stack_count: value.stack_count,
            deployment_status_counts: value.deployment_status_counts.into(),
            stack_status_counts: value.stack_status_counts.into(),
            swarm_service_status_counts: value.swarm_service_status_counts.into(),
            stats: value
                .stats
                .map(|items| items.into_iter().map(Into::into).collect()),
            platform_descriptor: value.platform_descriptor,
            cluster_id: value.cluster_id,
            prune_historical_swarm_task_containers: value.prune_historical_swarm_task_containers,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatView {
    pub container_id: Uuid,
    pub memory_active: f64,
    pub memory_cache: f64,
    pub cpu_usage: f64,
    pub memory_limit: f64,
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub created: i64,
}

impl From<citadel_platforms::ContainerStatSnapshot> for ContainerStatView {
    fn from(value: citadel_platforms::ContainerStatSnapshot) -> Self {
        Self {
            container_id: value.container_id,
            memory_active: value.memory_active,
            memory_cache: value.memory_cache,
            cpu_usage: value.cpu_usage,
            memory_limit: value.memory_limit,
            rx_bytes: value.rx_bytes,
            tx_bytes: value.tx_bytes,
            created: value.created,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerView {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub container_id: String,
    pub name: String,
    pub docker_image_id: String,
    pub created: i64,
    pub state: String,
    pub control_state: String,
    pub updated: i64,
    pub stack: Option<String>,
    pub is_system: bool,
    pub system_role: Option<String>,
    pub has_citadel_ownership_labels: bool,
    pub is_swarm_task: bool,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub projection_observed_at: Option<i64>,
    pub projection_stale_since: Option<i64>,
    pub projection_stale_reason: Option<String>,
    pub last_stats: Option<ContainerStatView>,
    pub ports: Value,
    pub deployment_id: Option<Uuid>,
    pub stack_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_view: Option<ImageView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_view: Option<ContainerDeploymentView>,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::ContainerDetails> for ContainerView {
    fn from(value: citadel_platforms::ContainerDetails) -> Self {
        Self {
            id: value.id,
            platform_id: value.platform_id,
            container_id: value.container_id,
            name: value.name,
            docker_image_id: value.docker_image_id,
            created: value.created,
            state: value.state,
            control_state: value.control_state,
            updated: value.updated,
            stack: value.stack,
            is_system: value.is_system,
            system_role: value.system_role,
            has_citadel_ownership_labels: value.has_citadel_ownership_labels,
            is_swarm_task: value.is_swarm_task,
            docker_node_id: value.docker_node_id,
            node_hostname: value.node_hostname,
            projection_observed_at: value.projection_observed_at,
            projection_stale_since: value.projection_stale_since,
            projection_stale_reason: value.projection_stale_reason,
            last_stats: value.last_stats.map(Into::into),
            ports: value.ports,
            deployment_id: value.deployment_id,
            stack_id: value.stack_id,
            image_view: value.image_view.map(Into::into),
            deployment_view: value.deployment_view.map(Into::into),
            capabilities: None,
        }
    }
}

/// The container contract embeds a summary, never Deployment configuration or
/// resource bindings. These are read-model fields, not a cross-feature entity.
#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDeploymentView {
    pub id: Uuid,
    pub name: String,
    pub platform_id: Uuid,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub created_by_actor_id: Uuid,
    pub control_state: String,
    pub platform_status: String,
    pub auto_update_state: ContainerDeploymentUpdateState,
}

impl From<citadel_platforms::ContainerDeploymentSummary> for ContainerDeploymentView {
    fn from(value: citadel_platforms::ContainerDeploymentSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            platform_id: value.platform_id,
            status: value.status,
            created_at: value.created_at,
            created_by_actor_id: value.created_by_actor_id,
            control_state: value.control_state,
            platform_status: value.platform_status,
            auto_update_state: value.auto_update_state.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageView {
    pub id: Uuid,
    pub tags: Vec<String>,
    pub name: String,
    pub docker_image_id: String,
    pub size: f64,
    pub is_in_use: bool,
    pub platform_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub control_state: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub registry_id: Option<Uuid>,
    pub repo_digests: Option<Vec<String>>,
    pub content_identity: Option<String>,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub is_stale: bool,
    pub stale_reason: Option<String>,
    pub capabilities: Option<ImageCapabilitiesView>,
}

impl From<citadel_platforms::ImageDetails> for ImageView {
    fn from(value: citadel_platforms::ImageDetails) -> Self {
        Self {
            id: value.id,
            tags: value.tags,
            name: value.name,
            docker_image_id: value.docker_image_id,
            size: value.size,
            is_in_use: value.is_in_use,
            platform_id: value.platform_id,
            created_at: value.created_at,
            control_state: value.control_state,
            updated_at: value.updated_at,
            registry_id: value.registry_id,
            repo_digests: value.repo_digests,
            content_identity: value.content_identity,
            docker_node_id: value.docker_node_id,
            node_hostname: value.node_hostname,
            is_stale: value.is_stale,
            stale_reason: value.stale_reason,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkView {
    pub name: String,
    pub id: String,
    pub created: String,
    pub driver: String,
    pub scope: String,
    pub enable_ipv4: bool,
    pub enable_ipv6: bool,
    pub internal: bool,
    pub attachable: bool,
    pub ingress: bool,
    pub config_only: bool,
    pub in_use: bool,
    pub config_from: Option<String>,
    pub ipam: Option<Value>,
    pub options: BTreeMap<String, String>,
    pub labels: BTreeMap<String, String>,
    pub containers: BTreeMap<String, Value>,
    pub peers: Vec<Value>,
    pub is_system: bool,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub is_stale: bool,
    pub stale_reason: Option<String>,
    pub capabilities: Option<NetworkCapabilitiesView>,
}

impl From<citadel_platforms::NetworkDetails> for NetworkView {
    fn from(value: citadel_platforms::NetworkDetails) -> Self {
        Self {
            name: value.name,
            id: value.id,
            created: value.created,
            driver: value.driver,
            scope: value.scope,
            enable_ipv4: value.enable_ipv4,
            enable_ipv6: value.enable_ipv6,
            internal: value.internal,
            attachable: value.attachable,
            ingress: value.ingress,
            config_only: value.config_only,
            in_use: value.in_use,
            config_from: value.config_from,
            ipam: value.ipam,
            options: value.options,
            labels: value.labels,
            containers: value.containers,
            peers: value.peers,
            is_system: value.is_system,
            docker_node_id: value.docker_node_id,
            node_hostname: value.node_hostname,
            is_stale: value.is_stale,
            stale_reason: value.stale_reason,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeView {
    pub id: String,
    pub name: String,
    pub in_use: bool,
    pub scope: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: String,
    pub cluster_volume: Option<Value>,
    pub usage_data: Option<VolumeUsageDataView>,
    pub containers: Vec<Value>,
    pub status: BTreeMap<String, String>,
    pub labels: BTreeMap<String, String>,
    pub options: BTreeMap<String, String>,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub is_stale: bool,
    pub stale_reason: Option<String>,
    pub capabilities: Option<VolumeCapabilitiesView>,
}

impl From<citadel_platforms::VolumeDetails> for VolumeView {
    fn from(value: citadel_platforms::VolumeDetails) -> Self {
        Self {
            id: value.id,
            name: value.name,
            in_use: value.in_use,
            scope: value.scope,
            driver: value.driver,
            mountpoint: value.mountpoint,
            created_at: value.created_at,
            cluster_volume: value.cluster_volume,
            usage_data: value.usage_data.map(Into::into),
            containers: value.containers,
            status: value.status,
            labels: value.labels,
            options: value.options,
            docker_node_id: value.docker_node_id,
            node_hostname: value.node_hostname,
            is_stale: value.is_stale,
            stale_reason: value.stale_reason,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct VolumeUsageDataView {
    #[serde(alias = "Size")]
    pub size: i64,
    #[serde(alias = "RefCount")]
    pub ref_count: i64,
}

impl From<citadel_platforms::VolumeUsageDataSummary> for VolumeUsageDataView {
    fn from(value: citadel_platforms::VolumeUsageDataSummary) -> Self {
        Self {
            size: value.size,
            ref_count: value.ref_count,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNodeView {
    pub id: String,
    pub version_index: i64,
    pub hostname: String,
    pub role: String,
    pub is_leader: bool,
    pub reachability: String,
    pub status: String,
    pub status_message: Option<String>,
    pub availability: String,
    pub engine_version: String,
    pub operating_system: String,
    pub architecture: String,
    pub address: String,
    pub labels: BTreeMap<String, String>,
    pub running_task_count: i32,
    pub desired_task_count: i32,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub is_stale: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::SwarmNodeSummary> for SwarmNodeView {
    fn from(value: citadel_platforms::SwarmNodeSummary) -> Self {
        Self {
            id: value.id,
            version_index: value.version_index,
            hostname: value.hostname,
            role: value.role,
            is_leader: value.is_leader,
            reachability: value.reachability,
            status: value.status,
            status_message: value.status_message,
            availability: value.availability,
            engine_version: value.engine_version,
            operating_system: value.operating_system,
            architecture: value.architecture,
            address: value.address,
            labels: value.labels,
            running_task_count: value.running_task_count,
            desired_task_count: value.desired_task_count,
            created_at: value.created_at,
            updated_at: value.updated_at,
            observed_at: value.observed_at,
            is_stale: value.is_stale,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceView {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub mode: String,
    pub image: String,
    pub running_task_count: i32,
    pub desired_task_count: i32,
    pub update_state: String,
    pub update_message: Option<String>,
    pub ports: Vec<String>,
    pub network_ids: Vec<String>,
    pub secret_ids: Vec<String>,
    pub config_ids: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub ownership: String,
    pub docker_stack_namespace: Option<String>,
    pub ownership_diagnostic: Option<String>,
    pub stack_id: Option<Uuid>,
    pub swarm_service_id: Option<Uuid>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub is_stale: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::SwarmServiceSummary> for SwarmServiceView {
    fn from(value: citadel_platforms::SwarmServiceSummary) -> Self {
        Self {
            id: value.id,
            version_index: value.version_index,
            name: value.name,
            mode: value.mode,
            image: value.image,
            running_task_count: value.running_task_count,
            desired_task_count: value.desired_task_count,
            update_state: value.update_state,
            update_message: value.update_message,
            ports: value.ports,
            network_ids: value.network_ids,
            secret_ids: value.secret_ids,
            config_ids: value.config_ids,
            labels: value.labels,
            ownership: value.ownership,
            docker_stack_namespace: value.docker_stack_namespace,
            ownership_diagnostic: value.ownership_diagnostic,
            stack_id: value.stack_id,
            swarm_service_id: value.swarm_service_id,
            created_at: value.created_at,
            updated_at: value.updated_at,
            observed_at: value.observed_at,
            is_stale: value.is_stale,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmTaskView {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub service_id: String,
    pub service_name: String,
    pub slot: Option<i32>,
    pub node_id: String,
    pub node_hostname: String,
    pub desired_state: String,
    pub state: String,
    pub status_message: Option<String>,
    pub error: Option<String>,
    pub image: String,
    pub ports: Vec<String>,
    pub status_timestamp: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub is_stale: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::SwarmTaskSummary> for SwarmTaskView {
    fn from(value: citadel_platforms::SwarmTaskSummary) -> Self {
        Self {
            id: value.id,
            version_index: value.version_index,
            name: value.name,
            service_id: value.service_id,
            service_name: value.service_name,
            slot: value.slot,
            node_id: value.node_id,
            node_hostname: value.node_hostname,
            desired_state: value.desired_state,
            state: value.state,
            status_message: value.status_message,
            error: value.error,
            image: value.image,
            ports: value.ports,
            status_timestamp: value.status_timestamp,
            created_at: value.created_at,
            updated_at: value.updated_at,
            observed_at: value.observed_at,
            is_stale: value.is_stale,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNetworkView {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub driver: String,
    pub is_attachable: bool,
    pub is_internal: bool,
    pub is_ingress: bool,
    pub is_encrypted: bool,
    pub enable_ipv6: bool,
    pub subnets: Vec<String>,
    pub service_names: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub is_stale: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::SwarmNetworkSummary> for SwarmNetworkView {
    fn from(value: citadel_platforms::SwarmNetworkSummary) -> Self {
        Self {
            id: value.id,
            name: value.name,
            scope: value.scope,
            driver: value.driver,
            is_attachable: value.is_attachable,
            is_internal: value.is_internal,
            is_ingress: value.is_ingress,
            is_encrypted: value.is_encrypted,
            enable_ipv6: value.enable_ipv6,
            subnets: value.subnets,
            service_names: value.service_names,
            labels: value.labels,
            created_at: value.created_at,
            observed_at: value.observed_at,
            is_stale: value.is_stale,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmConfigView {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub templating_driver: Option<String>,
    pub service_names: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub is_stale: bool,
    pub in_use: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::SwarmConfigSummary> for SwarmConfigView {
    fn from(value: citadel_platforms::SwarmConfigSummary) -> Self {
        Self {
            id: value.id,
            version_index: value.version_index,
            name: value.name,
            templating_driver: value.templating_driver,
            service_names: value.service_names,
            labels: value.labels,
            created_at: value.created_at,
            updated_at: value.updated_at,
            observed_at: value.observed_at,
            is_stale: value.is_stale,
            in_use: value.in_use,
            capabilities: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SwarmSecretView {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub driver: Option<String>,
    pub service_names: Vec<String>,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
    pub is_stale: bool,
    pub in_use: bool,
    pub capabilities: Option<PlatformCapabilitiesView>,
}

impl From<citadel_platforms::SwarmSecretSummary> for SwarmSecretView {
    fn from(value: citadel_platforms::SwarmSecretSummary) -> Self {
        Self {
            id: value.id,
            version_index: value.version_index,
            name: value.name,
            driver: value.driver,
            service_names: value.service_names,
            labels: value.labels,
            created_at: value.created_at,
            updated_at: value.updated_at,
            observed_at: value.observed_at,
            is_stale: value.is_stale,
            in_use: value.in_use,
            capabilities: None,
        }
    }
}

impl ContainerView {
    /// Shared HTTP/live runtime shape: `id` is the Docker ID, not the DB UUID.
    pub fn runtime_data(&self) -> Value {
        serde_json::json!({
            "id":self.container_id,"name":self.name,"platformId":self.platform_id,
            "image":self.image_view.as_ref().map_or("", |image| image.name.as_str()),"imageId":self.docker_image_id,"state":self.state,
            "controlState":self.control_state,"created":self.created,"stack":self.stack,
            "ports":self.ports,"containerStat":self.last_stats,
            "isSystem":self.is_system,"systemRole":self.system_role,
            "hasCitadelOwnershipLabels":self.has_citadel_ownership_labels,
            "isSwarmTask":self.is_swarm_task,"dockerNodeId":self.docker_node_id,
            "deploymentId":self.deployment_id,"stackId":self.stack_id,
            "capabilities":self.capabilities,
        })
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageInspectionView {
    #[serde(flatten)]
    pub image: citadel_platforms::images::ImageInspection,
    pub capabilities: Option<ImageCapabilitiesView>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct WorkloadStatusCounts {
    pub total: i64,
    pub healthy: i64,
    pub degraded: i64,
    pub failed: i64,
    pub stopped: i64,
    pub paused: i64,
    pub in_progress: i64,
    pub unknown: i64,
}
impl From<citadel_platforms::WorkloadStatusCounts> for WorkloadStatusCounts {
    fn from(value: citadel_platforms::WorkloadStatusCounts) -> Self {
        Self {
            total: value.total,
            healthy: value.healthy,
            degraded: value.degraded,
            failed: value.failed,
            stopped: value.stopped,
            paused: value.paused,
            in_progress: value.in_progress,
            unknown: value.unknown,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDeploymentUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: String,
}
impl From<citadel_platforms::ContainerDeploymentUpdateState> for ContainerDeploymentUpdateState {
    fn from(value: citadel_platforms::ContainerDeploymentUpdateState) -> Self {
        Self {
            last_checked_at: value.last_checked_at,
            status: value.status,
        }
    }
}
