use super::*;

pub(super) fn map_platform(row: PgRow) -> Result<PlatformDetails, AuthorizedReadError> {
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
            Ok(PlatformStatSnapshot {
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
    Ok(PlatformDetails {
        id: row.try_get("id").map_err(storage)?,
        tags: json(row.try_get("resource_tags").map_err(storage)?)?,
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
    })
}

pub(super) fn counts(
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

pub(super) fn map_container(row: PgRow) -> Result<ContainerDetails, AuthorizedReadError> {
    let platform_id = row.try_get("platformid").map_err(storage)?;
    let image_view = row
        .try_get::<Option<Uuid>, _>("image_id")
        .map_err(storage)?
        .map(|id| {
            Ok::<_, AuthorizedReadError>(ImageDetails {
                id,
                platform_id,
                tags: json(row.try_get("image_tags").map_err(storage)?)?,
                name: row.try_get("image_name").map_err(storage)?,
                docker_image_id: row.try_get("image_dockerimageid").map_err(storage)?,
                size: row.try_get("image_size").map_err(storage)?,
                is_in_use: row.try_get::<i32, _>("image_containers").map_err(storage)? > 0,
                created_at: row.try_get("image_createdat").map_err(storage)?,
                updated_at: row.try_get("image_updatedat").map_err(storage)?,
                registry_id: row.try_get("image_registryid").map_err(storage)?,
                control_state: "Idle".into(),
                repo_digests: None,
                content_identity: None,
                docker_node_id: None,
                node_hostname: None,
                is_stale: false,
                stale_reason: None,
            })
        })
        .transpose()?;
    let deployment_view = row
        .try_get::<Option<String>, _>("deployment_name")
        .map_err(storage)?
        .map(|name| {
            // Only identity/name/status are
            // joined. It must not expose Deployment configuration to Platform readers.
            let minimum = chrono::DateTime::from_timestamp(-62_135_596_800, 0)
                .expect("valid .NET minimum timestamp");
            Ok::<_, AuthorizedReadError>(ContainerDeploymentSummary {
                id: row.try_get("deploymentid").map_err(storage)?,
                name,
                platform_id,
                status: row.try_get("deployment_status").map_err(storage)?,
                created_at: minimum,
                created_by_actor_id: Uuid::nil(),
                control_state: "Idle".into(),
                platform_status: "Offline".into(),
                auto_update_state: ContainerDeploymentUpdateState {
                    last_checked_at: minimum,
                    status: "Unknown".into(),
                },
            })
        })
        .transpose()?;
    let last_stats = row
        .try_get::<Option<Uuid>, _>("stat_containerid")
        .map_err(storage)?
        .map(|container_id| {
            Ok(ContainerStatSnapshot {
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
    Ok(ContainerDetails {
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
        ports: crate::connectors::containers::ports::normalize(
            row.try_get("ports").map_err(storage)?,
        ),
        deployment_id: row.try_get("deploymentid").map_err(storage)?,
        stack_id: row.try_get("stackid").map_err(storage)?,
        image_view,
        deployment_view,
    })
}

pub(super) fn map_node_resource<T: DeserializeOwned>(
    row: PgRow,
) -> Result<NodeResourceProjection<T>, AuthorizedReadError> {
    Ok(NodeResourceProjection {
        resource: json(row.try_get("resource").map_err(storage)?)?,
        docker_node_id: row.try_get("dockernodeid").map_err(storage)?,
        node_hostname: row.try_get("hostname").map_err(storage)?,
        is_stale: row.try_get("isstale").map_err(storage)?,
    })
}

pub(super) fn map_image(row: PgRow) -> Result<ImageDetails, AuthorizedReadError> {
    Ok(ImageDetails {
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
    })
}

pub(super) fn json<T: DeserializeOwned>(value: Value) -> Result<T, AuthorizedReadError> {
    serde_json::from_value(value).map_err(storage)
}
