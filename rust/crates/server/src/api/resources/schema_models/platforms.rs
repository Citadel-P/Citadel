//! Server-owned OpenAPI descriptions of platforms values.
//! Field types and exhaustive conversions keep these descriptions tied to the domain.

use chrono::{DateTime, Utc};
use serde_json::Value;
use std::collections::BTreeMap;
use uuid::Uuid;

schema_model! {
    citadel_platforms::node_agents::NodeAgentOperation =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = NodeAgentOperation)]
    pub struct NodeAgentOperationSchema {
        pub operation_id: Uuid,
        pub kind: String,
        pub state: String,
        pub started_at_utc: DateTime<Utc>,
        pub error: Option<String>,
    }
}

schema_model! {
    citadel_platforms::node_agents::NodeAgentCoverage =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = NodeAgentCoverage)]
    pub struct NodeAgentCoverageSchema {
        pub docker_node_id: String,
        pub hostname: String,
        pub role: String,
        pub availability: String,
        pub node_status: String,
        pub architecture: String,
        pub data_source: &'static str,
        pub eligible: bool,
        pub supported: bool,
        pub schedulable: bool,
        pub service_task_state: Option<String>,
        pub agent_connection_state: &'static str,
        pub docker_reachable: bool,
        pub compatible: bool,
        pub projection_stale: bool,
        pub last_heartbeat_at_utc: Option<DateTime<Utc>>,
        pub last_successful_reconciliation_at: Option<DateTime<Utc>>,
        pub stale_since: Option<DateTime<Utc>>,
        pub stale_reason: Option<String>,
        pub reasons: Vec<&'static str>,
    }
}

schema_model! {
    citadel_platforms::node_agents::SwarmNodeAgentCoverage =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = SwarmNodeAgentCoverage)]
    pub struct SwarmNodeAgentCoverageSchema {
        pub state: &'static str,
        pub is_installed: bool,
        pub covered_nodes: usize,
        pub eligible_nodes: usize,
        pub total_nodes: usize,
        pub connected_nodes: usize,
        pub offline_nodes: usize,
        pub enrolling_nodes: usize,
        pub missing_nodes: usize,
        pub incompatible_nodes: usize,
        pub unsupported_nodes: usize,
        pub unschedulable_nodes: usize,
        pub stale_nodes: usize,
        pub last_membership_reconciliation_at_utc: Option<DateTime<Utc>>,
        pub agent_image_reference: Option<String>,
        pub agent_image_digest: Option<String>,
        pub enrollment_expires_at_utc: Option<DateTime<Utc>>,
        #[schema(value_type = Option < crate::api::resources::schema_models::platforms::NodeAgentOperationSchema >)]
        pub operation: Option<citadel_platforms::node_agents::NodeAgentOperation>,
        pub reasons: Vec<&'static str>,
        #[schema(value_type = Vec < crate::api::resources::schema_models::platforms::NodeAgentCoverageSchema >)]
        pub nodes: Vec<citadel_platforms::node_agents::NodeAgentCoverage>,
        pub can_manage_node_agents: bool,
    }
}

schema_model! {
    citadel_platforms::logs::LogSnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[schema(as = LogSnapshot)]
    pub struct LogSnapshotSchema {
        pub lines: Vec<String>,
        pub truncated: bool,
    }
}

schema_model! {
    citadel_platforms::images::ImageInspection =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ImageInspection)]
    pub struct ImageInspectionSchema {
        pub id: String,
        pub name: String,
        pub tag: String,
        pub size: i64,
        pub os: String,
        pub created: String,
        pub architecture: String,
        pub user: Option<String>,
        pub working_dir: Option<String>,
        pub entry_point: Vec<String>,
        pub stop_signal: Option<String>,
        pub env: Vec<String>,
        pub cmd: Vec<String>,
        pub repo_tags: Vec<String>,
        pub volumes: Vec<String>,
        pub exposed_ports: Vec<String>,
        #[schema(value_type = Vec < crate::api::resources::schema_models::platforms::ImageLayerSchema >)]
        pub layers: Vec<citadel_platforms::images::ImageLayer>,
        pub labels: BTreeMap<String, String>,
        #[schema(value_type = Vec < crate::api::resources::schema_models::platforms::ImageContainerSchema >)]
        pub containers: Vec<citadel_platforms::images::ImageContainer>,
        pub registry: Option<Value>,
        pub docker_node_id: Option<String>,
    }
}

schema_model! {
    citadel_platforms::images::ImageLayer =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ImageLayer)]
    pub struct ImageLayerSchema {
        pub id: String,
        pub created: i64,
        pub created_by: String,
        pub size: i64,
        pub comment: String,
    }
}

schema_model! {
    citadel_platforms::images::ImageContainer =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ImageContainer)]
    pub struct ImageContainerSchema {
        pub id: String,
        pub name: String,
        pub state: String,
        pub volumes: Vec<String>,
        pub networks: BTreeMap<String, String>,
        pub ports: Value,
    }
}

schema_model! {
    citadel_platforms::RuntimeSwarmPeer =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = RuntimeSwarmPeer)]
    pub struct RuntimeSwarmPeerSchema {
        pub node_id: String,
        pub address: String,
    }
}

