use std::collections::BTreeMap;
use std::sync::Arc;

use chrono::{DateTime, Utc};
use citadel_domain::ActorId;
use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use uuid::Uuid;

use crate::AuthorizedReadError;

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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_inspect: bool,
    pub can_pull: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_inspect: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct VolumeCapabilitiesView {
    pub can_read: bool,
    pub can_write: bool,
    pub can_execute: bool,
    pub can_inspect: bool,
    pub can_browse: bool,
    pub can_download: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EffectivePlatformPermission {
    pub level_mask: i32,
    pub specific_mask: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Serialize)]
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
    pub capabilities: Option<PlatformCapabilitiesView>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
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
    pub usage_data: Option<Value>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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

pub trait PlatformReadStore: Send + Sync {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        is_administrator: bool,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<PlatformView>, AuthorizedReadError>>;

    fn get_platform(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<PlatformView>, AuthorizedReadError>>;

    fn permissions_for_platforms<'a>(
        &'a self,
        actor_id: ActorId,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<BTreeMap<Uuid, EffectivePlatformPermission>, AuthorizedReadError>>;

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, AuthorizedReadError>>;

    fn get_container(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ContainerView>, AuthorizedReadError>>;

    fn list_images(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ImageView>, AuthorizedReadError>>;

    fn list_swarm_nodes(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNodeView>, AuthorizedReadError>>;

    fn get_swarm_node<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNodeView>, AuthorizedReadError>>;

    fn list_swarm_services(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmServiceView>, AuthorizedReadError>>;

    fn get_swarm_service<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmServiceView>, AuthorizedReadError>>;

    fn list_swarm_tasks<'a>(
        &'a self,
        platform_id: Uuid,
        service_id: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<SwarmTaskView>, AuthorizedReadError>>;

    fn get_swarm_task<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmTaskView>, AuthorizedReadError>>;

    fn list_swarm_configs(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmConfigView>, AuthorizedReadError>>;

    fn get_swarm_config<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmConfigView>, AuthorizedReadError>>;

    fn list_swarm_networks(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNetworkView>, AuthorizedReadError>>;

    fn get_swarm_network<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNetworkView>, AuthorizedReadError>>;

    fn list_swarm_secrets(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmSecretView>, AuthorizedReadError>>;

    fn get_swarm_secret<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmSecretView>, AuthorizedReadError>>;
}

#[derive(Clone)]
pub struct PlatformReadService {
    store: Arc<dyn PlatformReadStore>,
}

impl PlatformReadService {
    #[must_use]
    pub fn new(store: Arc<dyn PlatformReadStore>) -> Self {
        Self { store }
    }

    pub async fn list_authorized(
        &self,
        actor_id: ActorId,
        is_administrator: bool,
        tag_ids: &[Uuid],
    ) -> Result<Vec<PlatformView>, AuthorizedReadError> {
        self.store
            .list_authorized(actor_id, is_administrator, tag_ids)
            .await
    }

    pub async fn get_platform(
        &self,
        id: Uuid,
    ) -> Result<Option<PlatformView>, AuthorizedReadError> {
        self.store.get_platform(id).await
    }

    pub async fn permissions_for_platforms(
        &self,
        actor_id: ActorId,
        platform_ids: &[Uuid],
    ) -> Result<BTreeMap<Uuid, EffectivePlatformPermission>, AuthorizedReadError> {
        self.store
            .permissions_for_platforms(actor_id, platform_ids)
            .await
    }

    pub async fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<ContainerView>, AuthorizedReadError> {
        self.store.list_containers(platform_id).await
    }

    pub async fn get_container(
        &self,
        id: Uuid,
    ) -> Result<Option<ContainerView>, AuthorizedReadError> {
        self.store.get_container(id).await
    }

    pub async fn list_images(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<ImageView>, AuthorizedReadError> {
        self.store.list_images(platform_id).await
    }

    pub async fn list_swarm_nodes(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmNodeView>, AuthorizedReadError> {
        self.store.list_swarm_nodes(platform_id).await
    }

    pub async fn get_swarm_node(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmNodeView>, AuthorizedReadError> {
        self.store.get_swarm_node(platform_id, id).await
    }

    pub async fn list_swarm_services(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmServiceView>, AuthorizedReadError> {
        self.store.list_swarm_services(platform_id).await
    }

    pub async fn get_swarm_service(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmServiceView>, AuthorizedReadError> {
        self.store.get_swarm_service(platform_id, id).await
    }

    pub async fn list_swarm_tasks(
        &self,
        platform_id: Uuid,
        service_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<SwarmTaskView>, AuthorizedReadError> {
        self.store
            .list_swarm_tasks(platform_id, service_id, limit)
            .await
    }

    pub async fn get_swarm_task(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmTaskView>, AuthorizedReadError> {
        self.store.get_swarm_task(platform_id, id).await
    }

    pub async fn list_swarm_configs(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmConfigView>, AuthorizedReadError> {
        self.store.list_swarm_configs(platform_id).await
    }

    pub async fn get_swarm_config(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmConfigView>, AuthorizedReadError> {
        self.store.get_swarm_config(platform_id, id).await
    }

    pub async fn list_swarm_networks(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmNetworkView>, AuthorizedReadError> {
        self.store.list_swarm_networks(platform_id).await
    }

    pub async fn get_swarm_network(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmNetworkView>, AuthorizedReadError> {
        self.store.get_swarm_network(platform_id, id).await
    }

    pub async fn list_swarm_secrets(
        &self,
        platform_id: Uuid,
    ) -> Result<Vec<SwarmSecretView>, AuthorizedReadError> {
        self.store.list_swarm_secrets(platform_id).await
    }

    pub async fn get_swarm_secret(
        &self,
        platform_id: Uuid,
        id: &str,
    ) -> Result<Option<SwarmSecretView>, AuthorizedReadError> {
        self.store.get_swarm_secret(platform_id, id).await
    }
}
