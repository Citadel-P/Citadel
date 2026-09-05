use std::collections::BTreeMap;

use citadel_domain::ActorId;
use citadel_platforms::{
    AuthorizedReadError, ContainerStatView, ContainerView, EffectivePlatformPermission, ImageView,
    PlatformReadStore, PlatformStatView, PlatformView, SwarmConfigView, SwarmNetworkView,
    SwarmNodeView, SwarmSecretView, SwarmServiceView, SwarmTaskView, WorkloadStatusCounts,
};
use futures_util::future::BoxFuture;
use serde::de::DeserializeOwned;
use serde_json::Value;
use sqlx::postgres::PgRow;
use sqlx::{AssertSqlSafe, PgPool, Row};
use uuid::Uuid;

const PLATFORM_RESOURCE_TYPE: i32 = 0;
const READ_PERMISSION_MASK: i32 = 1 | 2 | 4;

const PLATFORM_SELECT: &str = r#"
SELECT p.*,
       deployment_counts.total AS deployment_count,
       deployment_counts.healthy AS deployment_healthy,
       deployment_counts.degraded AS deployment_degraded,
       deployment_counts.failed AS deployment_failed,
       deployment_counts.stopped AS deployment_stopped,
       deployment_counts.in_progress AS deployment_in_progress,
       deployment_counts.unknown AS deployment_unknown,
       stack_counts.total AS stack_count,
       stack_counts.healthy AS stack_healthy,
       stack_counts.degraded AS stack_degraded,
       stack_counts.failed AS stack_failed,
       stack_counts.stopped AS stack_stopped,
       stack_counts.paused AS stack_paused,
       stack_counts.in_progress AS stack_in_progress,
       stack_counts.unknown AS stack_unknown,
       service_counts.total AS service_count,
       service_counts.healthy AS service_healthy,
       service_counts.degraded AS service_degraded,
       service_counts.failed AS service_failed,
       service_counts.stopped AS service_stopped,
       service_counts.in_progress AS service_in_progress,
       service_counts.unknown AS service_unknown,
       stat.created AS stat_created,
       stat.cpuusage AS stat_cpuusage,
       stat.memoryusage AS stat_memoryusage,
       stat.rxbytes AS stat_rxbytes,
       stat.txbytes AS stat_txbytes,
       stat.diskusedbytes AS stat_diskusedbytes,
       stat.disktotalbytes AS stat_disktotalbytes,
       stat.diskusage AS stat_diskusage
