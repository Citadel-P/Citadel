//! Normalized adapter snapshots used by runtime mapping. Protocol calls use citadel-docker-api.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
// Docker emits null for some optional collection fields even when its OpenAPI
// schema declares an array or object. Treating null as empty preserves the
// generated collection type while accepting the daemon's wire representation.
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de> + Default,
{
    Option::<T>::deserialize(deserializer).map(Option::unwrap_or_default)
}
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerVersion {
    #[serde(
        rename = "Components",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub components: Vec<citadel_docker_api::models::SystemVersionComponentsInner>,
    #[serde(rename = "Version", default)]
    pub version: String,
    #[serde(rename = "ApiVersion", default)]
    pub api_version: String,
    #[serde(rename = "MinAPIVersion", default)]
    pub min_api_version: String,
    #[serde(rename = "GitCommit", default)]
    pub git_commit: String,
    #[serde(rename = "Os", default)]
    pub os: String,
    #[serde(rename = "Arch", default)]
    pub arch: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerInfo {
    #[serde(rename = "Name", default)]
    pub hostname: String,
    #[serde(rename = "ServerVersion", default)]
    pub server_version: String,
    #[serde(rename = "Driver", default)]
    pub driver: String,
    #[serde(rename = "OSVersion", default)]
    pub os_version: String,
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Containers", default)]
    pub containers: u64,
    #[serde(rename = "ContainersRunning", default)]
    pub containers_running: u64,
    #[serde(rename = "ContainersPaused", default)]
    pub containers_paused: u64,
    #[serde(rename = "ContainersStopped", default)]
    pub containers_stopped: u64,
    #[serde(rename = "Images", default)]
    pub images: u64,
    #[serde(rename = "DockerRootDir", default)]
    pub docker_root_dir: String,
    #[serde(rename = "NCPU", default)]
    pub cpu_count: u32,
    #[serde(rename = "MemTotal", default)]
    pub memory_total: u64,
    #[serde(rename = "OperatingSystem", default)]
    pub operating_system: String,
    #[serde(rename = "OSType", default)]
    pub os_type: String,
    #[serde(rename = "Architecture", default)]
    pub architecture: String,
    #[serde(rename = "Swarm", default)]
    pub swarm: Option<DockerSwarmInfo>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerSwarmInfo {
    #[serde(rename = "NodeID", default)]
    pub node_id: String,
    #[serde(rename = "NodeAddr", default)]
    pub node_addr: String,
    #[serde(rename = "LocalNodeState", default)]
    pub local_node_state: String,
    #[serde(rename = "ControlAvailable", default)]
    pub control_available: bool,
    #[serde(rename = "Error", default)]
    pub error: String,
    #[serde(
        rename = "RemoteManagers",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub remote_managers: Vec<DockerPeerNode>,
    #[serde(rename = "Nodes", default)]
    pub nodes: i64,
    #[serde(rename = "Managers", default)]
    pub managers: i64,
    #[serde(rename = "Cluster", default)]
    pub cluster: Option<DockerClusterInfo>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerPeerNode {
    #[serde(rename = "NodeID", default)]
    pub node_id: String,
    #[serde(rename = "Addr", default)]
    pub address: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerClusterInfo {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerSummary {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(rename = "Names", default)]
    pub names: Vec<String>,
    #[serde(rename = "Image", default)]
    pub image: String,
    #[serde(rename = "ImageID", default)]
    pub image_id: String,
    #[serde(rename = "Created", default)]
    pub created: i64,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
    #[serde(rename = "State", default)]
    pub state: String,
    #[serde(rename = "Status", default)]
    pub status: String,
    #[serde(rename = "Ports", default)]
    pub ports: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerInspect {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(rename = "Created", default)]
    pub created: String,
    #[serde(rename = "Path", default)]
    pub path: String,
    #[serde(rename = "Args", default)]
    pub args: Vec<String>,
    #[serde(rename = "State", default)]
    pub state: ContainerState,
    #[serde(rename = "Image", default)]
    pub image: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Config", default)]
    pub config: ContainerConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerState {
    #[serde(rename = "Status", default)]
    pub status: String,
    #[serde(rename = "Running", default)]
    pub running: bool,
    #[serde(rename = "Paused", default)]
    pub paused: bool,
    #[serde(rename = "Restarting", default)]
    pub restarting: bool,
    #[serde(rename = "OOMKilled", default)]
    pub oom_killed: bool,
    #[serde(rename = "Dead", default)]
    pub dead: bool,
    #[serde(rename = "Pid", default)]
    pub pid: i64,
    #[serde(rename = "ExitCode", default)]
    pub exit_code: i64,
    #[serde(rename = "Error", default)]
    pub error: String,
    #[serde(rename = "StartedAt", default)]
    pub started_at: String,
    #[serde(rename = "FinishedAt", default)]
    pub finished_at: String,
    #[serde(rename = "Health", default)]
    pub health: Option<ContainerHealth>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerHealth {
    #[serde(rename = "Status", default)]
    pub status: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerConfig {
    #[serde(rename = "Env", default)]
    pub environment: Vec<String>,
    #[serde(rename = "Image", default)]
    pub image: String,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerEventActor {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Attributes", default)]
    pub attributes: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerEvent {
    #[serde(rename = "Type", default)]
    pub resource_type: String,
    #[serde(rename = "Action", default)]
    pub action: String,
    #[serde(rename = "Actor", default)]
    pub actor: DockerEventActor,
    #[serde(default)]
    pub scope: String,
    #[serde(default)]
    pub time: i64,
    #[serde(rename = "timeNano", default)]
    pub time_nano: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ContainerStats {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub read: String,
    #[serde(default)]
    pub preread: String,
    #[serde(default)]
    pub cpu_stats: CpuStats,
    #[serde(default)]
    pub precpu_stats: CpuStats,
    #[serde(default)]
    pub memory_stats: MemoryStats,
    #[serde(default)]
    pub networks: HashMap<String, NetworkStats>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CpuStats {
    #[serde(default)]
    pub cpu_usage: CpuUsage,
    #[serde(default)]
    pub system_cpu_usage: u64,
    #[serde(default)]
    pub online_cpus: u32,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct CpuUsage {
    #[serde(default)]
    pub total_usage: u64,
    #[serde(default)]
    pub percpu_usage: Vec<u64>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct MemoryStats {
    #[serde(default)]
    pub usage: u64,
    #[serde(default)]
    pub stats: HashMap<String, u64>,
    #[serde(default)]
    pub limit: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkStats {
    #[serde(default)]
    pub rx_bytes: u64,
    #[serde(default)]
    pub rx_packets: u64,
    #[serde(default)]
    pub rx_errors: u64,
    #[serde(default)]
    pub rx_dropped: u64,
    #[serde(default)]
    pub tx_bytes: u64,
    #[serde(default)]
    pub tx_packets: u64,
    #[serde(default)]
    pub tx_errors: u64,
    #[serde(default)]
    pub tx_dropped: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmInspect {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Version", default)]
    pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "RootRotationInProgress", default)]
    pub root_rotation_in_progress: bool,
    #[serde(rename = "JoinTokens", default)]
    pub join_tokens: JoinTokens,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ObjectVersion {
    #[serde(rename = "Index", default)]
    pub index: u64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct JoinTokens {
    #[serde(rename = "Worker", default)]
    pub worker: String,
    #[serde(rename = "Manager", default)]
    pub manager: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageSummary {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(
        rename = "RepoTags",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub repo_tags: Vec<String>,
    #[serde(
        rename = "RepoDigests",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub repo_digests: Vec<String>,
    #[serde(rename = "Created", default)]
    pub created: i64,
    #[serde(rename = "Size", default)]
    pub size: i64,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
    #[serde(rename = "Containers", default)]
    pub containers: i64,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageInspect {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(
        rename = "RepoTags",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub repo_tags: Vec<String>,
    #[serde(rename = "Size", default)]
    pub size: i64,
    #[serde(rename = "Created", default)]
    pub created: String,
    #[serde(rename = "Os", default)]
    pub os: String,
    #[serde(rename = "Architecture", default)]
    pub architecture: String,
    #[serde(
        rename = "Config",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub config: ImageConfig,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageConfig {
    #[serde(rename = "Env", default, deserialize_with = "deserialize_null_default")]
    pub env: Vec<String>,
    #[serde(rename = "Cmd", default, deserialize_with = "deserialize_null_default")]
    pub cmd: Vec<String>,
    #[serde(
        rename = "Volumes",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub volumes: HashMap<String, Value>,
    #[serde(
        rename = "ExposedPorts",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub exposed_ports: HashMap<String, Value>,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageHistoryItem {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(rename = "Created", default)]
    pub created: i64,
    #[serde(rename = "CreatedBy", default)]
    pub created_by: String,
    #[serde(rename = "Size", default)]
    pub size: i64,
    #[serde(rename = "Comment", default)]
    pub comment: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageUsageContainer {
    #[serde(flatten)]
    pub summary: ContainerSummary,
    #[serde(
        rename = "Mounts",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub mounts: Vec<ImageUsageMount>,
    #[serde(
        rename = "NetworkSettings",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub network_settings: ImageUsageNetworks,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageUsageMount {
    #[serde(rename = "Name", default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageUsageNetworks {
    #[serde(
        rename = "Networks",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub networks: HashMap<String, ImageUsageNetwork>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ImageUsageNetwork {
    #[serde(rename = "NetworkID", default)]
    pub network_id: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VolumeListResponse {
    #[serde(
        rename = "Volumes",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub volumes: Vec<DockerVolume>,
    #[serde(
        rename = "Warnings",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerVolume {
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Driver", default)]
    pub driver: String,
    #[serde(rename = "Mountpoint", default)]
    pub mountpoint: String,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(
        rename = "Status",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub status: HashMap<String, Value>,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
    #[serde(rename = "Scope", default)]
    pub scope: String,
    #[serde(rename = "ClusterVolume", default)]
    pub cluster_volume: Option<Value>,
    #[serde(
        rename = "Options",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub options: HashMap<String, String>,
    #[serde(rename = "UsageData", default)]
    pub usage_data: Option<Value>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct VolumeCreateOptions {
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Driver", default)]
    pub driver: String,
    #[serde(rename = "DriverOpts", default)]
    pub driver_options: HashMap<String, String>,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct DockerNetwork {
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(rename = "Created", default)]
    pub created: String,
    #[serde(rename = "Scope", default)]
    pub scope: String,
    #[serde(rename = "Driver", default)]
    pub driver: String,
    #[serde(rename = "EnableIPv4", default)]
    pub enable_ipv4: bool,
    #[serde(rename = "EnableIPv6", default)]
    pub enable_ipv6: bool,
    #[serde(rename = "IPAM", default)]
    pub ipam: Option<Value>,
    #[serde(rename = "Internal", default)]
    pub internal: bool,
    #[serde(rename = "Attachable", default)]
    pub attachable: bool,
    #[serde(rename = "Ingress", default)]
    pub ingress: bool,
    #[serde(rename = "ConfigFrom", default)]
    pub config_from: Option<Value>,
    #[serde(rename = "ConfigOnly", default)]
    pub config_only: bool,
    #[serde(
        rename = "Containers",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub containers: HashMap<String, Value>,
    #[serde(
        rename = "Peers",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub peers: Vec<Value>,
    #[serde(rename = "Options", default)]
    pub options: HashMap<String, String>,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkCreateRequest {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Driver", skip_serializing_if = "Option::is_none")]
    pub driver: Option<String>,
    #[serde(rename = "Scope", skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    #[serde(rename = "Internal", skip_serializing_if = "Option::is_none")]
    pub internal: Option<bool>,
    #[serde(rename = "Attachable", skip_serializing_if = "Option::is_none")]
    pub attachable: Option<bool>,
    #[serde(rename = "Ingress", skip_serializing_if = "Option::is_none")]
    pub ingress: Option<bool>,
    #[serde(rename = "EnableIPv6", skip_serializing_if = "Option::is_none")]
    pub enable_ipv6: Option<bool>,
    #[serde(rename = "EnableIPv4", skip_serializing_if = "Option::is_none")]
    pub enable_ipv4: Option<bool>,
    #[serde(rename = "ConfigOnly", skip_serializing_if = "Option::is_none")]
    pub config_only: Option<bool>,
    #[serde(rename = "IPAM", skip_serializing_if = "Option::is_none")]
    pub ipam: Option<Value>,
    #[serde(rename = "ConfigFrom", skip_serializing_if = "Option::is_none")]
    pub config_from: Option<Value>,
    #[serde(
        rename = "Labels",
        default,
        deserialize_with = "deserialize_null_default"
    )]
    pub labels: HashMap<String, String>,
    #[serde(rename = "Options", default)]
    pub options: HashMap<String, String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct NetworkCreateResponse {
    #[serde(rename = "Id", default)]
    pub id: String,
    #[serde(rename = "Warning", default)]
    pub warning: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmNode {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Version", default)]
    pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "Spec", default)]
    pub spec: Value,
    #[serde(rename = "Description", default)]
    pub description: Value,
    #[serde(rename = "Status", default)]
    pub status: Value,
    #[serde(rename = "ManagerStatus", default)]
    pub manager_status: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmService {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Version", default)]
    pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "Spec", default)]
    pub spec: Value,
    #[serde(rename = "Endpoint", default)]
    pub endpoint: Value,
    #[serde(rename = "UpdateStatus", default)]
    pub update_status: Value,
    #[serde(rename = "ServiceStatus", default)]
    pub service_status: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmTask {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Version", default)]
    pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "Name", default)]
    pub name: String,
    #[serde(rename = "Spec", default)]
    pub spec: Value,
    #[serde(rename = "ServiceID", default)]
    pub service_id: String,
    #[serde(rename = "Slot", default)]
    pub slot: Option<i32>,
    #[serde(rename = "NodeID", default)]
    pub node_id: String,
    #[serde(rename = "Status", default)]
    pub status: Value,
    #[serde(rename = "DesiredState", default)]
    pub desired_state: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmSecret {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Version", default)]
    pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "Spec", default)]
    pub spec: Value,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct SwarmConfig {
    #[serde(rename = "ID", default)]
    pub id: String,
    #[serde(rename = "Version", default)]
    pub version: ObjectVersion,
    #[serde(rename = "CreatedAt", default)]
    pub created_at: String,
    #[serde(rename = "UpdatedAt", default)]
    pub updated_at: String,
    #[serde(rename = "Spec", default)]
    pub spec: Value,
}
