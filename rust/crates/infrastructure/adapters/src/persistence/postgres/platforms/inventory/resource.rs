//! Event refresh persistence cannot accept a complete inventory snapshot.
use super::store::{self, PostgresInventoryProjectionStore, storage};
use citadel_platforms::jobs::{ProjectionChange, ProjectionWrite, SnapshotGeneration};
use citadel_platforms::{
    RuntimeCapabilityError, RuntimeErrorKind,
    jobs::{ResourceInventory, ResourceSnapshot},
};
use sqlx::{Postgres, Transaction};

impl PostgresInventoryProjectionStore {
    pub async fn persist_resource(
        &self,
        snapshot: &ResourceSnapshot,
    ) -> Result<(), RuntimeCapabilityError> {
        self.persist_resource_checked(snapshot, None)
            .await
            .map(|_| ())
    }

    pub async fn persist_resource_checked(
        &self,
        snapshot: &ResourceSnapshot,
        generation: Option<&SnapshotGeneration>,
    ) -> Result<bool, RuntimeCapabilityError> {
        Ok(self
            .persist_resource_committed(snapshot, generation)
            .await?
            .accepted())
    }
    pub async fn persist_resource_committed(
        &self,
        snapshot: &ResourceSnapshot,
        generation: Option<&SnapshotGeneration>,
    ) -> Result<ProjectionChange, RuntimeCapabilityError> {
        let write = match snapshot.inventory.projection_kind() {
            Some(kind) => Some(ProjectionWrite::begin(snapshot.platform_id, None, kind).await),
            None => None,
        };
        if generation.is_some_and(|stamp| write.as_ref().is_none_or(|write| !stamp.matches(write)))
        {
            return Ok(ProjectionChange::Unavailable);
        }
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let mut changed = persist_resource(&mut tx, snapshot, None, &self.node_policy).await?;
        let identities = if matches!(snapshot.inventory, ResourceInventory::Containers(_)) {
            crate::persistence::postgres::platforms::runtime_index::stage_scope(
                &self.pool,
                &mut tx,
                snapshot.platform_id,
                None,
            )
            .await
            .map_err(storage)?
        } else {
            None
        };
        tx.commit().await.map_err(storage)?;
        if let Some(identities) = identities {
            identities.committed();
        }
        if let Some(write) = write {
            write.committed();
        }
        if matches!(snapshot.inventory, ResourceInventory::Containers(_)) {
            changed |= crate::persistence::postgres::platforms::status::reconcile_deployments(
                &self.pool,
                snapshot.platform_id,
                None,
                false,
            )
            .await
            .map_err(storage)?
                > 0;
        }
        Ok(ProjectionChange::committed(changed))
    }
}

