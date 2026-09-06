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
    pool: PgPool,
}

impl PostgresInventoryProjectionStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Initialize a missing Swarm inventory without overwriting a reconciliation
    /// which completed while Docker was being queried. No network I/O holds this lock.
    pub async fn initialize_swarm(
        &self,
        snapshot: &RuntimeInventorySnapshot,
    ) -> Result<bool, RuntimeCapabilityError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let saved = sqlx::query_as::<_, (Option<String>, Value)>(
            "SELECT clusterid,platformdescriptor FROM platforms WHERE id=$1 FOR UPDATE",
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
        persist_snapshot(&mut tx, snapshot).await?;
        tx.commit().await.map_err(storage)?;
        Ok(true)
    }
}

impl InventoryProjectionStore for PostgresInventoryProjectionStore {
    fn persist<'a>(
        &'a self,
        snapshot: &'a RuntimeInventorySnapshot,
    ) -> BoxFuture<'a, Result<InventoryProjectionChange, RuntimeCapabilityError>> {
        async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            persist_snapshot(&mut transaction, snapshot).await?;
            transaction.commit().await.map_err(storage)?;
            Ok(InventoryProjectionChange {
                platform_id: snapshot.platform_id,
                revision: snapshot.observed_at.timestamp_millis(),
            })
        }
        .boxed()
    }
}

pub(crate) async fn persist_snapshot(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
) -> Result<(), RuntimeCapabilityError> {
    persist_platform(transaction, snapshot).await?;
    persist_images(transaction, snapshot).await?;
    persist_containers(transaction, snapshot, None).await?;
    if let Some(swarm) = &snapshot.swarm {
        persist_swarm(transaction, snapshot, swarm).await?;
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
            serde_json::json!(
                swarm
                    .tasks
                    .iter()
                    .filter(|task| task.state.eq_ignore_ascii_case("running"))
                    .count()
            ),
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
    platformdescriptor = (platformdescriptor::jsonb || $9::jsonb)::json,
    status = 'Online'
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
    let payload = json(&snapshot.images)?;
    sqlx::query(
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
    RETURNING dockerimageid
)
DELETE FROM images image
WHERE image.platformid = $1
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE incoming.id = image.dockerimageid)
"#,
    )
    .bind(snapshot.platform_id)
    .bind(payload)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

pub(crate) async fn persist_containers(
    transaction: &mut Transaction<'_, Postgres>,
    snapshot: &RuntimeInventorySnapshot,
    node_id: Option<&str>,
) -> Result<(), RuntimeCapabilityError> {
    let payload = json(&snapshot.containers)?;
    let observed = snapshot.observed_at.timestamp();
    let conflict = if node_id.is_some() {
        "(dockercontainerid, platformid, dockernodeid) WHERE dockernodeid IS NOT NULL"
    } else {
        "(dockercontainerid, platformid) WHERE dockernodeid IS NULL"
    };
    sqlx::query(sqlx::AssertSqlSafe(format!(
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
        projectionobservedat, dockernodeid)
    SELECT gen_random_uuid(), $1, incoming.id, incoming.name, incoming."imageId",
           incoming.created, initcap(incoming.state), 'Idle', $3, incoming.stack,
           incoming."isSystem", incoming."systemRole",
           incoming."hasCitadelOwnershipLabels", incoming."isSwarmTask",
           COALESCE(incoming.ports, '[]'::jsonb)::json, 0, $3, $4
    FROM incoming
    ON CONFLICT {conflict} DO UPDATE
    SET name = EXCLUDED.name,
        dockerimageid = EXCLUDED.dockerimageid,
        state = EXCLUDED.state,
        updated = EXCLUDED.updated,
        stack = EXCLUDED.stack,
        issystem = EXCLUDED.issystem,
        systemrole = EXCLUDED.systemrole,
        hascitadelownershiplabels = EXCLUDED.hascitadelownershiplabels,
        isswarmtask = EXCLUDED.isswarmtask,
        ports = EXCLUDED.ports,
        projectionobservedat = EXCLUDED.projectionobservedat,
        projectionstalesince = NULL,
        projectionstalereason = NULL,
        rowversion = containers.rowversion + 1
    WHERE COALESCE(containers.projectionobservedat, 0) <= EXCLUDED.projectionobservedat
    RETURNING dockercontainerid
)
DELETE FROM containers container
WHERE container.platformid = $1
  AND container.dockernodeid IS NOT DISTINCT FROM $4
  AND COALESCE(container.projectionobservedat, 0) <= $3
  AND NOT EXISTS (SELECT 1 FROM incoming WHERE incoming.id = container.dockercontainerid)
"#
    )))
    .bind(snapshot.platform_id)
    .bind(payload)
    .bind(observed)
    .bind(node_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    sqlx::query(
        r#"
UPDATE containers container
SET imageid = image.id
FROM images image
WHERE container.platformid = $1
  AND container.dockernodeid IS NOT DISTINCT FROM $2
  AND image.platformid = container.platformid
  AND image.dockerimageid = container.dockerimageid
"#,
    )
    .bind(snapshot.platform_id)
    .bind(node_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}

async fn persist_swarm(
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

    let services = json(&swarm.services)?;
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
       value.network_ids, $3, value.ownership, value.ports,
       value.running_task_count, value.secret_ids, value.update_message,
       value.update_state, value.version_index, value.ownership_diagnostic,
       value.swarm_service_id, value.stack_id
FROM jsonb_to_recordset($2::jsonb) AS value(
    id text, version_index bigint, name text, mode text, image text,
    running_task_count integer, desired_task_count integer, update_state text,
    update_message text, ports jsonb, network_ids jsonb, secret_ids jsonb,
    config_ids jsonb, labels jsonb, stack_namespace text, force_update bigint,
    runtime_hash text, created_at timestamptz, updated_at timestamptz,
    ownership text, ownership_diagnostic text, swarm_service_id uuid, stack_id uuid)
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

pub(crate) fn json(value: &impl Serialize) -> Result<Value, RuntimeCapabilityError> {
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
