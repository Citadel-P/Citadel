use citadel_platforms::jobs::{ProjectionKind, ProjectionWrite};
use citadel_platforms::{
    InventoryProjectionChange, InventoryProjectionStore, RuntimeCapabilityError, RuntimeErrorKind,
    RuntimeInventorySnapshot, RuntimeSwarmInventory,
};
use futures_util::{FutureExt, future::BoxFuture};
use serde::Serialize;
use serde_json::Value;
use sqlx::{PgPool, Postgres, Transaction};

const STALE_RETENTION_HOURS: i32 = 24;

#[derive(Clone)]
pub struct PostgresInventoryProjectionStore {
    health_owned: bool,
    pub(super) pool: PgPool,
    pub(super) node_policy: citadel_platforms::node_agents::NodeAgentReconciliationPolicy,
}

impl PostgresInventoryProjectionStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            health_owned: false,
            node_policy: Default::default(),
        }
    }

    pub fn with_health_owner(mut self) -> Self {
        self.health_owned = true;
        self
    }

    pub fn with_node_policy(
        mut self,
        policy: citadel_platforms::node_agents::NodeAgentReconciliationPolicy,
    ) -> Self {
        self.node_policy = policy;
        self
    }

    /// A failed read invalidates confidence, not the last successful rows.
    /// Timestamp fencing protects a newer refresh which committed in parallel.
    pub async fn mark_swarm_stale(
        &self,
        platform: uuid::Uuid,
        started_at: chrono::DateTime<chrono::Utc>,
    ) -> Result<(), RuntimeCapabilityError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
            .bind(platform)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?;
        for table in [
            "swarmnodeprojections",
            "swarmserviceprojections",
            "swarmtaskprojections",
            "swarmconfigprojections",
            "swarmsecretprojections",
            "swarmnetworkprojections",
        ] {
            let statement =
                format!("UPDATE {table} SET isstale=TRUE WHERE platformid=$1 AND observedat<=$2");
            sqlx::query(sqlx::AssertSqlSafe(statement))
                .bind(platform)
                .bind(started_at)
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
        }
        tx.commit().await.map_err(storage)?;
        Ok(())
    }

    /// Initialize a missing Swarm inventory without overwriting a reconciliation
    /// which completed while Docker was being queried. No network I/O holds this lock.
    pub async fn initialize_swarm(
        &self,
        snapshot: &RuntimeInventorySnapshot,
    ) -> Result<bool, RuntimeCapabilityError> {
        let metadata =
            ProjectionWrite::begin(snapshot.platform_id, None, ProjectionKind::Platform).await;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let saved = sqlx::query_as::<_, (Option<String>, Value)>(
            "SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1 FOR NO KEY UPDATE",
        )
        .bind(snapshot.platform_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        .ok_or_else(|| {
            RuntimeCapabilityError::new(RuntimeErrorKind::NotFound, "Platform not found", false)
        })?;
        let info = snapshot.info.swarm.as_ref().filter(|info| {
            info.control_available && info.local_node_state.eq_ignore_ascii_case("active")
        });
        let manager = saved
            .1
            .get("nodeID")
            .or_else(|| saved.1.get("NodeID"))
            .and_then(Value::as_str);
        if !info.is_some_and(|info| {
            info.cluster_id.as_deref() == saved.0.as_deref()
                && saved.0.as_deref().is_some_and(|id| !id.is_empty())
                && manager.is_none_or(|id| id == info.node_id)
        }) || snapshot.swarm.is_none()
        {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "The connected Docker manager no longer belongs to this Swarm platform.",
                false,
            ));
        }
        let initialized: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1)",
        )
        .bind(snapshot.platform_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?;
        if initialized {
            return Ok(false);
        }
        super::resource::persist_swarm_snapshot(&mut tx, snapshot, &self.node_policy).await?;
        tx.commit().await.map_err(storage)?;
        metadata.committed();
        Ok(true)
    }
}

impl InventoryProjectionStore for PostgresInventoryProjectionStore {
    fn initialize_swarm<'a>(
        &'a self,
        snapshot: &'a RuntimeInventorySnapshot,
    ) -> BoxFuture<'a, Result<bool, RuntimeCapabilityError>> {
        Box::pin(self.initialize_swarm(snapshot))
    }
    fn refresh_swarm<'a>(
        &'a self,
        snapshot: &'a RuntimeInventorySnapshot,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(
            crate::persistence::postgres::platforms::swarm::inventory::refresh(
                &self.pool,
                snapshot,
                &self.node_policy,
            ),
        )
    }

    fn remove_swarm_resources<'a>(
        &'a self,
        platform: uuid::Uuid,
        kind: citadel_platforms::swarm_mutations::SwarmResourceKind,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            if ids.is_empty() {
                return Ok(());
            }
            use citadel_platforms::swarm_mutations::SwarmResourceKind;
            let sql = match kind {
                SwarmResourceKind::Service => {
                    "DELETE FROM swarmserviceprojections WHERE platformid=$1 AND dockerserviceid=ANY($2)"
                }
                SwarmResourceKind::Secret => {
                    "DELETE FROM swarmsecretprojections WHERE platformid=$1 AND dockersecretid=ANY($2)"
                }
                SwarmResourceKind::Config => {
                    "DELETE FROM swarmconfigprojections WHERE platformid=$1 AND dockerconfigid=ANY($2)"
                }
            };
            sqlx::query(sql)
                .bind(platform)
                .bind(ids)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
            Ok(())
        })
    }

    fn persist<'a>(
        &'a self,
        snapshot: &'a RuntimeInventorySnapshot,
    ) -> BoxFuture<'a, Result<InventoryProjectionChange, RuntimeCapabilityError>> {
        async move {
            let containers =
                ProjectionWrite::begin(snapshot.platform_id, None, ProjectionKind::Containers)
                    .await;
            let images =
                ProjectionWrite::begin(snapshot.platform_id, None, ProjectionKind::Images).await;
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            validate_snapshot_identity(&mut transaction, snapshot).await?;
            persist_snapshot_with_health(
                &mut transaction,
                snapshot,
                Some(&self.node_policy),
                !self.health_owned,
            )
            .await?;
            let identities = crate::persistence::postgres::platforms::runtime_index::stage_scope(
                &self.pool,
                &mut transaction,
                snapshot.platform_id,
                None,
            )
            .await
            .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            if let Some(identities) = identities {
                identities.committed();
            }
            containers.committed();
            images.committed();
            crate::persistence::postgres::platforms::status::reconcile_deployments(
                &self.pool,
                snapshot.platform_id,
                None,
                false,
            )
            .await
            .map_err(storage)?;
            Ok(InventoryProjectionChange {
                platform_id: snapshot.platform_id,
                revision: snapshot.observed_at.timestamp_millis(),
            })
        }
        .boxed()
    }
}

