use super::*;

pub(super) fn map_channel(row: sqlx::postgres::PgRow) -> Result<AlertChannel, AlertError> {
    Ok(AlertChannel {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        alert_destination: row.try_get("alertdestination").map_err(storage)?,
        url: row.try_get("url").map_err(storage)?,
        is_active: row.try_get("isactive").map_err(storage)?,
        audit: citadel_primitives::AuditMetadata {
            created_by_actor_id: citadel_primitives::ActorId::new(
                row.try_get("createdbyactorid").map_err(storage)?,
            ),
            created_at: row.try_get("createdat").map_err(storage)?,
        },
    })
}

pub(super) fn map_rule(row: sqlx::postgres::PgRow) -> Result<AlertRule, AlertError> {
    let limited: Value = row.try_get("limitedto").map_err(storage)?;
    let quiet: Value = row.try_get("quiethours").map_err(storage)?;
    Ok(AlertRule {
        id: row.try_get("id").map_err(storage)?,
        name: row.try_get("name").map_err(storage)?,
        description: row.try_get("description").map_err(storage)?,
        alert_type: row.try_get("type").map_err(storage)?,
        severity: row
            .try_get::<String, _>("severity")
            .map_err(storage)?
            .parse()
            .map_err(AlertError::Storage)?,
        cooldown_seconds: row.try_get("cooldownseconds").map_err(storage)?,
        required_matches: row.try_get("requiredmatches").map_err(storage)?,
        threshold: row.try_get("threshold").map_err(storage)?,
        status: row
            .try_get::<String, _>("status")
            .map_err(storage)?
            .parse()
            .map_err(AlertError::Storage)?,
        channel_ids: row.try_get("channelids").map_err(storage)?,
        limited_to: limited.as_array().cloned().unwrap_or_default(),
        quiet_hours: quiet.as_array().cloned().unwrap_or_default(),
        audit: citadel_primitives::AuditMetadata {
            created_by_actor_id: citadel_primitives::ActorId::new(
                row.try_get("createdbyactorid").map_err(storage)?,
            ),
            created_at: row.try_get("createdat").map_err(storage)?,
        },
    })
}

pub(super) fn map_event(row: sqlx::postgres::PgRow) -> Result<AlertEvent, AlertError> {
    let acknowledged_at: Option<DateTime<Utc>> = row.try_get("acknowledgedat").map_err(storage)?;
    let resolved_at: Option<DateTime<Utc>> = row.try_get("resolvedat").map_err(storage)?;
    let info: Value = row.try_get("info").map_err(storage)?;
    let message = info
        .get("HumanMessage")
        .or_else(|| info.get("humanMessage"))
        .and_then(Value::as_str)
        .unwrap_or("Alert condition matched.")
        .to_owned();
    Ok(AlertEvent {
        id: row.try_get("id").map_err(storage)?,
        alert_rule_id: row.try_get("alertruleid").map_err(storage)?,
        alert_type: row.try_get("type").map_err(storage)?,
        severity: row
            .try_get::<String, _>("severity")
            .map_err(storage)?
            .parse()
            .map_err(AlertError::Storage)?,
        status: citadel_alerts::AlertEventStatus::from_lifecycle(
            acknowledged_at.is_some(),
            resolved_at.is_some(),
        ),
        message,
        info,
        resource_id: row.try_get("resourceid").map_err(storage)?,
        resource_name: row.try_get("resourcename").map_err(storage)?,
        resource_type: row.try_get("resourcetype").map_err(storage)?,
        acknowledged_by_actor_id: row.try_get("acknowledgedbyactorid").map_err(storage)?,
        acknowledged_at,
        resolved_by_actor_id: row.try_get("resolvedbyactorid").map_err(storage)?,
        resolved_at,
        actor_id: row.try_get("actor_id").map_err(storage)?,
        actor_name: row.try_get("actor_name").map_err(storage)?,
        actor_type: row.try_get("actor_type").map_err(storage)?,
        resolution_note: row.try_get("resolutionnote").map_err(storage)?,
        created_at: row.try_get("createdat").map_err(storage)?,
        updated_at: row.try_get("updatedat").map_err(storage)?,
    })
}

pub(super) fn storage(error: sqlx::Error) -> AlertError {
    match &error {
        sqlx::Error::Database(database) if database.code().as_deref() == Some("23505") => {
            AlertError::Conflict("An Alert resource with the same value already exists.".into())
        }
        sqlx::Error::Database(database) if database.code().as_deref() == Some("23503") => {
            AlertError::Validation("An Alert resource reference does not exist.".into())
        }
        _ => AlertError::Storage(error.to_string()),
    }
}