pub(crate) async fn persist_resource(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &ResourceSnapshot,
    node: Option<&str>,
    policy: &crate::persistence::postgres::platforms::node_agents::reconciliation::NodeAgentReconciliationPolicy,
) -> Result<bool, RuntimeCapabilityError> {
    use crate::persistence::postgres::platforms::nodes::store as nodes;
    let platform = snapshot.platform_id;
    // Match the lock order of the existing container and full snapshot writers.
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
    let mut changed = true;
    match &snapshot.inventory {
        ResourceInventory::Platform(info) => {
            if let Some(node) = node {
                nodes::persist_metadata(tx, platform, node, snapshot.observed_at, info, false)
                    .await?;
            } else {
                store::validate_platform_identity(
                    tx,
                    platform,
                    info,
                    info.swarm.as_ref().is_some_and(|s| !s.node_id.is_empty()),
                )
                .await?;
                changed = persist_metadata(tx, platform, info).await?;
            }
        }
        ResourceInventory::Containers(values) => {
            changed = store::persist_container_set(
                tx,
                platform,
                values,
                node,
                snapshot.observed_at.timestamp(),
                true,
            )
            .await?;
        }
        ResourceInventory::Images(values) => {
            if let Some(node) = node {
                changed =
                    nodes::persist_images(tx, platform, node, snapshot.observed_at, values).await?;
            } else {
                changed = store::persist_image_set(tx, platform, values).await?;
                // Images can arrive after their containers. Repair the relation
                // from existing rows without reading container inventory again.
                changed |= sqlx::query("UPDATE containers c SET imageid=i.id FROM images i WHERE c.platformid=$1 AND i.platformid=c.platformid AND i.dockerimageid=c.dockerimageid AND c.imageid IS DISTINCT FROM i.id")
                    .bind(platform).execute(&mut **tx).await.map_err(storage)?.rows_affected() > 0;
                changed |= sqlx::query("UPDATE platforms SET imagecount=$2 WHERE id=$1 AND imagecount IS DISTINCT FROM $2")
                    .bind(platform)
                    .bind(values.len().min(i32::MAX as usize) as i32)
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)?.rows_affected() > 0;
            }
        }
        ResourceInventory::Networks(values) => {
            if let Some(node) = node {
                changed = nodes::persist_networks(tx, platform, node, snapshot.observed_at, values)
                    .await?;
            } else {
                // Standalone resources are read live; equal counts do not prove equal resources.
                sqlx::query("UPDATE platforms SET networkcount=$2 WHERE id=$1 AND networkcount IS DISTINCT FROM $2")
                    .bind(platform)
                    .bind(values.len().min(i32::MAX as usize) as i32)
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)? ;
            }
        }
        ResourceInventory::Volumes(values) => {
            if let Some(node) = node {
                changed = nodes::persist_volumes(tx, platform, node, snapshot.observed_at, values)
                    .await?;
            } else {
                // Standalone resources are read live; equal counts do not prove equal resources.
                sqlx::query("UPDATE platforms SET volumecount=$2 WHERE id=$1 AND volumecount IS DISTINCT FROM $2")
                    .bind(platform)
                    .bind(values.len().min(i32::MAX as usize) as i32)
                    .execute(&mut **tx)
                    .await
                    .map_err(storage)? ;
            }
        }
        ResourceInventory::Swarm {
            inventory,
            networks,
        } => {
            if node.is_some() {
                return Err(conflict("A worker cannot publish manager inventory"));
            }
            let (cluster, descriptor): (Option<String>, serde_json::Value) =
                sqlx::query_as("SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1")
                    .bind(platform)
                    .fetch_one(&mut **tx)
                    .await
                    .map_err(storage)?;
            let manager = descriptor
                .get("nodeID")
                .or_else(|| descriptor.get("NodeID"))
                .and_then(serde_json::Value::as_str)
                .filter(|v| !v.is_empty());
            let Some(manager) = manager.filter(|id| {
                inventory
                    .nodes
                    .iter()
                    .any(|n| n.id == *id && n.role.eq_ignore_ascii_case("manager"))
            }) else {
                return Err(conflict(
                    "Swarm refresh does not contain the pinned manager identity",
                ));
            };
            if cluster.as_deref().is_none_or(str::is_empty) {
                return Err(conflict("Swarm identity has not been initialized"));
            }
            let newer: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1 AND observedat>$2)").bind(platform).bind(snapshot.observed_at).fetch_one(&mut **tx).await.map_err(storage)?;
            if newer {
                return Ok(false);
            }
            // Compatibility view for existing Swarm-only SQL helpers. Never pass
            // this partial view to persist_snapshot or persist_platform.
            let view = citadel_platforms::RuntimeInventorySnapshot {
                platform_id: platform,
                observed_at: snapshot.observed_at,
                info: citadel_platforms::RuntimePlatformInfo {
                    swarm: Some(citadel_platforms::RuntimeSwarmInfo {
                        node_id: manager.to_owned(),
                        cluster_id: cluster,
                        control_available: true,
                        local_node_state: "active".into(),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                containers: vec![],
                images: vec![],
                volumes: vec![],
                networks: networks.clone(),
                swarm: None,
            };
            store::persist_swarm(tx, &view, inventory).await?;
            let counts = serde_json::json!({"nodes": inventory.nodes.len(),
                "managers": inventory.nodes.iter().filter(|n| n.role.eq_ignore_ascii_case("manager")).count(),
                "serviceCount": inventory.services.len(), "runningTaskCount": inventory.running_task_count});
            sqlx::query("UPDATE platforms SET platformdescriptor=(platformdescriptor::jsonb || $2::jsonb)::json WHERE id=$1")
                .bind(platform).bind(counts).execute(&mut **tx).await.map_err(storage)?;
            crate::persistence::postgres::platforms::node_agents::reconciliation::reconcile(
                tx, &view, inventory, policy,
            )
            .await
            .map_err(storage)?;
        }
    }
    Ok(changed)
}