FROM platforms p
LEFT JOIN LATERAL (
    SELECT COUNT(*) AS total,
           COUNT(*) FILTER (WHERE status = 'Healthy') AS healthy,
           COUNT(*) FILTER (WHERE status = 'Degraded') AS degraded,
           COUNT(*) FILTER (WHERE status = 'Failed') AS failed,
           COUNT(*) FILTER (WHERE status = 'Stopped') AS stopped,
           COUNT(*) FILTER (WHERE status IN ('Created', 'Pending', 'Applying')) AS in_progress,
           COUNT(*) FILTER (WHERE status = 'Unknown') AS unknown
    FROM deployments WHERE platformid = p.id
) deployment_counts ON TRUE
LEFT JOIN LATERAL (
    SELECT COUNT(*) AS total,
           COUNT(*) FILTER (WHERE release.status = 'Healthy') AS healthy,
           COUNT(*) FILTER (WHERE release.status = 'Degraded') AS degraded,
           COUNT(*) FILTER (WHERE release.status = 'Failed') AS failed,
           COUNT(*) FILTER (WHERE release.status = 'Stopped') AS stopped,
           COUNT(*) FILTER (WHERE release.status = 'Paused') AS paused,
           COUNT(*) FILTER (WHERE release.status IN ('Created', 'Pending', 'Applying')) AS in_progress,
           COUNT(*) FILTER (WHERE release.status = 'Unknown') AS unknown
    FROM stacks stack
    JOIN stackreleases release ON release.id = stack.currentstackreleaseid
    WHERE release.platformid = p.id
) stack_counts ON TRUE
LEFT JOIN LATERAL (
    SELECT COUNT(*) AS total,
           COUNT(*) FILTER (WHERE effective_health = 'Healthy') AS healthy,
           COUNT(*) FILTER (WHERE effective_health = 'Degraded') AS degraded,
           COUNT(*) FILTER (WHERE effective_health = 'Failed') AS failed,
           COUNT(*) FILTER (WHERE effective_health = 'Stopped') AS stopped,
           COUNT(*) FILTER (WHERE effective_health IN ('Created', 'Progressing')) AS in_progress,
           COUNT(*) FILTER (WHERE effective_health = 'Unknown') AS unknown
    FROM (
        SELECT CASE
          WHEN service.dockerserviceid IS NULL THEN service.health
          WHEN projection.dockerserviceid IS NULL OR projection.isstale THEN 'Unknown'
          WHEN lower(COALESCE(projection.updatestate,'')) IN ('paused','rollback_paused','rollback_completed') THEN 'Failed'
          WHEN projection.desiredtaskcount=0 THEN 'Stopped'
          WHEN projection.runningtaskcount>=projection.desiredtaskcount THEN 'Healthy'
          WHEN projection.runningtaskcount>0 THEN 'Degraded'
          ELSE 'Progressing'
        END effective_health
        FROM swarmservices service
        LEFT JOIN swarmserviceprojections projection
          ON projection.platformid=service.platformid
         AND projection.dockerserviceid=service.dockerserviceid
        WHERE service.platformid=p.id
    ) states
) service_counts ON TRUE
LEFT JOIN LATERAL (
    SELECT * FROM platformstats WHERE platformid = p.id ORDER BY created DESC LIMIT 1
) stat ON TRUE
"#;

#[derive(Clone)]
pub struct PostgresPlatformReadStore {
    pool: PgPool,
}