pub(crate) async fn validate_snapshot_identity(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
) -> Result<(), RuntimeCapabilityError> {
    validate_platform_identity(
        tx,
        snapshot.platform_id,
        &snapshot.info,
        snapshot.swarm.is_some(),
    )
    .await
}

pub(crate) async fn validate_platform_identity(
    tx: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    info: &citadel_platforms::RuntimePlatformInfo,
    has_swarm: bool,
) -> Result<(), RuntimeCapabilityError> {
    let saved: Option<(Option<String>, Value)> = sqlx::query_as(
        "SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1 FOR NO KEY UPDATE",
    )
    .bind(platform_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(storage)?;
    let Some((cluster, descriptor)) = saved else {
        return Err(identity_conflict("Platform no longer exists"));
    };
    validate_identity(&descriptor, cluster.as_deref(), info, has_swarm)?;
    // Serialize first-time identity pinning across different Platform rows too.
    let identity = info
        .swarm
        .as_ref()
        .and_then(|s| s.cluster_id.as_deref())
        .filter(|_| has_swarm)
        .unwrap_or(&info.daemon_id);
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 0))")
        .bind(identity)
        .execute(&mut **tx)
        .await
        .map_err(storage)?;
    let duplicate: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM platforms WHERE id<>$1 AND ((($2::boolean) AND clusterid=$3) OR (NOT $2 AND COALESCE(platformdescriptor->>'daemonId',platformdescriptor->>'DaemonId')=$3)))")
        .bind(platform_id).bind(has_swarm).bind(identity)
        .fetch_one(&mut **tx).await.map_err(storage)?;
    if duplicate {
        return Err(identity_conflict(
            "Runtime identity already belongs to another Platform",
        ));
    }
    if has_swarm {
        sqlx::query("UPDATE platforms SET clusterid=$2 WHERE id=$1 AND clusterid IS NULL")
            .bind(platform_id)
            .bind(identity)
            .execute(&mut **tx)
            .await
            .map_err(storage)?;
    }
    Ok(())
}

fn identity_conflict(message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Conflict, message, false)
}

fn validate_identity(
    descriptor: &Value,
    cluster: Option<&str>,
    info: &citadel_platforms::RuntimePlatformInfo,
    has_swarm_snapshot: bool,
) -> Result<(), RuntimeCapabilityError> {
    let field = |camel: &str, pascal: &str| {
        descriptor
            .get(camel)
            .or_else(|| descriptor.get(pascal))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty())
    };
    if info.daemon_id.trim().is_empty()
        || field("daemonId", "DaemonId").is_some_and(|id| id != info.daemon_id.trim())
    {
        return Err(identity_conflict(
            "The connected Docker daemon identity changed or is missing",
        ));
    }
    let swarm = descriptor
        .get("$type")
        .and_then(Value::as_str)
        .is_some_and(|t| t.eq_ignore_ascii_case("DockerSwarm"));
    let reported_swarm = info
        .swarm
        .as_ref()
        .is_some_and(|s| !s.node_id.trim().is_empty());
    if swarm != has_swarm_snapshot || swarm != reported_swarm {
        return Err(identity_conflict("The Platform runtime type changed"));
    }
    if swarm {
        let reported = info
            .swarm
            .as_ref()
            .filter(|s| s.control_available && s.local_node_state.eq_ignore_ascii_case("active"))
            .ok_or_else(|| identity_conflict("The endpoint is not an active Swarm manager"))?;
        let reported_cluster = reported
            .cluster_id
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| identity_conflict("The Swarm cluster identity is missing"))?;
        if cluster
            .filter(|s| !s.is_empty())
            .is_some_and(|id| id != reported_cluster)
            || field("nodeID", "NodeID").is_some_and(|id| id != reported.node_id)
        {
            return Err(identity_conflict(
                "The pinned Swarm cluster or manager identity changed",
            ));
        }
    }
    Ok(())
}

pub(crate) async fn persist_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    node_policy: Option<&citadel_platforms::node_agents::NodeAgentReconciliationPolicy>,
) -> Result<(), RuntimeCapabilityError> {
    persist_snapshot_with_health(transaction, snapshot, node_policy, true).await
}

async fn persist_snapshot_with_health(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    node_policy: Option<&citadel_platforms::node_agents::NodeAgentReconciliationPolicy>,
    update_health: bool,
) -> Result<(), RuntimeCapabilityError> {
    if snapshot.swarm.is_some() {
        // The event worker and a post-mutation refresh can finish in reverse
        // order. Serialize their short commits and never restore an older
        // manager observation over a newer one (including resource versions).
        sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
            .bind(snapshot.platform_id)
            .fetch_optional(&mut **transaction)
            .await
            .map_err(storage)?;
        let newer: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM swarmnodeprojections WHERE platformid=$1 AND observedat>$2)")
            .bind(snapshot.platform_id).bind(snapshot.observed_at).fetch_one(&mut **transaction).await.map_err(storage)?;
        if newer {
            return Ok(());
        }
    }
    persist_platform(transaction, snapshot).await?;
    if update_health {
        crate::persistence::postgres::platforms::status::platform_status(
            transaction,
            snapshot.platform_id,
            citadel_primitives::PlatformStatus::Online,
        )
        .await
        .map_err(storage)?;
    }
    persist_images(transaction, snapshot).await?;
    persist_containers(transaction, snapshot, None).await?;
    if let Some(swarm) = &snapshot.swarm {
        persist_swarm(transaction, snapshot, swarm).await?;
        if let Some(policy) = node_policy {
            crate::persistence::postgres::platforms::node_agents::reconciliation::reconcile(
                transaction,
                snapshot,
                swarm,
                policy,
            )
            .await
            .map_err(storage)?;
        }
    }
    Ok(())
}

