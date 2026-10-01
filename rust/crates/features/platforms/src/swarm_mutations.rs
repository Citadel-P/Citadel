//! Native Swarm inventory inputs. These operations do not create Citadel workloads.
use crate::{RuntimeCapabilityError, RuntimeErrorKind, RuntimeSwarmNode};
use chrono::{DateTime, Utc};
use futures_util::future::BoxFuture;
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use tokio_util::sync::CancellationToken;

/// Inventory resources that can be deleted directly from a Swarm manager.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwarmResourceKind {
    Service,
    Secret,
    Config,
}
impl SwarmResourceKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Service => "service",
            Self::Secret => "secret",
            Self::Config => "config",
        }
    }
}

/// Require the currently connected daemon to remain the pinned Swarm manager.
pub fn manager_matches(
    info: &crate::RuntimePlatformInfo,
    cluster_id: Option<&str>,
    descriptor: &crate::PlatformRoutingMetadata,
) -> bool {
    let manager = descriptor.node_id.as_deref();
    let daemon = descriptor.daemon_id.as_deref();
    info.swarm.as_ref().is_some_and(|swarm| {
        swarm.control_available
            && swarm.local_node_state.eq_ignore_ascii_case("active")
            && cluster_id
                .is_some_and(|id| !id.is_empty() && Some(id) == swarm.cluster_id.as_deref())
            && manager.is_some_and(|id| !id.is_empty() && id == swarm.node_id)
            && daemon.is_some_and(|id| !id.is_empty() && id == info.daemon_id)
    })
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmNodeInput {
    pub version_index: i64,
    pub availability: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwarmNodeAvailabilityTarget {
    pub node_id: String,
    pub version_index: i64,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmNodesAvailabilityInput {
    #[serde(default)]
    pub nodes: Vec<SwarmNodeAvailabilityTarget>,
    pub availability: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateSwarmMaterialInput {
    pub name: String,
    #[serde(default)]
    pub data: String,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateSwarmResourceLabelsInput {
    pub version_index: i64,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
}
#[derive(Deserialize)]
pub struct DeleteSwarmResourcesInput {
    #[serde(default)]
    pub ids: Vec<String>,
}
pub fn resource_id(id: &str) -> Result<(), &'static str> {
    if id.trim().is_empty() || id.len() > 255 || id.chars().any(char::is_control) {
        Err("A resource id of at most 255 characters is required.")
    } else {
        Ok(())
    }
}
pub fn availability(value: &str) -> Result<(), &'static str> {
    if ["Active", "Pause", "Drain"]
        .iter()
        .any(|v| v.eq_ignore_ascii_case(value))
    {
        Ok(())
    } else {
        Err("Availability must be Active, Pause, or Drain.")
    }
}
pub fn labels(values: &BTreeMap<String, String>) -> Result<(), &'static str> {
    if values.len() > 1000
        || values
            .iter()
            .any(|(key, value)| key.trim().is_empty() || key.len() > 255 || value.len() > 4096)
    {
        Err(
            "Labels must have nonempty keys of at most 255 characters and values of at most 4096 characters.",
        )
    } else {
        Ok(())
    }
}
pub fn version(value: i64) -> Result<(), &'static str> {
    if value < 0 {
        Err("Version index must not be negative.")
    } else {
        Ok(())
    }
}
impl UpdateSwarmNodeInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        version(self.version_index)?;
        availability(&self.availability)?;
        labels(&self.labels)
    }
}
impl UpdateSwarmNodesAvailabilityInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        availability(&self.availability)?;
        if self.nodes.is_empty() || self.nodes.len() > 100 {
            return Err("Select between 1 and 100 nodes.");
        }
        let mut ids = BTreeSet::new();
        for node in &self.nodes {
            resource_id(&node.node_id)?;
            version(node.version_index)?;
            if !ids.insert(&node.node_id) {
                return Err("Each node can only be selected once.");
            }
        }
        Ok(())
    }
}
impl CreateSwarmMaterialInput {
    pub fn validate(&self, secret: bool) -> Result<(), &'static str> {
        resource_id(&self.name)?;
        labels(&self.labels)?;
        if secret && self.data.is_empty() {
            return Err("Secret data is required.");
        }
        if self.data.len() > if secret { 500 * 1024 } else { 1000 * 1024 } {
            return Err("Resource data exceeds the permitted size.");
        }
        Ok(())
    }
}
impl UpdateSwarmResourceLabelsInput {
    pub fn validate(&self) -> Result<(), &'static str> {
        version(self.version_index)?;
        labels(&self.labels)
    }
}
impl DeleteSwarmResourcesInput {
    pub fn validate(&mut self) -> Result<(), &'static str> {
        if self.ids.is_empty() || self.ids.len() > 100 {
            return Err("Select between 1 and 100 resources.");
        }
        for id in &self.ids {
            resource_id(id)?;
        }
        let mut seen = BTreeSet::new();
        self.ids.retain(|id| seen.insert(id.clone()));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_utf8_payload_bytes_and_immutable_update_contract() {
        let input = CreateSwarmMaterialInput {
            name: "token".into(),
            data: "é".repeat(256001),
            labels: BTreeMap::new(),
        };
        assert!(input.validate(true).is_err());
        assert!(input.validate(false).is_ok());
        let labels: UpdateSwarmResourceLabelsInput =
            serde_json::from_str(r#"{"versionIndex":2,"labels":{},"data":"ignored"}"#).unwrap();
        assert!(labels.validate().is_ok());
    }
    #[test]
    fn validates_entire_selection_and_versions() {
        let input: UpdateSwarmNodesAvailabilityInput = serde_json::from_str(r#"{"availability":"Drain","nodes":[{"nodeId":"n","versionIndex":1},{"nodeId":"n","versionIndex":2}]}"#).unwrap();
        assert!(input.validate().is_err());
        assert!(version(-1).is_err());
        assert!(availability("pause").is_ok());
        assert!(availability("stop").is_err());
        let mut input = DeleteSwarmResourcesInput {
            ids: vec!["s".into(), "s".into()],
        };
        input.validate().unwrap();
        assert_eq!(input.ids, ["s"]);
    }
}

#[derive(Debug, Clone)]
pub struct SwarmServiceInspection {
    pub id: String,
    pub version_index: u64,
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
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

/// Native Swarm control and inspection; transport details remain in adapters.
pub trait SwarmControlPort: Send + Sync {
    fn inspect_service<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<SwarmServiceInspection, RuntimeCapabilityError>>;
    fn inspect_node<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(RuntimeSwarmNode, i32, i32), RuntimeCapabilityError>>;
    fn update_node<'a>(
        &'a self,
        id: &'a str,
        input: &'a UpdateSwarmNodeInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn restart_service<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn delete_service<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn create_material<'a>(
        &'a self,
        secret: bool,
        input: &'a CreateSwarmMaterialInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn update_labels<'a>(
        &'a self,
        secret: bool,
        id: &'a str,
        input: &'a UpdateSwarmResourceLabelsInput,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn delete_material<'a>(
        &'a self,
        secret: bool,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;
    fn config_data<'a>(
        &'a self,
        id: &'a str,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<String, RuntimeCapabilityError>>;
}

pub async fn manager_identity(
    runtime: &(impl crate::PlatformInfoPort + ?Sized),
    platform: &crate::PlatformDetails,
    cancel: &CancellationToken,
) -> Result<(), RuntimeCapabilityError> {
    let info = runtime.get_info(cancel).await?;
    if !manager_matches(
        &info,
        platform.cluster_id.as_deref(),
        &platform.platform_descriptor.routing,
    ) {
        return Err(RuntimeCapabilityError::new(
            RuntimeErrorKind::Conflict,
            "The connected Docker manager no longer belongs to this Swarm platform.",
            false,
        ));
    }
    Ok(())
}

pub fn partial(
    error: RuntimeCapabilityError,
    completed: usize,
    total: usize,
    verb: &str,
) -> RuntimeCapabilityError {
    if completed == 0 {
        error
    } else {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Conflict,
            format!(
                "{verb} {completed} of {total} resources before Docker rejected the operation: {error}"
            ),
            false,
        )
    }
}

pub fn check_node(
    node: &crate::SwarmNodeSummary,
    version: i64,
) -> Result<(), crate::AuthorizedReadError> {
    if node.is_stale {
        Err(read_conflict(
            "Node inventory is stale. Refresh before making changes.",
        ))
    } else if node.version_index != version {
        Err(read_conflict("Node changed. Reload it before saving."))
    } else {
        Ok(())
    }
}

fn read_conflict(message: &str) -> crate::AuthorizedReadError {
    crate::AuthorizedReadError::Conflict(message.into())
}

pub async fn service_guard(
    reads: &crate::PlatformReadService,
    services: &dyn citadel_swarm_services::SwarmServiceRepository,
    pid: uuid::Uuid,
    id: &str,
) -> Result<(), crate::AuthorizedReadError> {
    let service = reads
        .get_swarm_service(pid, id)
        .await?
        .ok_or(crate::AuthorizedReadError::NotFound)?;
    if service.is_stale {
        return Err(read_conflict("Service inventory is stale."));
    }
    let managed = services
        .owns_runtime_service(pid, id)
        .await
        .map_err(|e| crate::AuthorizedReadError::Storage(e.to_string()))?;
    if managed || service.ownership == "System" {
        return Err(read_conflict(
            "This Service must be changed through its Citadel owner.",
        ));
    }
    Ok(())
}
pub async fn material_guard(
    reads: &crate::PlatformReadService,
    pid: uuid::Uuid,
    id: &str,
    secret: bool,
    deleting: bool,
    version: Option<i64>,
) -> Result<(), crate::AuthorizedReadError> {
    let (stale, in_use, current) = if secret {
        let r = reads
            .get_swarm_secret(pid, id)
            .await?
            .ok_or(crate::AuthorizedReadError::NotFound)?;
        (r.is_stale, r.in_use, r.version_index)
    } else {
        let r = reads
            .get_swarm_config(pid, id)
            .await?
            .ok_or(crate::AuthorizedReadError::NotFound)?;
        (r.is_stale, r.in_use, r.version_index)
    };
    if stale || (deleting && in_use) || version.is_some_and(|v| v != current) {
        return Err(read_conflict(if stale {
            "Resource inventory is stale."
        } else if deleting && in_use {
            "Resource is in use and cannot be deleted."
        } else {
            "Resource changed. Reload it before saving."
        }));
    }
    Ok(())
}

/// Resolves and collects a manager snapshot without exposing its transport.
pub trait SwarmSnapshotPort: Send + Sync {
    fn snapshot<'a>(
        &'a self,
        platform: &'a crate::PlatformDetails,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<crate::RuntimeInventorySnapshot, RuntimeCapabilityError>>;
}

pub async fn initialize_inventory(
    reads: &crate::PlatformReadService,
    store: &dyn crate::InventoryProjectionStore,
    source: &(impl SwarmSnapshotPort + ?Sized),
    platform: &crate::PlatformDetails,
) -> Result<bool, RuntimeCapabilityError> {
    let _guard = reads.inventory_guard(platform.id).await;
    let initialized = reads
        .has_swarm_nodes(platform.id)
        .await
        .map_err(|e| RuntimeCapabilityError::new(RuntimeErrorKind::Remote, e.to_string(), true))?;
    if initialized {
        return Ok(false);
    }
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let snapshot = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        source.snapshot(platform, &cancel),
    )
    .await
    .map_err(|_| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Swarm inventory initialization timed out.",
            true,
        )
    })??;
    store.initialize_swarm(&snapshot).await
}

pub async fn refresh_inventory(
    store: &dyn crate::InventoryProjectionStore,
    source: &(impl SwarmSnapshotPort + ?Sized),
    platform: &crate::PlatformDetails,
    changed: impl Fn(&str),
) -> Result<(), RuntimeCapabilityError> {
    let cancel = CancellationToken::new();
    let _guard = cancel.clone().drop_guard();
    let snapshot = tokio::time::timeout(
        std::time::Duration::from_secs(30),
        source.snapshot(platform, &cancel),
    )
    .await
    .map_err(|_| {
        RuntimeCapabilityError::new(
            RuntimeErrorKind::Timeout,
            "Swarm inventory refresh timed out.",
            false,
        )
    })??;
    store.refresh_swarm(&snapshot).await?;
    for kind in ["node", "service", "task", "secret", "config", "platform"] {
        changed(kind);
    }
    Ok(())
}

pub async fn availability_updates(
    reads: &crate::PlatformReadService,
    pid: uuid::Uuid,
    input: UpdateSwarmNodesAvailabilityInput,
) -> Result<Vec<(String, UpdateSwarmNodeInput)>, crate::AuthorizedReadError> {
    let mut updates = Vec::with_capacity(input.nodes.len());
    for target in input.nodes {
        let node = reads
            .get_swarm_node(pid, &target.node_id)
            .await?
            .ok_or(crate::AuthorizedReadError::NotFound)?;
        check_node(&node, target.version_index)?;
        if !node.availability.eq_ignore_ascii_case(&input.availability) {
            updates.push((
                node.id,
                UpdateSwarmNodeInput {
                    version_index: target.version_index,
                    availability: input.availability.clone(),
                    labels: node.labels,
                },
            ));
        }
    }
    Ok(updates)
}
pub async fn update_nodes(
    runtime: &(impl SwarmControlPort + ?Sized),
    updates: &[(String, UpdateSwarmNodeInput)],
    cancel: &CancellationToken,
) -> Result<(), RuntimeCapabilityError> {
    for (index, (id, input)) in updates.iter().enumerate() {
        if let Err(error) = runtime.update_node(id, input, cancel).await {
            return Err(partial(error, index, updates.len(), "Updated"));
        }
    }
    Ok(())
}
pub async fn delete_resources(
    runtime: &(impl SwarmControlPort + ?Sized),
    ids: &[String],
    kind: SwarmResourceKind,
    cancel: &CancellationToken,
    removed: &mut Vec<String>,
) -> Result<(), RuntimeCapabilityError> {
    for (index, id) in ids.iter().enumerate() {
        let result = if kind == SwarmResourceKind::Service {
            runtime.delete_service(id, cancel).await
        } else {
            runtime
                .delete_material(kind == SwarmResourceKind::Secret, id, cancel)
                .await
        };
        match result {
            Ok(()) => {}
            Err(error) if error.kind == RuntimeErrorKind::NotFound => {}
            Err(error) => return Err(partial(error, index, ids.len(), "Deleted")),
        }
        removed.push(id.clone());
    }
    Ok(())
}

/// Refresh even on ambiguous writes, while preserving the original write failure.
pub async fn finish_mutation(
    store: &dyn crate::InventoryProjectionStore,
    source: &(impl SwarmSnapshotPort + ?Sized),
    platform: &crate::PlatformDetails,
    result: Result<(), RuntimeCapabilityError>,
    changed: impl Fn(&str),
) -> Result<(), RuntimeCapabilityError> {
    let refreshed = refresh_inventory(store, source, platform, changed).await;
    result.and(refreshed)
}
#[derive(Debug, thiserror::Error)]
pub enum SwarmCompletionError {
    #[error(transparent)]
    Runtime(RuntimeCapabilityError),
    #[error(transparent)]
    Projection(RuntimeCapabilityError),
}
/// Confirmed siblings disappear even if a later deletion or the refresh fails.
pub async fn finish_deletion(
    store: &dyn crate::InventoryProjectionStore,
    source: &(impl SwarmSnapshotPort + ?Sized),
    platform: &crate::PlatformDetails,
    result: Result<(), RuntimeCapabilityError>,
    kind: SwarmResourceKind,
    removed: &[String],
    changed: impl Fn(&str, &str, &str),
) -> Result<(), SwarmCompletionError> {
    let result = finish_mutation(store, source, platform, result, |kind| {
        changed(kind, "update", "")
    })
    .await;
    store
        .remove_swarm_resources(platform.id, kind, removed)
        .await
        .map_err(SwarmCompletionError::Projection)?;
    for id in removed {
        changed(kind.as_str(), "remove", id);
    }
    result.map_err(SwarmCompletionError::Runtime)
}