impl PostgresPlatformReadStore {
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

impl PlatformReadStore for PostgresPlatformReadStore {
    fn list_authorized<'a>(
        &'a self,
        actor_id: ActorId,
        is_administrator: bool,
        tag_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<Vec<PlatformView>, AuthorizedReadError>> {
        Box::pin(async move {
            let query = format!(
                r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
), global_access AS (
    SELECT EXISTS (
        SELECT 1 FROM actor_scope scope
        JOIN actorroles assignment ON assignment.actorid = scope.actorid
        JOIN permissions permission ON permission.roleid = assignment.roleid
        WHERE permission.resourcetype = $2
          AND (permission.permissionlevel & $3) <> 0
    ) AS allowed
)
{PLATFORM_SELECT}
WHERE ($4 OR (SELECT allowed FROM global_access) OR EXISTS (
    SELECT 1 FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $2
      AND access.resourceid = p.id
      AND (access.permissionlevel & $3) <> 0
))
AND (cardinality($5::uuid[]) = 0 OR EXISTS (
    SELECT 1 FROM resourcetags tag
    WHERE tag.resourcetype = 'Platform'
      AND tag.resourceid = p.id
      AND tag.tagid = ANY($5::uuid[])
))
ORDER BY p.name, p.id
"#
            );
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(actor_id.value())
                .bind(PLATFORM_RESOURCE_TYPE)
                .bind(READ_PERMISSION_MASK)
                .bind(is_administrator)
                .bind(tag_ids)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_platform)
                .collect()
        })
    }

    fn get_platform(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<PlatformView>, AuthorizedReadError>> {
        Box::pin(async move {
            let query = format!("{PLATFORM_SELECT} WHERE p.id = $1");
            sqlx::query(AssertSqlSafe(query.as_str()))
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_platform)
                .transpose()
        })
    }

    fn permissions_for_platforms<'a>(
        &'a self,
        actor_id: ActorId,
        platform_ids: &'a [Uuid],
    ) -> BoxFuture<'a, Result<BTreeMap<Uuid, EffectivePlatformPermission>, AuthorizedReadError>>
    {
        Box::pin(async move {
            if platform_ids.is_empty() {
                return Ok(BTreeMap::new());
            }
            sqlx::query(
                r#"
WITH actor_scope AS (
    SELECT actor.id AS actorid
    FROM actors actor
    WHERE actor.id = $1 AND actor.isenabled
    UNION
    SELECT team.actorid
    FROM actorteammemberships membership
    JOIN teams team ON team.id = membership.teamid
    JOIN actors team_actor ON team_actor.id = team.actorid AND team_actor.isenabled
    WHERE membership.memberactorid = $1
)
SELECT requested.id,
       COALESCE(bit_or(effective_grant.permissionlevel), 0)::integer AS levelmask,
       COALESCE(bit_or(effective_grant.specificpermissions), 0)::integer AS specificmask
FROM unnest($2::uuid[]) requested(id)
LEFT JOIN LATERAL (
    SELECT permission.permissionlevel, permission.specificpermissions
    FROM actor_scope scope
    JOIN actorroles assignment ON assignment.actorid = scope.actorid
    JOIN permissions permission ON permission.roleid = assignment.roleid
    WHERE permission.resourcetype = $3
    UNION ALL
    SELECT access.permissionlevel, access.specificpermissions
    FROM actor_scope scope
    JOIN resourceaccesses access ON access.actorid = scope.actorid
    WHERE access.resourcetype = $3 AND access.resourceid = requested.id
) effective_grant ON TRUE
GROUP BY requested.id
"#,
            )
            .bind(actor_id.value())
            .bind(platform_ids)
            .bind(PLATFORM_RESOURCE_TYPE)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(|row| {
                Ok((
                    row.try_get("id").map_err(storage)?,
                    EffectivePlatformPermission {
                        level_mask: row.try_get("levelmask").map_err(storage)?,
                        specific_mask: row.try_get("specificmask").map_err(storage)?,
                    },
                ))
            })
            .collect()
        })
    }

    fn list_containers(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ContainerView>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(CONTAINER_SELECT)
                .bind(platform_id)
                .fetch_all(&self.pool)
                .await
                .map_err(storage)?
                .into_iter()
                .map(map_container)
                .collect()
        })
    }

    fn get_container(
        &self,
        id: Uuid,
    ) -> BoxFuture<'_, Result<Option<ContainerView>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(CONTAINER_SELECT_BY_ID)
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .map_err(storage)?
                .map(map_container)
                .transpose()
        })
    }

    fn list_images(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<ImageView>, AuthorizedReadError>> {
        Box::pin(async move {
            sqlx::query(
                r#"
SELECT image.*, NULL::text AS contentidentity, NULL::text AS dockernodeid,
       NULL::text AS nodehostname, FALSE AS isstale, NULL::text AS stalereason,
       NULL::jsonb AS repodigests
FROM images image
WHERE image.platformid = $1
ORDER BY image.name, image.id
"#,
            )
            .bind(platform_id)
            .fetch_all(&self.pool)
            .await
            .map_err(storage)?
            .into_iter()
            .map(map_image)
            .collect()
        })
    }

    fn list_swarm_nodes(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNodeView>, AuthorizedReadError>> {
        Box::pin(list_swarm_nodes(&self.pool, platform_id, None))
    }

    fn get_swarm_node<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNodeView>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_nodes(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_services(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmServiceView>, AuthorizedReadError>> {
        Box::pin(list_swarm_services(&self.pool, platform_id, None))
    }

    fn get_swarm_service<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmServiceView>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_services(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_tasks<'a>(
        &'a self,
        platform_id: Uuid,
        service_id: Option<&'a str>,
        limit: usize,
    ) -> BoxFuture<'a, Result<Vec<SwarmTaskView>, AuthorizedReadError>> {
        Box::pin(list_swarm_tasks(
            &self.pool,
            platform_id,
            None,
            service_id,
            Some(limit),
        ))
    }

    fn get_swarm_task<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmTaskView>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(
                list_swarm_tasks(&self.pool, platform_id, Some(id), None, Some(1))
                    .await?
                    .pop(),
            )
        })
    }

    fn list_swarm_configs(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmConfigView>, AuthorizedReadError>> {
        Box::pin(list_swarm_configs(&self.pool, platform_id, None))
    }

    fn get_swarm_config<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmConfigView>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_configs(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_networks(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmNetworkView>, AuthorizedReadError>> {
        Box::pin(list_swarm_networks(&self.pool, platform_id, None))
    }

    fn get_swarm_network<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmNetworkView>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_networks(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }

    fn list_swarm_secrets(
        &self,
        platform_id: Uuid,
    ) -> BoxFuture<'_, Result<Vec<SwarmSecretView>, AuthorizedReadError>> {
        Box::pin(list_swarm_secrets(&self.pool, platform_id, None))
    }

    fn get_swarm_secret<'a>(
        &'a self,
        platform_id: Uuid,
        id: &'a str,
    ) -> BoxFuture<'a, Result<Option<SwarmSecretView>, AuthorizedReadError>> {
        Box::pin(async move {
            Ok(list_swarm_secrets(&self.pool, platform_id, Some(id))
                .await?
                .pop())
        })
    }
}