enum_schema!(
    VolumeEntryTypeSchema,
    "VolumeEntryType",
    citadel_platforms::volume_content::VolumeEntryType,
    [File, Directory, Symlink, Other]
);

schema_model! {
    citadel_platforms::volume_content::VolumeFileEntry =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all(serialize = "camelCase", deserialize = "PascalCase"))]
    #[schema(rename_all = "camelCase")]
    #[schema(as = VolumeFileEntry)]
    pub struct VolumeFileEntrySchema {
        pub name: String,
        pub path: String,
        #[serde(rename = "type", alias = "Type")]
        #[schema(value_type = crate::api::resources::schema_models::platforms::VolumeEntryTypeSchema)]
        pub entry_type: citadel_platforms::volume_content::VolumeEntryType,
        pub size: Option<u64>,
        pub modified_at: Option<chrono::DateTime<chrono::Utc>>,
        pub link_target: Option<String>,
    }
}

schema_model! {
    citadel_platforms::volume_content::VolumeDirectory =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = VolumeDirectory)]
    pub struct VolumeDirectorySchema {
        pub platform_id: Uuid,
        pub volume_name: String,
        pub path: String,
        #[schema(value_type = Vec < crate::api::resources::schema_models::platforms::VolumeFileEntrySchema >)]
        pub entries: Vec<citadel_platforms::volume_content::VolumeFileEntry>,
        pub is_truncated: bool,
    }
}

schema_model! {
    citadel_platforms::PlatformStatSnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = PlatformStatSnapshot)]
    pub struct PlatformStatSnapshotSchema {
        pub created: i64,
        pub tx_bytes: f64,
        pub rx_bytes: f64,
        pub cpu_usage: f64,
        pub memory_usage: f64,
        #[schema(required = true)]
        pub disk_used_bytes: Option<i64>,
        #[schema(required = true)]
        pub disk_total_bytes: Option<i64>,
        #[schema(required = true)]
        pub disk_usage: Option<f64>,
    }
}

schema_model! {
    citadel_platforms::ContainerStatSnapshot =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ContainerStatSnapshot)]
    pub struct ContainerStatSnapshotSchema {
        pub container_id: Uuid,
        pub memory_active: f64,
        pub memory_cache: f64,
        pub cpu_usage: f64,
        pub memory_limit: f64,
        pub rx_bytes: f64,
        pub tx_bytes: f64,
        pub created: i64,
    }
}

schema_model! {
    citadel_platforms::ServiceStatistics =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ServiceStatistics)]
    pub struct ServiceStatisticsSchema {
        pub docker_service_id: String,
        pub observed_tasks: usize,
        pub expected_tasks: usize,
        pub complete: bool,
        pub observed_container_projection_ids: Vec<Uuid>,
        pub missing_docker_node_ids: Vec<String>,
        pub oldest_sample_at: Option<chrono::DateTime<chrono::Utc>>,
        pub newest_sample_at: Option<chrono::DateTime<chrono::Utc>>,
        #[schema(value_type = Vec < crate::api::resources::schema_models::platforms::ContainerStatSnapshotSchema >)]
        pub stats: Vec<citadel_platforms::ContainerStatSnapshot>,
    }
}

schema_model! {
    citadel_platforms::swarm_mutations::CreateSwarmMaterialInput =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = CreateSwarmMaterialInput)]
    pub struct CreateSwarmMaterialInputSchema {
        pub name: String,
        #[serde(default)]
        pub data: String,
        #[serde(default)]
        pub labels: BTreeMap<String, String>,
    }
}

schema_model! {
    citadel_platforms::image_pull::PullImageStreamItem =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = PullImageStreamItem)]
    pub struct PullImageStreamItemSchema {
        pub id: Option<String>,
        pub from: Option<String>,
        pub stream: Option<String>,
        pub status: Option<String>,
        pub error_message: Option<String>,
        pub progress_message: Option<String>,
        pub docker_image_id: Option<String>,
        pub digest: Option<String>,
        #[schema(value_type = Option < crate::api::resources::schema_models::platforms::ImagePullProgressSchema >)]
        pub progress: Option<citadel_platforms::image_pull::ImagePullProgress>,
        #[schema(value_type = Option < crate::api::resources::schema_models::platforms::ImagePullErrorSchema >)]
        pub error: Option<citadel_platforms::image_pull::ImagePullError>,
    }
}

schema_model! {
    citadel_platforms::image_pull::ImagePullProgress =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    # [schema (as = platforms :: image_pull :: ImagePullProgress)]
    pub struct ImagePullProgressSchema {
        pub units: Option<String>,
        pub current: Option<i64>,
        pub total: Option<i64>,
        pub start: Option<i64>,
    }
}

schema_model! {
    citadel_platforms::image_pull::ImagePullError =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ImagePullError)]
    pub struct ImagePullErrorSchema {
        pub code: Option<i64>,
        pub message: Option<String>,
    }
}

schema_model! {
    citadel_platforms::node_agents::lifecycle::NodeAgentProgress =>
    #[derive(serde::Serialize, utoipa::ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = NodeAgentProgress)]
    pub struct NodeAgentProgressSchema {
        pub platform_id: Uuid,
        pub operation_id: Uuid,
        pub stage: &'static str,
        pub message: String,
        pub is_completed: bool,
        pub is_warning: bool,
        pub error_message: Option<String>,
    }
}
