//! Actor-visible network relationships. Configuration/project references are not attachments.
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
pub struct NetworkUsageQuery {
    pub id: String,
    pub name: String,
    pub scope: String,
    pub docker_node_id: Option<String>,
    pub container_ids: Vec<String>,
    /// Docker-reported Compose project or Swarm stack namespace, not an attachment.
    pub stack_project: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkRelationship {
    Attachment,
    ServiceNetwork,
    SavedReference,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkWorkloadKind {
    Deployment,
    Stack,
    SwarmService,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsageEntry {
    pub relationship: NetworkRelationship,
    pub docker_service_id: Option<String>,
    pub service_name: Option<String>,
    pub observed_at: Option<i64>,
    pub is_stale: bool,
    pub container_id: Option<Uuid>,
    pub docker_container_id: Option<String>,
    pub docker_node_id: Option<String>,
    pub container_name: Option<String>,
    pub container_state: Option<String>,
    pub workload_kind: Option<NetworkWorkloadKind>,
    pub workload_id: Option<Uuid>,
    pub workload_name: Option<String>,
    pub workload_status: Option<String>,
    pub workload_processing: bool,
    pub reference: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkUsage {
    pub entries: Vec<NetworkUsageEntry>,
    /// Includes unresolved names, missing observations and actor-limited visibility.
    pub is_partial: bool,
    pub truncated: bool,
}

/// Authorized runtime identities for selecting a workload, without its configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkContainer {
    pub container_id: Uuid,
    pub docker_container_id: String,
    pub docker_node_id: Option<String>,
    pub container_name: String,
    pub container_state: Option<String>,
    pub is_stale: bool,
    pub workload_kind: Option<NetworkWorkloadKind>,
    pub workload_id: Option<Uuid>,
    pub workload_name: Option<String>,
    pub workload_status: Option<String>,
    pub workload_processing: bool,
}