async fn persist_platform(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
) -> Result<(), RuntimeCapabilityError> {
    let mut descriptor = serde_json::json!({
        "daemonId": snapshot.info.daemon_id,
        "containerCount": snapshot.info.container_count,
        "containersRunning": snapshot.info.containers_running,
        "containersPaused": snapshot.info.containers_paused,
        "containersStopped": snapshot.info.containers_stopped,
        "operatingSystem": snapshot.info.operating_system,
        "osType": snapshot.info.os_type,
        "architecture": snapshot.info.architecture,
        "apiVersion": snapshot.info.api_version,
        "minimumApiVersion": snapshot.info.minimum_api_version,
        // Refresh (or clear) the connected daemon identity on every inventory
        // commit. Legacy manager Containers have no explicit node column.
        "nodeID": snapshot.info.swarm.as_ref()
            .filter(|swarm| swarm.control_available && swarm.local_node_state.eq_ignore_ascii_case("active"))
            .map(|swarm| swarm.node_id.as_str())
            .filter(|id| !id.is_empty()),
    });
    if let Some(swarm) = &snapshot.swarm {
        let object = descriptor
            .as_object_mut()
            .expect("the descriptor patch is an object");
        object.insert("nodes".into(), serde_json::json!(swarm.nodes.len()));
        object.insert(
            "managers".into(),
            serde_json::json!(
                swarm
                    .nodes
                    .iter()
                    .filter(|node| node.role.eq_ignore_ascii_case("manager"))
                    .count()
            ),
        );
        object.insert(
            "serviceCount".into(),
            serde_json::json!(swarm.services.len()),
        );
        object.insert(
            "runningTaskCount".into(),
            serde_json::json!(swarm.running_task_count),
        );
    }
    sqlx::query(
        r#"
UPDATE platforms
SET cpucount = $2,
    memtotal = $3,
    imagecount = $4,
    networkcount = $5,
    volumecount = $6,
    serverversion = $7,
    agentversion = $8,
    platformdescriptor = (platformdescriptor::jsonb || $9::jsonb)::json
WHERE id = $1
"#,
    )
    .bind(snapshot.platform_id)
    .bind(bounded_i32(snapshot.info.cpu_count))
    .bind(snapshot.info.memory_total)
    .bind(bounded_i32(snapshot.images.len() as i64))
    .bind(bounded_i32(snapshot.networks.len() as i64))
    .bind(bounded_i32(snapshot.volumes.len() as i64))
    .bind(&snapshot.info.server_version)
    .bind(&snapshot.info.agent_version)
    .bind(descriptor)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn persist_images(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
) -> Result<(), RuntimeCapabilityError> {
    persist_image_set(transaction, snapshot.platform_id, &snapshot.images)
        .await
        .map(|_| ())
}

/// Reject ambiguous batches even when ON CONFLICT skips identical rows.
pub(crate) fn validate_runtime_ids<'a>(
    ids: impl IntoIterator<Item = &'a str>,
) -> Result<(), RuntimeCapabilityError> {
    let mut unique = std::collections::BTreeSet::new();
    for id in ids {
        if !unique.insert(id) {
            return Err(RuntimeCapabilityError::new(
                RuntimeErrorKind::Conflict,
                "Duplicate runtime identity in resource observation",
                false,
            ));
        }
    }
    Ok(())
}

pub(super) async fn persist_image_set(
    transaction: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    images: &[citadel_platforms::RuntimeImageSummary],
) -> Result<bool, RuntimeCapabilityError> {
    persist_image_observations(transaction, platform_id, images, true).await
}

pub(super) async fn persist_image_observations(
    transaction: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    images: &[citadel_platforms::RuntimeImageSummary],
    complete: bool,
) -> Result<bool, RuntimeCapabilityError> {
    validate_runtime_ids(images.iter().map(|value| value.id.as_str()))?;
    let payload = json(images)?;
    let changed: bool = sqlx::query_scalar(
        r#"
WITH incoming AS (
    SELECT * FROM jsonb_to_recordset($2::jsonb) AS value(
        id text, repo_tags jsonb, repo_digests jsonb, created bigint,
        size bigint, containers bigint)
), upserted AS (
    INSERT INTO images (
        id, containers, createdat, dockerimageid, name, platformid, rowversion,
        size, tags, controlstate)
    SELECT gen_random_uuid(), bounded.containers,
           to_timestamp(bounded.created), bounded.id,
           COALESCE(NULLIF(regexp_replace(bounded.repo_tags->>0, ':[^/:]+$', ''), ''), bounded.id),
           $1, 0, bounded.size::double precision, bounded.repo_tags::json, 'Idle'
    FROM incoming bounded
    ON CONFLICT (dockerimageid, platformid) DO UPDATE
    SET containers = EXCLUDED.containers,
        name = EXCLUDED.name,
        size = EXCLUDED.size,
        tags = EXCLUDED.tags,
        updatedat = now(),
        rowversion = images.rowversion + 1
    WHERE (images.containers,images.name,images.size,images.tags::jsonb)
        IS DISTINCT FROM (EXCLUDED.containers,EXCLUDED.name,EXCLUDED.size,EXCLUDED.tags::jsonb)
    RETURNING dockerimageid
), deleted AS (
DELETE FROM images image
WHERE $3::boolean AND image.platformid = $1
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE incoming.id = image.dockerimageid)
RETURNING dockerimageid
)
SELECT EXISTS(SELECT 1 FROM upserted) OR EXISTS(SELECT 1 FROM deleted)
"#,
    )
    .bind(platform_id)
    .bind(payload)
    .bind(complete)
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(changed)
}