const CONTAINER_SELECT: &str = r#"
SELECT container.id, container.platformid, container.dockercontainerid, container.name,
       container.dockerimageid, container.created, container.state,
       COALESCE(container.controlstate, 'Idle') AS controlstate, container.updated,
       container.stack, container.issystem, container.systemrole,
       container.hascitadelownershiplabels, container.isswarmtask, container.dockernodeid,
       node.hostname AS nodehostname, container.projectionobservedat,
       container.projectionstalesince, container.projectionstalereason, container.ports,
       container.deploymentid, container.stackid,
       stat.containerid AS stat_containerid, stat.memoryactive AS stat_memoryactive,
       stat.memorycache AS stat_memorycache, stat.cpuusage AS stat_cpuusage,
       stat.memorylimit AS stat_memorylimit, stat.rxbytes AS stat_rxbytes,
       stat.txbytes AS stat_txbytes, stat.created AS stat_created
FROM containers container
LEFT JOIN swarmnodeprojections node
  ON node.platformid = container.platformid AND node.dockernodeid = container.dockernodeid
LEFT JOIN LATERAL (
    SELECT * FROM containerstats WHERE containerid = container.id ORDER BY created DESC LIMIT 1
) stat ON TRUE
WHERE container.platformid = $1
ORDER BY container.name, container.id
"#;

const CONTAINER_SELECT_BY_ID: &str = r#"
SELECT container.id, container.platformid, container.dockercontainerid, container.name,
       container.dockerimageid, container.created, container.state,
       COALESCE(container.controlstate, 'Idle') AS controlstate, container.updated,
       container.stack, container.issystem, container.systemrole,
       container.hascitadelownershiplabels, container.isswarmtask, container.dockernodeid,
       node.hostname AS nodehostname, container.projectionobservedat,
       container.projectionstalesince, container.projectionstalereason, container.ports,
       container.deploymentid, container.stackid,
       stat.containerid AS stat_containerid, stat.memoryactive AS stat_memoryactive,
       stat.memorycache AS stat_memorycache, stat.cpuusage AS stat_cpuusage,
       stat.memorylimit AS stat_memorylimit, stat.rxbytes AS stat_rxbytes,
       stat.txbytes AS stat_txbytes, stat.created AS stat_created
FROM containers container
LEFT JOIN swarmnodeprojections node
  ON node.platformid = container.platformid AND node.dockernodeid = container.dockernodeid
LEFT JOIN LATERAL (
    SELECT * FROM containerstats WHERE containerid = container.id ORDER BY created DESC LIMIT 1
) stat ON TRUE
WHERE container.id = $1
LIMIT 1
"#;

