use citadel_activities::{ActivityEvent, ActivityEventInfo};
use citadel_platforms::{
    CreatePlatformInput, PlatformDetails, PlatformRegistrationError, RuntimePlatformInfo,
};
use citadel_primitives::ActorId;
use sqlx::{PgPool, Row};

pub async fn update(
    pool: &PgPool,
    current: &PlatformDetails,
    input: &CreatePlatformInput,
    info: Option<&RuntimePlatformInfo>,
    actor: ActorId,
    rename: bool,
) -> Result<(), PlatformRegistrationError> {
    let address = if input.connector_type == citadel_platforms::PlatformConnectorType::Local {
        if current.connector_type == citadel_platforms::ConnectorKind::Local {
            current.address.as_str()
        } else {
            citadel_platforms::LOCAL_DOCKER_ADDRESS
        }
    } else {
        input.address.as_deref().unwrap_or(&current.address)
    };
    let write = citadel_platforms::jobs::ProjectionWrite::begin(
        current.id,
        None,
        citadel_platforms::jobs::ProjectionKind::Platform,
    )
    .await;
    let mut tx = pool.begin().await.map_err(storage)?;
    sqlx::query("SET LOCAL lock_timeout = '5s'")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtext('citadel-platform-registration'))")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    let row = sqlx::query("SELECT name,address,description,connectortype,prunehistoricalswarmtaskcontainers,platformdescriptor,clusterid FROM platforms WHERE id=$1 FOR UPDATE")
        .bind(current.id).fetch_optional(&mut *tx).await.map_err(storage)?.ok_or_else(|| PlatformRegistrationError::Validation("Platform no longer exists.".into()))?;
    if row.get::<String, _>("name") != current.name
        || (!rename
            && (row.get::<String, _>("address") != current.address
                || row.get::<Option<String>, _>("description") != current.description
                || super::classification::connector_kind(row.get("connectortype"))
                    .map_err(storage)?
                    != current.connector_type
                || row.get::<Option<String>, _>("clusterid") != current.cluster_id
                || row.get::<bool, _>("prunehistoricalswarmtaskcontainers")
                    != current.prune_historical_swarm_task_containers))
    {
        return Err(PlatformRegistrationError::Conflict(
            "The Platform changed while this request was being validated. Reload and retry.".into(),
        ));
    }
    let conflict: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM platforms WHERE id<>$1 AND (lower(name)=lower($2) OR ($3::text IS NOT NULL AND platformdescriptor->>'daemonId'=$3) OR ($4::text IS NOT NULL AND clusterid=$4) OR address=$5))")
        .bind(current.id).bind(&input.name).bind(info.map(|v| v.daemon_id.trim())).bind(info.and_then(|v| v.swarm.as_ref()).and_then(|v| v.cluster_id.as_deref())).bind(address).fetch_one(&mut *tx).await.map_err(storage)?;
    if conflict {
        return Err(PlatformRegistrationError::Conflict(
            "A Platform with the same name or Docker identity already exists.".into(),
        ));
    }
    if rename {
        sqlx::query("UPDATE platforms SET name=$2 WHERE id=$1")
            .bind(current.id)
            .bind(&input.name)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let event = ActivityEvent::new_platform_event(
            current.id,
            input.name.clone(),
            actor,
            ActivityEventInfo::PlatformRenamed {
                old_name: current.name.clone(),
                new_name: input.name.clone(),
            },
            chrono::Utc::now(),
        )
        .map_err(|e| PlatformRegistrationError::Storage(e.to_string()))?;
        crate::persistence::postgres::activities::store::insert_activity(&mut tx, &event)
            .await
            .map_err(|e| PlatformRegistrationError::Storage(e.to_string()))?;
    } else {
        let mut descriptor: serde_json::Value = row.get("platformdescriptor");
        if let Some(info) = info {
            descriptor["daemonId"] = info.daemon_id.trim().into();
            descriptor["operatingSystem"] = info.operating_system.clone().into();
            descriptor["osType"] = info.os_type.clone().into();
            descriptor["architecture"] = info.architecture.clone().into();
            if let Some(swarm) = &info.swarm {
                descriptor["nodeID"] = swarm.node_id.clone().into();
                descriptor["controlAvailable"] = swarm.control_available.into();
                descriptor["clusterId"] = serde_json::json!(swarm.cluster_id);
            }
        }
        sqlx::query("UPDATE platforms SET name=$2,address=$3,description=$4,connectortype=$5,prunehistoricalswarmtaskcontainers=$6,platformdescriptor=$7,cpucount=$8,memtotal=$9,serverversion=$10,agentversion=$11,clusterid=$12 WHERE id=$1")
            .bind(current.id).bind(&input.name).bind(address).bind(&input.description).bind(input.connector_type.as_str()).bind(input.prune_historical_swarm_task_containers).bind(descriptor)
            .bind(info.map_or(current.cpu_count,|v| v.cpu_count)).bind(info.map_or(current.mem_total,|v| v.memory_total)).bind(info.map(|v| v.server_version.as_str()).or(current.server_version.as_deref())).bind(info.and_then(|v| v.agent_version.as_deref()).or(current.agent_version.as_deref())).bind(info.and_then(|v|v.swarm.as_ref()).and_then(|v|v.cluster_id.as_deref()).or(current.cluster_id.as_deref())).execute(&mut *tx).await.map_err(storage)?;
    }
    sqlx::query("SELECT pg_notify('citadel_platform_targets','')")
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
    if !rename && input.prune_historical_swarm_task_containers {
        sqlx::query("SELECT pg_notify('citadel_swarm_prune','')")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
    }
    tx.commit().await.map_err(storage)?;
    write.committed();
    Ok(())
}

fn storage(error: sqlx::Error) -> PlatformRegistrationError {
    PlatformRegistrationError::Storage(error.to_string())
}

pub struct PostgresPlatformManagementRepository(pub PgPool);
impl citadel_platforms::management::PlatformManagementRepository
    for PostgresPlatformManagementRepository
{
    fn update<'a>(
        &'a self,
        current: &'a PlatformDetails,
        input: &'a CreatePlatformInput,
        info: Option<&'a RuntimePlatformInfo>,
        actor: ActorId,
        rename: bool,
    ) -> futures_util::future::BoxFuture<'a, Result<(), PlatformRegistrationError>> {
        Box::pin(update(&self.0, current, input, info, actor, rename))
    }
}