pub(crate) async fn persist_containers(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    node_id: Option<&str>,
) -> Result<(), RuntimeCapabilityError> {
    persist_container_set(
        transaction,
        snapshot.platform_id,
        &snapshot.containers,
        node_id,
        snapshot.observed_at.timestamp(),
        true,
    )
    .await
    .map(|_| ())
}

/// A fully inspected container is authoritative for itself, never for absent siblings.
pub(crate) async fn persist_container_observation(
    transaction: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    node_id: Option<&str>,
    container: &citadel_platforms::RuntimeContainerSummary,
    observed: i64,
) -> Result<bool, RuntimeCapabilityError> {
    persist_container_set(
        transaction,
        platform_id,
        std::slice::from_ref(container),
        node_id,
        observed,
        false,
    )
    .await
}

pub(super) async fn persist_container_set(
    transaction: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
    containers: &[citadel_platforms::RuntimeContainerSummary],
    node_id: Option<&str>,
    observed: i64,
    complete: bool,
) -> Result<bool, RuntimeCapabilityError> {
    validate_runtime_ids(containers.iter().map(|value| value.id.as_str()))?;
    sqlx::query("SELECT id FROM platforms WHERE id=$1 FOR NO KEY UPDATE")
        .bind(platform_id)
        .fetch_optional(&mut **transaction)
        .await
        .map_err(storage)?;
    let incoming_ids: Vec<_> = containers.iter().map(|c| c.id.clone()).collect();
    let removed = if complete {
        crate::persistence::postgres::platforms::status::removed_bindings(
            transaction,
            platform_id,
            node_id,
            &incoming_ids,
            observed,
        )
        .await
        .map_err(storage)?
    } else {
        Default::default()
    };
    // Transactional notifications are delivered only after the new inventory commits.
    // Every transport uses this projection path, including node-scoped Edge inventory.
    sqlx::query("SELECT pg_notify('citadel_container_created', json_build_object('platform', $1::uuid, 'container', incoming, 'node', $3::text)::text) FROM unnest($2::text[]) incoming WHERE NOT EXISTS(SELECT 1 FROM containers WHERE platformid=$1 AND dockercontainerid=incoming AND dockernodeid IS NOT DISTINCT FROM $3)")
        .bind(platform_id).bind(&incoming_ids).bind(node_id)
        .execute(&mut **transaction).await.map_err(storage)?;
    let payload = json(&containers)?;
    let conflict = if node_id.is_some() {
        "(dockercontainerid, platformid, dockernodeid) WHERE dockernodeid IS NOT NULL"
    } else {
        "(dockercontainerid, platformid) WHERE dockernodeid IS NULL"
    };
    let mut changed: bool = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        r#"
WITH incoming AS (
    SELECT * FROM jsonb_to_recordset($2::jsonb) AS value(
        id text, name text, image text, "imageId" text, created bigint, state text,
        status text, labels jsonb, ports jsonb, stack text,
        "systemRole" text, "hasCitadelOwnershipLabels" boolean, "isSwarmTask" boolean,
        "isSystem" boolean)
), upserted AS (
    INSERT INTO containers (
        id, platformid, dockercontainerid, name, dockerimageid, created, state,
        controlstate, updated, stack, issystem, systemrole,
        hascitadelownershiplabels, isswarmtask, ports, rowversion,
        projectionobservedat, dockernodeid, stackid, deploymentid, imageid)
    SELECT gen_random_uuid(), $1, incoming.id, incoming.name,
           CASE WHEN NOT $5::boolean AND incoming."imageId" = ''
                THEN incoming.image ELSE incoming."imageId" END,
           incoming.created, initcap(incoming.state), 'Idle', $3, incoming.stack,
           incoming."isSystem", incoming."systemRole",
           incoming."hasCitadelOwnershipLabels", incoming."isSwarmTask",
           COALESCE(incoming.ports, '[]'::jsonb)::json, 0, $3, $4,
           release.stackid, deployment.id, image.id
    FROM incoming
    LEFT JOIN images image
      ON image.platformid=$1 AND image.dockerimageid=CASE WHEN NOT $5::boolean AND incoming."imageId"='' THEN incoming.image ELSE incoming."imageId" END
    LEFT JOIN stacks stack
      ON stack.id = CASE WHEN incoming.labels->>'com.citadel.stack-id'
          ~* '^[0-9a-f]{{8}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{12}}$'
          THEN (incoming.labels->>'com.citadel.stack-id')::uuid END
     AND incoming.labels->>'com.citadel.managed' = 'true'
     AND NOT (incoming.labels ? 'com.citadel.deployment-id')
    LEFT JOIN stackreleases release
      ON release.id = stack.currentstackreleaseid AND release.platformid = $1
    LEFT JOIN deployments deployment
      ON deployment.id = CASE WHEN incoming.labels->>'com.citadel.deployment-id'
          ~* '^[0-9a-f]{{8}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{4}}-[0-9a-f]{{12}}$'
          THEN (incoming.labels->>'com.citadel.deployment-id')::uuid END
     AND deployment.platformid = $1
     AND incoming.labels->>'com.citadel.managed' = 'true'
     AND NOT (incoming.labels ? 'com.citadel.stack-id')
    ON CONFLICT {conflict} DO UPDATE
    SET name = EXCLUDED.name,
        dockerimageid = EXCLUDED.dockerimageid,
        imageid = COALESCE(EXCLUDED.imageid, containers.imageid),
        state = EXCLUDED.state,
        updated = EXCLUDED.updated,
        stack = EXCLUDED.stack,
        issystem = EXCLUDED.issystem,
        systemrole = EXCLUDED.systemrole,
        hascitadelownershiplabels = EXCLUDED.hascitadelownershiplabels,
        isswarmtask = EXCLUDED.isswarmtask,
        ports = EXCLUDED.ports,
        stackid = CASE WHEN containers.deploymentid IS NULL
            THEN COALESCE(containers.stackid, EXCLUDED.stackid) ELSE containers.stackid END,
        deploymentid = CASE WHEN containers.stackid IS NULL
            THEN COALESCE(containers.deploymentid, EXCLUDED.deploymentid) ELSE containers.deploymentid END,
        projectionobservedat = EXCLUDED.projectionobservedat,
        projectionstalesince = NULL,
        projectionstalereason = NULL,
        rowversion = containers.rowversion + 1
    WHERE COALESCE(containers.projectionobservedat, 0) <= EXCLUDED.projectionobservedat
      AND (containers.name, containers.dockerimageid, containers.state, containers.stack,
           containers.issystem, containers.systemrole, containers.hascitadelownershiplabels,
           containers.isswarmtask, containers.ports::jsonb, containers.stackid, containers.deploymentid,
           containers.projectionstalesince, containers.projectionstalereason, containers.imageid)
          IS DISTINCT FROM
          (EXCLUDED.name, EXCLUDED.dockerimageid, EXCLUDED.state, EXCLUDED.stack,
           EXCLUDED.issystem, EXCLUDED.systemrole, EXCLUDED.hascitadelownershiplabels,
           EXCLUDED.isswarmtask, EXCLUDED.ports::jsonb,
           CASE WHEN containers.deploymentid IS NULL THEN COALESCE(containers.stackid, EXCLUDED.stackid) ELSE containers.stackid END,
           CASE WHEN containers.stackid IS NULL THEN COALESCE(containers.deploymentid, EXCLUDED.deploymentid) ELSE containers.deploymentid END,
           NULL::bigint, NULL::text, COALESCE(EXCLUDED.imageid, containers.imageid))
    RETURNING dockercontainerid
), deleted AS (
DELETE FROM containers container
WHERE $5::boolean AND container.platformid = $1
  AND container.dockernodeid IS NOT DISTINCT FROM $4
  AND COALESCE(container.projectionobservedat, 0) <= $3
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE incoming.id = container.dockercontainerid)
RETURNING dockercontainerid
)
SELECT EXISTS(SELECT 1 FROM upserted) OR EXISTS(SELECT 1 FROM deleted)
"#
    )))
    .bind(platform_id)
    .bind(payload)
    .bind(observed)
    .bind(node_id)
    .bind(complete)
    .fetch_one(&mut **transaction)
    .await
    .map_err(storage)?;
    // Watermarks are causal bookkeeping, not semantic revisions. Advance even on a no-op.
    sqlx::query("UPDATE containers SET projectionobservedat=$4 WHERE platformid=$1 AND dockernodeid IS NOT DISTINCT FROM $2 AND dockercontainerid=ANY($3) AND COALESCE(projectionobservedat,0)<$4")
        .bind(platform_id).bind(node_id).bind(&incoming_ids).bind(observed)
        .execute(&mut **transaction).await.map_err(storage)?;
    // Recovery also catches up owners skipped while a command held their locks.
    if complete {
        changed |= crate::persistence::postgres::platforms::status::reconcile(
            transaction,
            platform_id,
            node_id,
            &removed,
            false,
        )
        .await
        .map_err(storage)?;
    }
    Ok(changed)
}

