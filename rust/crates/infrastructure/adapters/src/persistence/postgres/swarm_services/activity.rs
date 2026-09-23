use super::*;

pub(super) fn service_creation_activity(
    service: SwarmServiceActivitySnapshot,
    source: Option<citadel_activities::ActivitySourceResource>,
) -> ActivityEventInfo {
    match source {
        Some(source) => ActivityEventInfo::SwarmServiceDuplicated { service, source },
        None => ActivityEventInfo::swarm_service_created(service),
    }
}
pub(crate) fn activity_snapshot(
    row: &PgRow,
) -> Result<SwarmServiceActivitySnapshot, SwarmServiceError> {
    Ok(SwarmServiceActivitySnapshot {
        id: row.try_get("id").map_err(storage)?,
        platform_id: row.try_get("platformid").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        docker_name: row.try_get("dockername").map_err(storage)?,
        docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
        spec: row.try_get("spec").map_err(storage)?,
    })
}
pub(crate) async fn insert_swarm_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    platform_id: Uuid,
    actor_id: ActorId,
    info: ActivityEventInfo,
    status: ActivityStatus,
) -> Result<(), SwarmServiceError> {
    let activity = ActivityEvent::new_swarm_service_event(
        id,
        name.to_owned(),
        platform_id,
        actor_id,
        info,
        status,
        Utc::now(),
    )
    .map_err(storage)?;
    insert_activity(tx, &activity).await.map_err(storage)
}
