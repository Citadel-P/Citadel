use super::*;

pub(super) async fn write_rule_activity(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    name: &str,
    actor: ActorId,
    info: citadel_activities::ActivityEventInfo,
) -> Result<(), AlertError> {
    let activity = citadel_activities::ActivityEvent::new_alert_rule_event(
        id,
        name.to_owned(),
        actor,
        info,
        Utc::now(),
    )
    .map_err(|error| AlertError::Storage(error.to_string()))?;
    crate::persistence::postgres::activities::store::insert_activity(tx, &activity)
        .await
        .map_err(|error| AlertError::Storage(error.to_string()))
}