fn conflict(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}

/// Platform metadata never overwrites resource counts or resource projections.
async fn persist_metadata(
    tx: &mut Transaction<'_, Postgres>,
    platform: uuid::Uuid,
    info: &citadel_platforms::RuntimePlatformInfo,
) -> Result<bool, RuntimeCapabilityError> {
    let descriptor = serde_json::json!({
        "daemonId": info.daemon_id, "operatingSystem": info.operating_system,
        "osType": info.os_type, "architecture": info.architecture,
        "apiVersion": info.api_version, "minimumApiVersion": info.minimum_api_version,
        "nodeID": info.swarm.as_ref().filter(|s| s.control_available && s.local_node_state.eq_ignore_ascii_case("active")).map(|s| &s.node_id),
    });
    let result = sqlx::query("UPDATE platforms SET cpucount=$2,memtotal=$3,serverversion=$4,agentversion=$5,platformdescriptor=(platformdescriptor::jsonb || $6::jsonb)::json WHERE id=$1 AND (cpucount,memtotal,serverversion,agentversion,platformdescriptor::jsonb) IS DISTINCT FROM ($2,$3,$4,$5,platformdescriptor::jsonb || $6::jsonb)")
        .bind(platform).bind(info.cpu_count.clamp(0, i32::MAX as i64) as i32)
        .bind(info.memory_total).bind(&info.server_version).bind(&info.agent_version)
        .bind(descriptor).execute(&mut **tx).await.map_err(storage)?;
    Ok(result.rows_affected() > 0)
}

/// Compatibility boundary for manager-only callers. Never replaces standalone sets.
pub(crate) async fn persist_swarm_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &citadel_platforms::RuntimeInventorySnapshot,
    policy: &crate::persistence::postgres::platforms::node_agents::reconciliation::NodeAgentReconciliationPolicy,
) -> Result<(), RuntimeCapabilityError> {
    let inventory = snapshot
        .swarm
        .clone()
        .ok_or_else(|| conflict("Swarm inventory required"))?;
    store::validate_platform_identity(tx, snapshot.platform_id, &snapshot.info, true).await?;
    // Legacy initialization may not have pinned the manager yet. Pin only
    // identity; CPU/memory/version metadata belongs to Platform reconciliation.
    let identity = serde_json::json!({"daemonId": snapshot.info.daemon_id,
        "nodeID": snapshot.info.swarm.as_ref().map(|s| &s.node_id)});
    sqlx::query("UPDATE platforms SET platformdescriptor=(platformdescriptor::jsonb || $2::jsonb)::json WHERE id=$1")
        .bind(snapshot.platform_id).bind(identity).execute(&mut **tx).await.map_err(storage)?;
    persist_resource(
        tx,
        &ResourceSnapshot {
            platform_id: snapshot.platform_id,
            observed_at: snapshot.observed_at,
            inventory: ResourceInventory::Swarm {
                inventory,
                networks: snapshot.networks.clone(),
            },
        },
        None,
        policy,
    )
    .await
    .map(|_| ())
}