pub(super) async fn persist_swarm(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    swarm: &RuntimeSwarmInventory,
) -> Result<(), RuntimeCapabilityError> {
    let observed_at = snapshot.observed_at;
    for table in [
        "swarmnodeprojections",
        "swarmserviceprojections",
        "swarmtaskprojections",
        "swarmconfigprojections",
        "swarmsecretprojections",
        "swarmnetworkprojections",
    ] {
        let statement = format!("UPDATE {table} SET isstale = TRUE WHERE platformid = $1");
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(snapshot.platform_id)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?;
    }

    let nodes = json(&swarm.nodes)?;
    sqlx::query(
        r#"
INSERT INTO swarmnodeprojections (
    platformid, dockernodeid, address, architecture, availability,
    desiredtaskcount, dockercreatedat, dockerupdatedat, engineversion, hostname,
    isleader, isstale, labels, observedat, operatingsystem, reachability, role,
    runningtaskcount, status, statusmessage, versionindex)
SELECT $1, value.id, value.address, value.architecture, value.availability,
       0, value.created_at, value.updated_at, value.engine_version, value.hostname,
       value.is_leader, FALSE, value.labels, $3, value.operating_system,
       value.reachability, value.role, 0, value.status, value.status_message,
       value.version_index
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, version_index bigint, hostname text, role text, is_leader boolean,
    reachability text, status text, status_message text, availability text,
    engine_version text, operating_system text, architecture text, address text,
    labels jsonb, created_at timestamptz, updated_at timestamptz)
ON CONFLICT (platformid, dockernodeid) DO UPDATE SET
    address = EXCLUDED.address, architecture = EXCLUDED.architecture,
    availability = EXCLUDED.availability, dockercreatedat = EXCLUDED.dockercreatedat,
    dockerupdatedat = EXCLUDED.dockerupdatedat, engineversion = EXCLUDED.engineversion,
    hostname = EXCLUDED.hostname, isleader = EXCLUDED.isleader, isstale = FALSE,
    labels = EXCLUDED.labels, observedat = EXCLUDED.observedat,
    operatingsystem = EXCLUDED.operatingsystem, reachability = EXCLUDED.reachability,
    role = EXCLUDED.role, status = EXCLUDED.status,
    statusmessage = EXCLUDED.statusmessage, versionindex = EXCLUDED.versionindex
"#,
    )
    .bind(snapshot.platform_id)
    .bind(nodes)
    .bind(observed_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;

    // Resolve ownership against the current release, not untrusted daemon labels.
    // The upsert below still preserves an explicitly imported Docker identity.
    let owners: Vec<(uuid::Uuid, String, Value)> = sqlx::query_as(
        "SELECT s.id,s.name,r.spec FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE r.platformid=$1",
    )
    .bind(snapshot.platform_id)
    .fetch_all(&mut **transaction)
    .await
    .map_err(storage)?;
    let namespaces: std::collections::HashMap<_, _> = owners
        .into_iter()
        .map(|(id, name, spec)| {
            let namespace = spec
                .get("projectName")
                .or_else(|| spec.get("ProjectName"))
                .and_then(Value::as_str)
                .filter(|name| !name.trim().is_empty())
                .map(|name| name.trim().to_owned())
                .unwrap_or_else(|| citadel_stacks::normalize_project_name(&name, id));
            (id, namespace)
        })
        .collect();
    let mut services = swarm.services.clone();
    for service in &mut services {
        if service.ownership == citadel_swarm_services::SwarmServiceOwnership::CitadelStack
            && let Some(expected) = service.stack_id.and_then(|id| namespaces.get(&id))
            && service.stack_namespace.as_deref() != Some(expected.as_str())
        {
            service.stack_id = None;
            service.ownership = citadel_swarm_services::SwarmServiceOwnership::OwnershipConflict;
            service.ownership_diagnostic =
                Some("The claimed Citadel Stack does not own this Docker Stack namespace.".into());
        }
    }
    let services = json(&services)?;
    sqlx::query(
        r#"
INSERT INTO swarmserviceprojections (
    platformid, dockerserviceid, configids, desiredtaskcount, dockercreatedat,
    dockerstacknamespace, dockerupdatedat, forceupdate, image, isstale, labels,
    liveruntimehash, mode, name, networkids, observedat, ownership, ports,
    runningtaskcount, secretids, updatemessage, updatestate, versionindex,
    ownershipdiagnostic, swarmserviceid, stackid)
SELECT $1, value.id, value.config_ids, value.desired_task_count, value.created_at,
       value.stack_namespace, value.updated_at, value.force_update, value.image,
       FALSE, value.labels, value.runtime_hash, value.mode, value.name,
       value.network_ids, $3,
       CASE WHEN value.stack_id IS NOT NULL AND owner.id IS NULL AND value.ownership='CitadelStack'
            THEN 'DockerStackExternal' ELSE value.ownership END, value.ports,
       value.running_task_count, value.secret_ids, value.update_message,
       value.update_state, value.version_index,
       CASE WHEN value.stack_id IS NOT NULL AND owner.id IS NULL AND value.ownership='CitadelStack'
            THEN 'The Citadel Stack owner no longer exists. This Docker Stack can be imported.'
            ELSE value.ownership_diagnostic END,
       value.swarm_service_id, owner.id
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, version_index bigint, name text, mode text, image text,
    running_task_count integer, desired_task_count integer, update_state text,
    update_message text, ports jsonb, network_ids jsonb, secret_ids jsonb,
    config_ids jsonb, labels jsonb, stack_namespace text, force_update bigint,
    runtime_hash text, created_at timestamptz, updated_at timestamptz,
    ownership text, ownership_diagnostic text, swarm_service_id uuid, stack_id uuid)
LEFT JOIN stacks owner ON owner.id=value.stack_id
    AND EXISTS(SELECT 1 FROM stackreleases release WHERE release.id=owner.currentstackreleaseid AND release.platformid=$1)
WHERE TRUE
ON CONFLICT (platformid, dockerserviceid) DO UPDATE SET
    configids = EXCLUDED.configids, desiredtaskcount = EXCLUDED.desiredtaskcount,
    dockercreatedat = EXCLUDED.dockercreatedat,
    dockerstacknamespace = EXCLUDED.dockerstacknamespace,
    dockerupdatedat = EXCLUDED.dockerupdatedat, forceupdate = EXCLUDED.forceupdate,
    image = EXCLUDED.image, isstale = FALSE, labels = EXCLUDED.labels,
    liveruntimehash = EXCLUDED.liveruntimehash, mode = EXCLUDED.mode,
    name = EXCLUDED.name, networkids = EXCLUDED.networkids,
    observedat = EXCLUDED.observedat, ports = EXCLUDED.ports,
    runningtaskcount = EXCLUDED.runningtaskcount, secretids = EXCLUDED.secretids,
    updatemessage = EXCLUDED.updatemessage, updatestate = EXCLUDED.updatestate,
    versionindex = EXCLUDED.versionindex,
    ownership = CASE WHEN swarmserviceprojections.stackid IS NULL
                     THEN EXCLUDED.ownership ELSE 'CitadelStack' END,
    ownershipdiagnostic = CASE WHEN swarmserviceprojections.stackid IS NULL
                               THEN EXCLUDED.ownershipdiagnostic ELSE NULL END,
    swarmserviceid = EXCLUDED.swarmserviceid,
    stackid = COALESCE(swarmserviceprojections.stackid, EXCLUDED.stackid)
"#,
    )
    .bind(snapshot.platform_id)
    .bind(services)
    .bind(observed_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;

    // Adoption links the existing Docker identity before labels change on Apply.
    // Retain that persisted ownership through subsequent daemon snapshots.
    sqlx::query(
        "UPDATE swarmserviceprojections p SET ownership='CitadelService',swarmserviceid=s.id,ownershipdiagnostic=NULL FROM swarmservices s WHERE p.platformid=$1 AND s.platformid=p.platformid AND s.dockerserviceid=p.dockerserviceid AND p.stackid IS NULL AND p.ownership IN ('Unmanaged','CitadelService')"
    ).bind(snapshot.platform_id).execute(&mut **transaction).await.map_err(storage)?;
    sqlx::query(
        "UPDATE swarmserviceprojections p SET ownership='Unmanaged',swarmserviceid=NULL,ownershipdiagnostic='Orphaned Citadel Service ownership' WHERE p.platformid=$1 AND p.ownership='CitadelService' AND p.stackid IS NULL AND NOT EXISTS(SELECT 1 FROM swarmservices s WHERE s.id=p.swarmserviceid AND s.platformid=p.platformid)"
    ).bind(snapshot.platform_id).execute(&mut **transaction).await.map_err(storage)?;

    // Docker labels can outlive their Citadel owner. Preserve real imported
    // associations, but never keep a deleted Stack as the owner of a Service.
    sqlx::query("UPDATE swarmserviceprojections p SET ownership=CASE WHEN dockerstacknamespace IS NULL THEN 'Unmanaged' ELSE 'DockerStackExternal' END,stackid=NULL,ownershipdiagnostic='Orphaned Citadel Stack ownership' WHERE p.platformid=$1 AND p.stackid IS NOT NULL AND NOT EXISTS(SELECT 1 FROM stacks s JOIN stackreleases r ON r.id=s.currentstackreleaseid WHERE s.id=p.stackid AND r.platformid=p.platformid)")
        .bind(snapshot.platform_id).execute(&mut **transaction).await.map_err(storage)?;

    sqlx::query("UPDATE swarmserviceprojections p SET ownership='OwnershipConflict',ownershipdiagnostic='Multiple Docker identities claim the Citadel Service.' WHERE p.platformid=$1 AND NOT p.isstale AND p.swarmserviceid IS NOT NULL AND (EXISTS(SELECT 1 FROM swarmserviceprojections other WHERE other.platformid=p.platformid AND other.swarmserviceid=p.swarmserviceid AND other.dockerserviceid<>p.dockerserviceid AND NOT other.isstale) OR EXISTS(SELECT 1 FROM swarmservices owner WHERE owner.id=p.swarmserviceid AND owner.platformid=p.platformid AND owner.dockerserviceid IS NOT NULL AND owner.dockerserviceid<>p.dockerserviceid))")
        .bind(snapshot.platform_id).execute(&mut **transaction).await.map_err(storage)?;

    let tasks = json(&swarm.tasks)?;
    sqlx::query(
        r#"
INSERT INTO swarmtaskprojections (
    platformid, dockertaskid, desiredstate, dockercontainerid, dockercreatedat,
    dockernodeid, dockerserviceid, dockerupdatedat, error, image, isstale, name,
    nodehostname, observedat, ports, servicename, slot, state, statusmessage,
    statustimestamp, versionindex)
SELECT $1, value.id, value.desired_state, value.container_id, value.created_at,
       value.node_id, value.service_id, value.updated_at, value.error, value.image,
       FALSE, value.name, COALESCE(node.hostname, ''), $3, value.ports,
       COALESCE(service.name, ''), value.slot, value.state, value.status_message,
       value.status_timestamp, value.version_index
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, version_index bigint, name text, service_id text, slot integer,
    node_id text, desired_state text, state text, status_message text, error text,
    image text, ports jsonb, container_id text, status_timestamp timestamptz,
    created_at timestamptz, updated_at timestamptz)
LEFT JOIN swarmnodeprojections node
  ON node.platformid = $1 AND node.dockernodeid = value.node_id
LEFT JOIN swarmserviceprojections service
  ON service.platformid = $1 AND service.dockerserviceid = value.service_id
ON CONFLICT (platformid, dockertaskid) DO UPDATE SET
    desiredstate = EXCLUDED.desiredstate, dockercontainerid = EXCLUDED.dockercontainerid,
    dockercreatedat = EXCLUDED.dockercreatedat, dockernodeid = EXCLUDED.dockernodeid,
    dockerserviceid = EXCLUDED.dockerserviceid, dockerupdatedat = EXCLUDED.dockerupdatedat,
    error = EXCLUDED.error, image = EXCLUDED.image, isstale = FALSE,
    name = EXCLUDED.name, nodehostname = EXCLUDED.nodehostname,
    observedat = EXCLUDED.observedat, ports = EXCLUDED.ports,
    servicename = EXCLUDED.servicename, slot = EXCLUDED.slot, state = EXCLUDED.state,
    statusmessage = EXCLUDED.statusmessage, statustimestamp = EXCLUDED.statustimestamp,
    versionindex = EXCLUDED.versionindex
"#,
    )
    .bind(snapshot.platform_id)
    .bind(tasks)
    .bind(observed_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;

    crate::persistence::postgres::platforms::swarm::reconciliation::reconcile(
        transaction,
        snapshot,
        swarm,
    )
    .await
    .map_err(storage)?;
    crate::persistence::postgres::platforms::swarm::stack_status::reconcile(
        transaction,
        snapshot,
        swarm,
    )
    .await
    .map_err(storage)?;
    persist_configs(transaction, snapshot, swarm).await?;
    persist_secrets(transaction, snapshot, swarm).await?;
    persist_swarm_networks(transaction, snapshot).await?;
    update_swarm_counts(transaction, snapshot.platform_id).await?;
    cleanup_stale(transaction, snapshot.platform_id).await
}

