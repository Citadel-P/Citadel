use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::Serialize;
use serde_json::Value;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{PlatformRuntimePort, RuntimeCapabilityError};
use citadel_domain::SwarmServiceOwnership;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeImageSummary {
    pub id: String,
    pub repo_tags: Vec<String>,
    pub repo_digests: Vec<String>,
    pub created: i64,
    pub size: i64,
    pub containers: i64,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeNetworkSummary {
    pub id: String,
    pub name: String,
    pub created: String,
    pub driver: String,
    pub scope: String,
    pub enable_ipv4: bool,
    pub enable_ipv6: bool,
    pub internal: bool,
    pub attachable: bool,
    pub ingress: bool,
    pub config_only: bool,
    pub config_from: Option<String>,
    pub ipam: Option<Value>,
    pub options: BTreeMap<String, String>,
    pub labels: BTreeMap<String, String>,
    pub container_count: usize,
    pub containers: BTreeMap<String, Value>,
    pub peers: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeVolumeSummary {
    pub name: String,
    pub in_use: bool,
    pub scope: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: String,
    pub cluster_volume: Option<Value>,
    pub usage_data: Option<Value>,
    pub status: BTreeMap<String, Value>,
    pub labels: BTreeMap<String, String>,
    pub options: BTreeMap<String, String>,
    pub containers: Vec<Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeSwarmNode {
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
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeSwarmService {
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
    pub stack_namespace: Option<String>,
    pub ownership: SwarmServiceOwnership,
    pub ownership_diagnostic: Option<String>,
    pub swarm_service_id: Option<Uuid>,
    pub stack_id: Option<Uuid>,
    pub force_update: i64,
    pub runtime_hash: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl RuntimeSwarmService {
    #[must_use]
    pub fn normalize_ownership(mut self) -> Self {
        const MANAGED: &str = "com.citadel.managed";
        const DEPLOYMENT_ID: &str = "com.citadel.deployment-id";
        const STACK_ID: &str = "com.citadel.stack-id";
        const SERVICE_ID: &str = "com.citadel.service-id";
        const SYSTEM: &str = "com.citadel.system";
        const SYSTEM_ROLE: &str = "com.citadel.system-role";

        self.swarm_service_id = self
            .labels
            .get(SERVICE_ID)
            .and_then(|value| Uuid::parse_str(value).ok());
        self.stack_id = self
            .labels
            .get(STACK_ID)
            .and_then(|value| Uuid::parse_str(value).ok());
        self.stack_namespace = self
            .labels
            .get("com.docker.stack.namespace")
            .filter(|value| !value.trim().is_empty())
            .cloned();

        let managed = self
            .labels
            .get(MANAGED)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        let system = self
            .labels
            .get(SYSTEM)
            .is_some_and(|value| value.eq_ignore_ascii_case("true"));
        let has_service = self.labels.contains_key(SERVICE_ID);
        let has_stack = self.labels.contains_key(STACK_ID);
        let has_other_owner = self.labels.contains_key(DEPLOYMENT_ID) || has_stack;

        let (ownership, diagnostic) = if system && self.labels.contains_key(SYSTEM_ROLE) {
            (SwarmServiceOwnership::System, None)
        } else if !managed {
            (
                if self.stack_namespace.is_some() {
                    SwarmServiceOwnership::DockerStackExternal
                } else {
                    SwarmServiceOwnership::Unmanaged
                },
                None,
            )
        } else if has_service {
            if self.swarm_service_id.is_none() {
                (
                    SwarmServiceOwnership::OwnershipConflict,
                    Some("Invalid Citadel Service ownership label".to_owned()),
                )
            } else if has_other_owner {
                (
                    SwarmServiceOwnership::OwnershipConflict,
                    Some("Conflicting Citadel ownership labels".to_owned()),
                )
            } else {
                (SwarmServiceOwnership::CitadelService, None)
            }
        } else if has_stack {
            if self.stack_id.is_none() {
                (
                    SwarmServiceOwnership::OwnershipConflict,
                    Some("Invalid Citadel Stack ownership label".to_owned()),
                )
            } else if self.stack_namespace.is_none() {
                (
                    SwarmServiceOwnership::OwnershipConflict,
                    Some(
                        "Citadel Stack ownership is missing the Docker Stack namespace".to_owned(),
                    ),
                )
            } else {
                (SwarmServiceOwnership::CitadelStack, None)
            }
        } else if self.stack_namespace.is_some() {
            (SwarmServiceOwnership::DockerStackExternal, None)
        } else {
            (
                SwarmServiceOwnership::Unmanaged,
                Some("Orphaned Citadel metadata".to_owned()),
            )
        };
        self.ownership = ownership;
        self.ownership_diagnostic = diagnostic;
        self
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeSwarmTask {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub service_id: String,
    pub slot: Option<i32>,
    pub node_id: String,
    pub desired_state: String,
    pub state: String,
    pub status_message: Option<String>,
    pub error: Option<String>,
    pub image: String,
    pub ports: Vec<String>,
    pub container_id: Option<String>,
    pub status_timestamp: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeSwarmConfig {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub templating_driver: Option<String>,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct RuntimeSwarmSecret {
    pub id: String,
    pub version_index: i64,
    pub name: String,
    pub driver: Option<String>,
    pub labels: BTreeMap<String, String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RuntimeSwarmInventory {
    pub nodes: Vec<RuntimeSwarmNode>,
    pub services: Vec<RuntimeSwarmService>,
    pub tasks: Vec<RuntimeSwarmTask>,
    pub configs: Vec<RuntimeSwarmConfig>,
    pub secrets: Vec<RuntimeSwarmSecret>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RuntimeInventorySnapshot {
    pub platform_id: Uuid,
    pub info: crate::RuntimePlatformInfo,
    pub containers: Vec<crate::RuntimeContainerSummary>,
    pub images: Vec<RuntimeImageSummary>,
    pub networks: Vec<RuntimeNetworkSummary>,
    pub volumes: Vec<RuntimeVolumeSummary>,
    pub swarm: Option<RuntimeSwarmInventory>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InventoryProjectionChange {
    pub platform_id: Uuid,
    pub revision: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeContainerStat {
    pub docker_container_id: String,
    pub memory_active: f64,
    pub memory_cache: f64,
    pub cpu_usage: f64,
    pub memory_limit: f64,
    pub rx_bytes: f64,
    pub tx_bytes: f64,
    pub created: i64,
}

pub type RuntimeContainerStatsStream = std::pin::Pin<
    Box<
        dyn futures_util::Stream<Item = Result<Vec<RuntimeContainerStat>, RuntimeCapabilityError>>
            + Send,
    >,
>;

pub trait ContainerStatsStore: Send + Sync {
    fn persist<'a>(
        &'a self,
        platform_id: Uuid,
        stats: &'a [RuntimeContainerStat],
    ) -> BoxFuture<'a, Result<usize, RuntimeCapabilityError>>;
}

pub trait InventoryProjectionStore: Send + Sync {
    fn persist<'a>(
        &'a self,
        snapshot: &'a RuntimeInventorySnapshot,
    ) -> BoxFuture<'a, Result<InventoryProjectionChange, RuntimeCapabilityError>>;
}

pub trait PlatformInventoryPort: PlatformRuntimePort {
    fn list_images<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeImageSummary>, RuntimeCapabilityError>>;

    fn list_networks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeNetworkSummary>, RuntimeCapabilityError>>;

    fn inspect_network<'a>(
        &'a self,
        id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeNetworkSummary, RuntimeCapabilityError>>;

    fn list_volumes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeVolumeSummary>, RuntimeCapabilityError>>;

    fn inspect_volume<'a>(
        &'a self,
        name: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<RuntimeVolumeSummary, RuntimeCapabilityError>>;

    fn list_swarm_nodes<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmNode>, RuntimeCapabilityError>>;

    fn list_swarm_services<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmService>, RuntimeCapabilityError>>;

    fn list_swarm_tasks<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmTask>, RuntimeCapabilityError>>;

    fn list_swarm_configs<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmConfig>, RuntimeCapabilityError>>;

    fn list_swarm_secrets<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<RuntimeSwarmSecret>, RuntimeCapabilityError>>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn service_ownership_matches_existing_citadel_label_rules() {
        let service_id = Uuid::now_v7();
        let managed = RuntimeSwarmService {
            labels: BTreeMap::from([
                ("com.citadel.managed".to_owned(), "true".to_owned()),
                ("com.citadel.service-id".to_owned(), service_id.to_string()),
            ]),
            ..RuntimeSwarmService::default()
        }
        .normalize_ownership();
        assert_eq!(managed.ownership, SwarmServiceOwnership::CitadelService);
        assert_eq!(managed.swarm_service_id, Some(service_id));
        assert_eq!(managed.ownership_diagnostic, None);

        let external_stack = RuntimeSwarmService {
            labels: BTreeMap::from([("com.docker.stack.namespace".to_owned(), "demo".to_owned())]),
            ..RuntimeSwarmService::default()
        }
        .normalize_ownership();
        assert_eq!(
            external_stack.ownership,
            SwarmServiceOwnership::DockerStackExternal
        );
    }

    #[test]
    fn malformed_or_conflicting_ownership_is_not_treated_as_managed() {
        let malformed = RuntimeSwarmService {
            labels: BTreeMap::from([
                ("com.citadel.managed".to_owned(), "true".to_owned()),
                ("com.citadel.service-id".to_owned(), "not-a-uuid".to_owned()),
            ]),
            ..RuntimeSwarmService::default()
        }
        .normalize_ownership();
        assert_eq!(
            malformed.ownership,
            SwarmServiceOwnership::OwnershipConflict
        );
        assert!(malformed.ownership_diagnostic.is_some());
    }
}