fn map_platform(row: PgRow) -> Result<PlatformView, AuthorizedReadError> {
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    let platform_type = descriptor
        .get("$type")
        .and_then(Value::as_str)
        .unwrap_or("Docker")
        .to_owned();
    let stats = row
        .try_get::<Option<i64>, _>("stat_created")
        .map_err(storage)?
        .map(|created| {
            Ok(PlatformStatView {
                created,
                tx_bytes: row.try_get("stat_txbytes").map_err(storage)?,
                rx_bytes: row.try_get("stat_rxbytes").map_err(storage)?,
                cpu_usage: row.try_get("stat_cpuusage").map_err(storage)?,
                memory_usage: row.try_get("stat_memoryusage").map_err(storage)?,
                disk_used_bytes: row.try_get("stat_diskusedbytes").map_err(storage)?,
                disk_total_bytes: row.try_get("stat_disktotalbytes").map_err(storage)?,
                disk_usage: row.try_get("stat_diskusage").map_err(storage)?,
            })
        })
        .transpose()?
        .map(|stat| vec![stat]);
    Ok(PlatformView {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        address: row.try_get("address").map_err(storage)?,
        network_count: row.try_get("networkcount").map_err(storage)?,
        volume_count: row.try_get("volumecount").map_err(storage)?,
        image_count: i64::from(row.try_get::<i32, _>("imagecount").map_err(storage)?),
        cpu_count: i64::from(row.try_get::<i32, _>("cpucount").map_err(storage)?),
        mem_total: row.try_get("memtotal").map_err(storage)?,
        agent_version: row.try_get("agentversion").map_err(storage)?,
        server_version: row.try_get("serverversion").map_err(storage)?,
        platform_type,
        status: row.try_get("status").map_err(storage)?,
        connector_type: row.try_get("connectortype").map_err(storage)?,
        deployment_count: row.try_get("deployment_count").map_err(storage)?,
        stack_count: row.try_get("stack_count").map_err(storage)?,
        deployment_status_counts: counts(&row, "deployment", false)?,
        stack_status_counts: counts(&row, "stack", true)?,
        swarm_service_status_counts: counts(&row, "service", false)?,
        stats,
        platform_descriptor: descriptor,
        cluster_id: row.try_get("clusterid").map_err(storage)?,
        prune_historical_swarm_task_containers: row
            .try_get("prunehistoricalswarmtaskcontainers")
            .map_err(storage)?,
        capabilities: None,
    })
}

fn counts(
    row: &PgRow,
    prefix: &str,
    has_paused: bool,
) -> Result<WorkloadStatusCounts, AuthorizedReadError> {
    let column = |suffix| format!("{prefix}_{suffix}");
    let total = row.try_get(column("count").as_str()).map_err(storage)?;
    Ok(WorkloadStatusCounts {
        total,
        healthy: row.try_get(column("healthy").as_str()).map_err(storage)?,
        degraded: row.try_get(column("degraded").as_str()).map_err(storage)?,
        failed: row.try_get(column("failed").as_str()).map_err(storage)?,
        stopped: row.try_get(column("stopped").as_str()).map_err(storage)?,
        paused: if has_paused {
            row.try_get(column("paused").as_str()).map_err(storage)?
        } else {
            0
        },
        in_progress: row
            .try_get(column("in_progress").as_str())
            .map_err(storage)?,
        unknown: row.try_get(column("unknown").as_str()).map_err(storage)?,
    })
}