async fn persist_configs(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    swarm: &RuntimeSwarmInventory,
) -> Result<(), RuntimeCapabilityError> {
    sqlx::query(
        r#"
INSERT INTO swarmconfigprojections (
    platformid, dockerconfigid, dockercreatedat, dockerupdatedat, isstale,
    labels, name, observedat, servicenames, templatingdriver, versionindex)
SELECT $1, value.id, value.created_at, value.updated_at, FALSE, value.labels,
       value.name, $3, '[]'::jsonb, value.templating_driver, value.version_index
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, version_index bigint, name text, templating_driver text,
    labels jsonb, created_at timestamptz, updated_at timestamptz)
ON CONFLICT (platformid, dockerconfigid) DO UPDATE SET
    dockercreatedat = EXCLUDED.dockercreatedat, dockerupdatedat = EXCLUDED.dockerupdatedat,
    isstale = FALSE, labels = EXCLUDED.labels, name = EXCLUDED.name,
    observedat = EXCLUDED.observedat, templatingdriver = EXCLUDED.templatingdriver,
    versionindex = EXCLUDED.versionindex
"#,
    )
    .bind(snapshot.platform_id)
    .bind(json(&swarm.configs)?)
    .bind(snapshot.observed_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn persist_secrets(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    swarm: &RuntimeSwarmInventory,
) -> Result<(), RuntimeCapabilityError> {
    sqlx::query(
        r#"
INSERT INTO swarmsecretprojections (
    platformid, dockersecretid, dockercreatedat, dockerupdatedat, driver,
    isstale, labels, name, observedat, servicenames, versionindex)
SELECT $1, value.id, value.created_at, value.updated_at, value.driver, FALSE,
       value.labels, value.name, $3, '[]'::jsonb, value.version_index
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, version_index bigint, name text, driver text,
    labels jsonb, created_at timestamptz, updated_at timestamptz)
ON CONFLICT (platformid, dockersecretid) DO UPDATE SET
    dockercreatedat = EXCLUDED.dockercreatedat, dockerupdatedat = EXCLUDED.dockerupdatedat,
    driver = EXCLUDED.driver, isstale = FALSE, labels = EXCLUDED.labels,
    name = EXCLUDED.name, observedat = EXCLUDED.observedat,
    versionindex = EXCLUDED.versionindex
"#,
    )
    .bind(snapshot.platform_id)
    .bind(json(&swarm.secrets)?)
    .bind(snapshot.observed_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn persist_swarm_networks(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
) -> Result<(), RuntimeCapabilityError> {
    let networks = snapshot
        .networks
        .iter()
        .filter(|network| network.scope.eq_ignore_ascii_case("swarm"))
        .collect::<Vec<_>>();
    sqlx::query(
        r#"
INSERT INTO swarmnetworkprojections (
    platformid, dockernetworkid, dockercreatedat, driver, enableipv6,
    isattachable, isencrypted, isingress, isinternal, isstale, labels, name,
    observedat, scope, servicenames, subnets)
SELECT $1, value.id, NULLIF(value.created, '')::timestamptz, value.driver,
       value.enable_ipv6, value.attachable,
       CASE lower(COALESCE(value.options->>'encrypted', 'false'))
           WHEN 'true' THEN TRUE ELSE FALSE END,
       value.ingress,
       value.internal, FALSE, value.labels, value.name, $3, value.scope,
       '[]'::jsonb, COALESCE((
           SELECT jsonb_agg(COALESCE(config->>'Subnet', config->>'subnet'))
           FROM jsonb_array_elements(
               COALESCE(value.ipam->'Config', value.ipam->'config', '[]'::jsonb)
           ) config
           WHERE COALESCE(config->>'Subnet', config->>'subnet') IS NOT NULL
       ), '[]'::jsonb)
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, name text, created text, driver text, scope text,
    enable_ipv6 boolean, internal boolean, attachable boolean, ingress boolean,
    ipam jsonb, options jsonb, labels jsonb)
ON CONFLICT (platformid, dockernetworkid) DO UPDATE SET
    dockercreatedat = EXCLUDED.dockercreatedat, driver = EXCLUDED.driver,
    enableipv6 = EXCLUDED.enableipv6, isattachable = EXCLUDED.isattachable,
    isencrypted = EXCLUDED.isencrypted, isingress = EXCLUDED.isingress,
    isinternal = EXCLUDED.isinternal, isstale = FALSE, labels = EXCLUDED.labels,
    name = EXCLUDED.name, observedat = EXCLUDED.observedat, scope = EXCLUDED.scope,
    subnets = EXCLUDED.subnets
"#,
    )
    .bind(snapshot.platform_id)
    .bind(json(&networks)?)
    .bind(snapshot.observed_at)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn update_swarm_counts(
    transaction: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
) -> Result<(), RuntimeCapabilityError> {
    sqlx::query(
        "UPDATE swarmnodeprojections SET runningtaskcount = 0, desiredtaskcount = 0 WHERE platformid = $1",
    )
    .bind(platform_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;

    sqlx::query(
        r#"
UPDATE swarmnodeprojections node
SET runningtaskcount = counts.running,
    desiredtaskcount = counts.desired
FROM (
    SELECT dockernodeid,
           COUNT(*) FILTER (WHERE state = 'running' AND NOT isstale)::integer AS running,
           COUNT(*) FILTER (WHERE desiredstate = 'running' AND NOT isstale)::integer AS desired
    FROM swarmtaskprojections WHERE platformid = $1 GROUP BY dockernodeid
) counts
WHERE node.platformid = $1 AND node.dockernodeid = counts.dockernodeid
"#,
    )
    .bind(platform_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;

    for (table, id_column, service_column) in [
        ("swarmconfigprojections", "dockerconfigid", "configids"),
        ("swarmsecretprojections", "dockersecretid", "secretids"),
        ("swarmnetworkprojections", "dockernetworkid", "networkids"),
    ] {
        let statement = format!(
            r#"
UPDATE {table} resource
SET servicenames = COALESCE((
    SELECT jsonb_agg(service.name ORDER BY service.name)
    FROM swarmserviceprojections service
    WHERE service.platformid = $1 AND NOT service.isstale
      AND service.{service_column} ? resource.{id_column}
), '[]'::jsonb)
WHERE resource.platformid = $1
"#
        );
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(platform_id)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?;
    }
    Ok(())
}

async fn cleanup_stale(
    transaction: &mut Transaction<'_, Postgres>,
    platform_id: uuid::Uuid,
) -> Result<(), RuntimeCapabilityError> {
    for table in [
        "swarmtaskprojections",
        "swarmserviceprojections",
        "swarmnodeprojections",
        "swarmconfigprojections",
        "swarmsecretprojections",
        "swarmnetworkprojections",
    ] {
        let statement = format!(
            "DELETE FROM {table} WHERE platformid = $1 AND isstale AND observedat < now() - make_interval(hours => $2)"
        );
        sqlx::query(sqlx::AssertSqlSafe(statement))
            .bind(platform_id)
            .bind(STALE_RETENTION_HOURS)
            .execute(&mut **transaction)
            .await
            .map_err(storage)?;
    }
    Ok(())
}

pub(crate) fn json(value: &(impl Serialize + ?Sized)) -> Result<Value, RuntimeCapabilityError> {
    serde_json::to_value(value).map_err(|error| {
        RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), false)
    })
}

fn bounded_i32(value: i64) -> i32 {
    i32::try_from(value).unwrap_or_else(|_| {
        if value.is_negative() {
            i32::MIN
        } else {
            i32::MAX
        }
    })
}

pub(crate) fn storage(error: impl std::fmt::Display) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(RuntimeErrorKind::Remote, error.to_string(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_counts_are_bounded_before_postgres_integer_storage() {
        assert_eq!(bounded_i32(i64::MAX), i32::MAX);
        assert_eq!(bounded_i32(i64::MIN), i32::MIN);
    }
}
