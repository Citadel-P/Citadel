use super::*;

pub(super) const PLATFORM_RESOURCE_TYPE: i32 = 0;

pub(super) const READ_PERMISSION_MASK: i32 = 1 | 2 | 4;

pub(super) const PLATFORM_SELECT: &str = r#"
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

pub(super) const CONTAINER_SELECT: &str = r#"
SELECT container.id, container.platformid, container.dockercontainerid, container.name,
       container.dockerimageid, container.created, container.state,
       COALESCE(container.controlstate, 'Idle') AS controlstate, container.updated,
       container.stack, container.issystem, container.systemrole,
       container.hascitadelownershiplabels, container.isswarmtask, container.dockernodeid,
       node.hostname AS nodehostname, container.projectionobservedat,
       container.projectionstalesince, container.projectionstalereason, container.ports,
       container.deploymentid, container.stackid,
       image.id AS image_id, image.tags AS image_tags, image.name AS image_name,
       image.dockerimageid AS image_dockerimageid, image.size AS image_size,
       image.containers AS image_containers, image.createdat AS image_createdat,
       image.updatedat AS image_updatedat, image.registryid AS image_registryid,
       deployment.name AS deployment_name, deployment.status AS deployment_status,
       stat.containerid AS stat_containerid, stat.memoryactive AS stat_memoryactive,
       stat.memorycache AS stat_memorycache, stat.cpuusage AS stat_cpuusage,
       stat.memorylimit AS stat_memorylimit, stat.rxbytes AS stat_rxbytes,
       stat.txbytes AS stat_txbytes, stat.created AS stat_created
FROM containers container
LEFT JOIN images image ON image.id=container.imageid AND image.platformid=container.platformid
LEFT JOIN deployments deployment ON deployment.id=container.deploymentid AND deployment.platformid=container.platformid
LEFT JOIN swarmnodeprojections node
  ON node.platformid = container.platformid AND node.dockernodeid = container.dockernodeid
LEFT JOIN LATERAL (
    SELECT * FROM containerstats WHERE containerid = container.id ORDER BY created DESC LIMIT 1
) stat ON TRUE
"#;

pub(super) async fn list_swarm_nodes(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmNodeSummary>, AuthorizedReadError> {
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
        Ok(SwarmNodeSummary {
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
        })
    })
    .collect()
}

pub(super) async fn list_swarm_services(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmServiceSummary>, AuthorizedReadError> {
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
        Ok(SwarmServiceSummary {
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
        })
    })
    .collect()
}

pub(super) async fn list_swarm_tasks(
    pool: &PgPool,
    platform_id: Uuid,
    task_id: Option<&str>,
    service_id: Option<&str>,
    limit: Option<usize>,
) -> Result<Vec<SwarmTaskSummary>, AuthorizedReadError> {
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
        Ok(SwarmTaskSummary {
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
        })
    })
    .collect()
}

pub(super) async fn list_swarm_configs(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmConfigSummary>, AuthorizedReadError> {
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
        Ok(SwarmConfigSummary {
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
        })
    })
    .collect()
}

pub(super) async fn list_swarm_networks(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmNetworkSummary>, AuthorizedReadError> {
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
        Ok(SwarmNetworkSummary {
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
        })
    })
    .collect()
}

pub(super) async fn list_swarm_secrets(
    pool: &PgPool,
    platform_id: Uuid,
    id: Option<&str>,
) -> Result<Vec<SwarmSecretSummary>, AuthorizedReadError> {
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
        Ok(SwarmSecretSummary {
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
        })
    })
    .collect()
}

pub(super) fn storage(error: impl std::fmt::Display) -> AuthorizedReadError {
    AuthorizedReadError::Storage(error.to_string())
}