fn map_container(row: PgRow) -> Result<ContainerView, AuthorizedReadError> {
    let last_stats = row
        .try_get::<Option<Uuid>, _>("stat_containerid")
        .map_err(storage)?
        .map(|container_id| {
            Ok(ContainerStatView {
                container_id,
                memory_active: row.try_get("stat_memoryactive").map_err(storage)?,
                memory_cache: row.try_get("stat_memorycache").map_err(storage)?,
                cpu_usage: row.try_get("stat_cpuusage").map_err(storage)?,
                memory_limit: row.try_get("stat_memorylimit").map_err(storage)?,
                rx_bytes: row.try_get("stat_rxbytes").map_err(storage)?,
                tx_bytes: row.try_get("stat_txbytes").map_err(storage)?,
                created: row.try_get("stat_created").map_err(storage)?,
            })
        })
        .transpose()?;
    Ok(ContainerView {
        id: row.try_get("id").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        container_id: row.try_get("dockercontainerid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        docker_image_id: row.try_get("dockerimageid").map_err(storage)?,
        created: row.try_get("created").map_err(storage)?,
        state: row.try_get("state").map_err(storage)?,
        control_state: row.try_get("controlstate").map_err(storage)?,
        updated: row.try_get("updated").map_err(storage)?,
        stack: row.try_get("stack").map_err(storage)?,
        is_system: row.try_get("issystem").map_err(storage)?,
        system_role: row.try_get("systemrole").map_err(storage)?,
        has_citadel_ownership_labels: row.try_get("hascitadelownershiplabels").map_err(storage)?,
        is_swarm_task: row.try_get("isswarmtask").map_err(storage)?,
        docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
        node_hostname: row.try_get("nodehostname").map_err(storage)?,
        projection_observed_at: row.try_get("projectionobservedat").map_err(storage)?,
        projection_stale_since: row.try_get("projectionstalesince").map_err(storage)?,
        projection_stale_reason: row.try_get("projectionstalereason").map_err(storage)?,
        last_stats,
        ports: crate::container_ports::normalize(row.try_get("ports").map_err(storage)?),
        deployment_id: row.try_get("deploymentid").map_err(storage)?,
        stack_id: row.try_get("stackid").map_err(storage)?,
        capabilities: None,
    })
}

fn map_image(row: PgRow) -> Result<ImageView, AuthorizedReadError> {
    Ok(ImageView {
        id: row.try_get("id").map_err(storage)?,
        tags: json(row.try_get("tags").map_err(storage)?)?,
        name: row.try_get("name").map_err(storage)?,
        docker_image_id: row.try_get("dockerimageid").map_err(storage)?,
        size: row.try_get("size").map_err(storage)?,
        is_in_use: row.try_get::<i32, _>("containers").map_err(storage)? > 0,
        platform_id: row.try_get("platformid").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        control_state: row
            .try_get::<Option<String>, _>("controlstate")
            .map_err(storage)?
            .unwrap_or_else(|| "Idle".to_owned()),
        updated_at: row.try_get("updatedat").map_err(storage)?,
        registry_id: row.try_get("registryid").map_err(storage)?,
        repo_digests: row
            .try_get::<Option<Value>, _>("repodigests")
            .map_err(storage)?
            .map(json)
            .transpose()?,
        content_identity: row.try_get("contentidentity").map_err(storage)?,
        docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
        node_hostname: row.try_get("nodehostname").map_err(storage)?,
        is_stale: row.try_get("isstale").map_err(storage)?,
        stale_reason: row.try_get("stalereason").map_err(storage)?,
        capabilities: None,
    })
}

async fn list_swarm_nodes(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmNodeView>, AuthorizedReadError> {
    sqlx::query(
        r#"
SELECT * FROM swarmnodeprojections
WHERE platformid = $1 AND ($2::text IS NULL OR dockernodeid = $2)
ORDER BY hostname, dockernodeid
"#,
    )
    .bind(platform_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        Ok(SwarmNodeView {
            id: row.try_get("dockernodeid").map_err(storage)?,
            version_index: row.try_get("versionindex").map_err(storage)?,
            hostname: row.try_get("hostname").map_err(storage)?,
            role: row.try_get("role").map_err(storage)?,
            is_leader: row.try_get("isleader").map_err(storage)?,
            reachability: row.try_get("reachability").map_err(storage)?,
            status: row.try_get("status").map_err(storage)?,
            status_message: row.try_get("statusmessage").map_err(storage)?,
            availability: row.try_get("availability").map_err(storage)?,
            engine_version: row.try_get("engineversion").map_err(storage)?,
            operating_system: row.try_get("operatingsystem").map_err(storage)?,
            architecture: row.try_get("architecture").map_err(storage)?,
            address: row.try_get("address").map_err(storage)?,
            labels: json(row.try_get("labels").map_err(storage)?)?,
            running_task_count: row.try_get("runningtaskcount").map_err(storage)?,
            desired_task_count: row.try_get("desiredtaskcount").map_err(storage)?,
            created_at: row.try_get("dockercreatedat").map_err(storage)?,
            updated_at: row.try_get("dockerupdatedat").map_err(storage)?,
            observed_at: row.try_get("observedat").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            capabilities: None,
        })
    })
    .collect()
}

async fn list_swarm_services(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmServiceView>, AuthorizedReadError> {
    sqlx::query(
        r#"
SELECT * FROM swarmserviceprojections
WHERE platformid = $1 AND ($2::text IS NULL OR dockerserviceid = $2)
ORDER BY name, dockerserviceid
"#,
    )
    .bind(platform_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        Ok(SwarmServiceView {
            id: row.try_get("dockerserviceid").map_err(storage)?,
            version_index: row.try_get("versionindex").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            mode: row.try_get("mode").map_err(storage)?,
            image: row.try_get("image").map_err(storage)?,
            running_task_count: row.try_get("runningtaskcount").map_err(storage)?,
            desired_task_count: row.try_get("desiredtaskcount").map_err(storage)?,
            update_state: row.try_get("updatestate").map_err(storage)?,
            update_message: row.try_get("updatemessage").map_err(storage)?,
            ports: json(row.try_get("ports").map_err(storage)?)?,
            network_ids: json(row.try_get("networkids").map_err(storage)?)?,
            secret_ids: json(row.try_get("secretids").map_err(storage)?)?,
            config_ids: json(row.try_get("configids").map_err(storage)?)?,
            labels: json(row.try_get("labels").map_err(storage)?)?,
            ownership: row.try_get("ownership").map_err(storage)?,
            docker_stack_namespace: row.try_get("dockerstacknamespace").map_err(storage)?,
            ownership_diagnostic: row.try_get("ownershipdiagnostic").map_err(storage)?,
            stack_id: row.try_get("stackid").map_err(storage)?,
            swarm_service_id: row.try_get("swarmserviceid").map_err(storage)?,
            created_at: row.try_get("dockercreatedat").map_err(storage)?,
            updated_at: row.try_get("dockerupdatedat").map_err(storage)?,
            observed_at: row.try_get("observedat").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            capabilities: None,
        })
    })
    .collect()
}

async fn list_swarm_tasks(
    pool: &PgPool,
    platform_id: Uuid,
    task_id: Option<&str>,
    service_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<SwarmTaskView>, AuthorizedReadError> {
    sqlx::query(
        r#"
SELECT * FROM swarmtaskprojections
WHERE platformid = $1
  AND ($2::text IS NULL OR dockertaskid = $2)
  AND ($3::text IS NULL OR dockerserviceid = $3)
ORDER BY servicename, slot NULLS LAST, name, dockertaskid
LIMIT $4
"#,
    )
    .bind(platform_id)
    .bind(task_id)
    .bind(service_id)
    .bind(i64::try_from(limit.unwrap_or(usize::MAX)).unwrap_or(i64::MAX))
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        Ok(SwarmTaskView {
            id: row.try_get("dockertaskid").map_err(storage)?,
            version_index: row.try_get("versionindex").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            service_id: row.try_get("dockerserviceid").map_err(storage)?,
            service_name: row.try_get("servicename").map_err(storage)?,
            slot: row.try_get("slot").map_err(storage)?,
            node_id: row.try_get("dockernodeid").map_err(storage)?,
            node_hostname: row.try_get("nodehostname").map_err(storage)?,
            desired_state: row.try_get("desiredstate").map_err(storage)?,
            state: row.try_get("state").map_err(storage)?,
            status_message: row.try_get("statusmessage").map_err(storage)?,
            error: row.try_get("error").map_err(storage)?,
            image: row.try_get("image").map_err(storage)?,
            ports: json(row.try_get("ports").map_err(storage)?)?,
            status_timestamp: row.try_get("statustimestamp").map_err(storage)?,
            created_at: row.try_get("dockercreatedat").map_err(storage)?,
            updated_at: row.try_get("dockerupdatedat").map_err(storage)?,
            observed_at: row.try_get("observedat").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            capabilities: None,
        })
    })
    .collect()
}

