use chrono::{DateTime, Utc};
use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectivePlatformPermission {
    pub level_mask: i32,
    pub specific_mask: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformStatSnapshot {
    pub created: i64,
    pub tx_bytes: f64,
    pub rx_bytes: f64,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub disk_used_bytes: Option<i64>,
    pub disk_total_bytes: Option<i64>,
    pub disk_usage: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformDetails {
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
    pub stats: Option<Vec<PlatformStatSnapshot>>,
    pub platform_descriptor: Value,
    pub cluster_id: Option<String>,
    pub prune_historical_swarm_task_containers: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatSnapshot {
    pub container_id: Uuid,
    pub memory_active: f64,
    pub memory_cache: f64,
    pub cpu_usage: f64,
    pub memory_limit: f64,
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub created: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDetails {
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
    pub last_stats: Option<ContainerStatSnapshot>,
    pub ports: Value,
    pub deployment_id: Option<Uuid>,
    pub stack_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_view: Option<ImageDetails>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deployment_view: Option<ContainerDeploymentSummary>,
}

/// The container contract embeds a summary, never Deployment configuration or
/// resource bindings. These are read-model fields, not a cross-feature entity.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDeploymentSummary {
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

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerDeploymentUpdateState {
    pub last_checked_at: DateTime<Utc>,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageDetails {
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkDetails {
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
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeDetails {
    pub id: String,
    pub name: String,
    pub in_use: bool,
    pub scope: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: String,
    pub cluster_volume: Option<Value>,
    pub usage_data: Option<VolumeUsageDataSummary>,
    pub containers: Vec<Value>,
    pub status: BTreeMap<String, String>,
    pub labels: BTreeMap<String, String>,
    pub options: BTreeMap<String, String>,
    pub docker_node_id: Option<String>,
    pub node_hostname: Option<String>,
    pub is_stale: bool,
    pub stale_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeUsageDataSummary {
    #[serde(alias = "Size")]
    pub size: i64,
    #[serde(alias = "RefCount")]
    pub ref_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNodeSummary {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmServiceSummary {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmTaskSummary {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNetworkSummary {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmConfigSummary {
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmSecretSummary {
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
}

#[derive(Debug, Clone)]
pub struct NodeResourceProjection<T> {
    pub resource: T,
    pub docker_node_id: String,
    pub node_hostname: Option<String>,
    pub is_stale: bool,
}

/// Identity only, for associating live samples without loading resource details.
#[derive(Debug, Clone)]
pub struct ContainerIdentity {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub deployment_id: Option<Uuid>,
    pub stack_id: Option<Uuid>,
    pub container_id: String,
    pub docker_node_id: Option<String>,
}
#[derive(Debug, Clone)]
pub struct PlatformTelemetryContext {
    pub cpu_count: i64,
    pub mem_total: i64,
    pub network_count: i32,
    pub volume_count: i32,
    pub image_count: i64,
    pub descriptor: Value,
}
