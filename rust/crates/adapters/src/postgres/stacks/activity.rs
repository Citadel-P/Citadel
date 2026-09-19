use super::*;

pub(super) fn duplicate_activity_info(
    input: &CreateStack,
    snapshot: StackActivitySnapshot,
) -> Result<ActivityEventInfo, StackError> {
    let Some(source) = input.duplicate_source.as_ref() else {
        return Ok(ActivityEventInfo::StackCreated { stack: snapshot });
    };
    let resource_id = source.id;
    let resource_name = source.name.clone();
    Ok(ActivityEventInfo::StackDuplicated {
        stack: snapshot,
        source: ActivitySourceResource {
            resource_type: ActivityResourceType::Stack,
            resource_id,
            resource_name,
        },
    })
}
pub(super) async fn insert_stack_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    platform: Uuid,
    actor: ActorId,
    info: ActivityEventInfo,
    status: ActivityStatus,
) -> Result<(), StackError> {
    let event = ActivityEvent::new_stack_event(
        id,
        name.to_owned(),
        platform,
        actor,
        info,
        status,
        Utc::now(),
    )
    .map_err(|e| StackError::Storage(e.to_string()))?;
    insert_activity(tx, &event)
        .await
        .map_err(|e| StackError::Storage(e.to_string()))
}