async fn list_swarm_configs(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmConfigView>, AuthorizedReadError> {
    sqlx::query(
        r#"
SELECT * FROM swarmconfigprojections
WHERE platformid = $1 AND ($2::text IS NULL OR dockerconfigid = $2)
ORDER BY name, dockerconfigid
"#,
    )
    .bind(platform_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        let service_names: Vec<String> = json(row.try_get("servicenames").map_err(storage)?)?;
        Ok(SwarmConfigView {
            id: row.try_get("dockerconfigid").map_err(storage)?,
            version_index: row.try_get("versionindex").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            templating_driver: row.try_get("templatingdriver").map_err(storage)?,
            in_use: !service_names.is_empty(),
            service_names,
            labels: json(row.try_get("labels").map_err(storage)?)?,
            created_at: row.try_get("dockercreatedat").map_err(storage)?,
            updated_at: row.try_get("dockerupdatedat").map_err(storage)?,
            observed_at: row.try_get("observedat").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            capabilities: None,
        })
    })
    .collect()
}

async fn list_swarm_networks(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmNetworkView>, AuthorizedReadError> {
    sqlx::query(
        r#"
SELECT * FROM swarmnetworkprojections
WHERE platformid = $1 AND ($2::text IS NULL OR dockernetworkid = $2)
ORDER BY name, dockernetworkid
"#,
    )
    .bind(platform_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        Ok(SwarmNetworkView {
            id: row.try_get("dockernetworkid").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            scope: row.try_get("scope").map_err(storage)?,
            driver: row.try_get("driver").map_err(storage)?,
            is_attachable: row.try_get("isattachable").map_err(storage)?,
            is_internal: row.try_get("isinternal").map_err(storage)?,
            is_ingress: row.try_get("isingress").map_err(storage)?,
            is_encrypted: row.try_get("isencrypted").map_err(storage)?,
            enable_ipv6: row.try_get("enableipv6").map_err(storage)?,
            subnets: json(row.try_get("subnets").map_err(storage)?)?,
            service_names: json(row.try_get("servicenames").map_err(storage)?)?,
            labels: json(row.try_get("labels").map_err(storage)?)?,
            created_at: row.try_get("dockercreatedat").map_err(storage)?,
            observed_at: row.try_get("observedat").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            capabilities: None,
        })
    })
    .collect()
}

async fn list_swarm_secrets(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmSecretView>, AuthorizedReadError> {
    sqlx::query(
        r#"
SELECT * FROM swarmsecretprojections
WHERE platformid = $1 AND ($2::text IS NULL OR dockersecretid = $2)
ORDER BY name, dockersecretid
"#,
    )
    .bind(platform_id)
    .bind(id)
    .fetch_all(pool)
    .await
    .map_err(storage)?
    .into_iter()
    .map(|row| {
        let service_names: Vec<String> = json(row.try_get("servicenames").map_err(storage)?)?;
        Ok(SwarmSecretView {
            id: row.try_get("dockersecretid").map_err(storage)?,
            version_index: row.try_get("versionindex").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            driver: row.try_get("driver").map_err(storage)?,
            in_use: !service_names.is_empty(),
            service_names,
            labels: json(row.try_get("labels").map_err(storage)?)?,
            created_at: row.try_get("dockercreatedat").map_err(storage)?,
            updated_at: row.try_get("dockerupdatedat").map_err(storage)?,
            observed_at: row.try_get("observedat").map_err(storage)?,
            is_stale: row.try_get("isstale").map_err(storage)?,
            capabilities: None,
        })
    })
    .collect()
}

fn json<T: DeserializeOwned>(value: Value) -> Result<T, AuthorizedReadError> {
    serde_json::from_value(value).map_err(storage)
}

fn storage(error: impl std::fmt::Display) -> AuthorizedReadError {
    AuthorizedReadError::Storage(error.to_string())
}
