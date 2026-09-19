use super::*;
use citadel_swarm_services::TagSummary;

pub(super) fn map_service(row: PgRow) -> Result<SwarmServiceDetails, SwarmServiceError> {
    let spec = SwarmServiceSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
    let desired: String = row.try_get("desiredspechash").map_err(storage)?;
    let applied: Option<String> = row.try_get("lastapplieddesiredspechash").map_err(storage)?;
    let live: Option<String> = row.try_get("liveruntimehash").map_err(storage)?;
    let applied_runtime: Option<String> = row.try_get("lastappliedruntimehash").map_err(storage)?;
    let level: i32 = row.try_get("permission_level").map_err(storage)?;
    let specific: i32 = row.try_get("permission_specific").map_err(storage)?;
    let operation_id: Option<Uuid> = row.try_get("operationid").map_err(storage)?;
    let current_operation = operation_id
        .map(|id| {
            Ok(SwarmServiceOperation {
                id,
                kind: row.try_get("operationkind").map_err(storage)?,
                state: row.try_get("operationstate").map_err(storage)?,
                prepared_at: row.try_get("preparedat").map_err(storage)?,
                attempted_at: row.try_get("attemptedat").map_err(storage)?,
                completed_at: row.try_get("completedat").map_err(storage)?,
                result_code: row.try_get("resultcode").map_err(storage)?,
                warnings: row
                    .try_get::<Option<Value>, _>("warnings")
                    .map_err(storage)?
                    .map(json)
                    .transpose()?
                    .unwrap_or_default(),
                result_message: row.try_get("resultmessage").map_err(storage)?,
            })
        })
        .transpose()?;
    let last_checked = match row.try_get::<Option<DateTime<Utc>>, _>("effective_last_checked_at") {
        Ok(Some(value)) => value,
        _ => dotnet_min_datetime(),
    };
    Ok(SwarmServiceDetails {
        service: citadel_swarm_services::SwarmService {
            id: row.try_get("id").map_err(storage)?,
            platform_id: row.try_get("platformid").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            description: row.try_get("description").map_err(storage)?,
            docker_name: row.try_get("dockername").map_err(storage)?,
            docker_service_id: row.try_get("dockerserviceid").map_err(storage)?,
            spec,
            health: row.try_get("effective_health").map_err(storage)?,
            synchronization_state: row
                .try_get("effective_synchronization_state")
                .map_err(storage)?,
            control_state: row.try_get("controlstate").map_err(storage)?,
            auto_update_state: AutoUpdateState {
                last_checked_at: last_checked,
                status: row.try_get("autoupdatestate_status").map_err(storage)?,
                current_digest: row
                    .try_get("autoupdatestate_currentdigest")
                    .map_err(storage)?,
                remote_digest: row
                    .try_get("autoupdatestate_remotedigest")
                    .map_err(storage)?,
                last_error: row.try_get("autoupdatestate_lasterror").map_err(storage)?,
            },
            applied_image_digest: row.try_get("appliedimagedigest").map_err(storage)?,
            has_pending_desired_changes: applied.as_deref() != Some(desired.as_str()),
            has_runtime_drift: matches!((&applied_runtime,&live),(Some(a),Some(b)) if a!=b),
            row_version: row.try_get("rowversion").map_err(storage)?,
            created_at: row.try_get("createdat").map_err(storage)?,
            updated_at: row.try_get("updatedat").map_err(storage)?,
        },
        platform_name: row.try_get("platform_name").map_err(storage)?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        running_task_count: row.try_get("runningtaskcount").map_err(storage)?,
        desired_task_count: row.try_get("desiredtaskcount").map_err(storage)?,
        update_state: row.try_get("projection_update_state").map_err(storage)?,
        update_message: row.try_get("updatemessage").map_err(storage)?,
        current_operation,
        tags: decode_tags(row.try_get("tags").map_err(storage)?)?,
        tasks: Some(json(row.try_get("tasks").map_err(storage)?)?),
        effective_permission: decode_permission(
            row.try_get("permission_administrator").map_err(storage)?,
            level,
            specific,
        )?,
    })
}

pub(super) fn ensure_idle(row: &PgRow) -> Result<(), SwarmServiceError> {
    if row.try_get::<String, _>("controlstate").map_err(storage)? == "Idle" {
        Ok(())
    } else {
        Err(SwarmServiceError::Conflict(
            "The Service already has an operation in progress.".to_owned(),
        ))
    }
}
pub(super) fn ensure_swarm_manager(row: &PgRow) -> Result<(), SwarmServiceError> {
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    if descriptor.get("$type").and_then(Value::as_str) != Some("DockerSwarm") {
        return Err(SwarmServiceError::Validation(
            "Managed Services require a Docker Swarm platform.".to_owned(),
        ));
    }
    if row
        .try_get::<String, _>("platform_status")
        .map_err(storage)?
        != "Online"
        || descriptor.get("controlAvailable").and_then(Value::as_bool) == Some(false)
    {
        return Err(SwarmServiceError::Conflict(
            "The Docker Swarm manager is not available.".to_owned(),
        ));
    }
    Ok(())
}
pub(super) fn docker_name(name: &str, id: Uuid) -> String {
    let mut out = String::with_capacity(name.len().min(48) + 9);
    let mut separator = false;
    for ch in name.trim().to_ascii_lowercase().chars() {
        let normalized = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
            ch
        } else {
            '-'
        };
        if normalized == '-' && separator {
            continue;
        }
        out.push(normalized);
        separator = normalized == '-';
        if out.len() >= 48 {
            break;
        }
    }
    let base = out.trim_matches(['-', '_']);
    let base = if base.is_empty() { "service" } else { base };
    format!("{base}-{}", &id.simple().to_string()[..8])
}
pub(super) fn json<T: serde::de::DeserializeOwned>(value: Value) -> Result<T, SwarmServiceError> {
    serde_json::from_value(value).map_err(storage)
}
pub(super) fn dotnet_min_datetime() -> DateTime<Utc> {
    DateTime::from_timestamp(-62_135_596_800, 0).expect(".NET minimum date is representable")
}
pub(super) fn database_error(error: sqlx::Error) -> SwarmServiceError {
    if let sqlx::Error::Database(db) = &error
        && db.code().as_deref() == Some("23505")
    {
        return SwarmServiceError::Conflict(
            "A Service with the same name already exists on this Platform.".to_owned(),
        );
    }
    storage(error)
}
pub(super) fn storage(error: impl std::fmt::Display) -> SwarmServiceError {
    SwarmServiceError::Storage(error.to_string())
}

pub(super) fn decode_permission(
    administrator: bool,
    level: i32,
    specific: i32,
) -> Result<EffectivePermission, SwarmServiceError> {
    if administrator {
        return Ok(EffectivePermission::Administrator);
    }
    Ok(EffectivePermission::Granted {
        level: PermissionLevel::from_i32(level).ok_or_else(|| {
            SwarmServiceError::Storage(format!("unknown persisted PermissionLevel value {level}"))
        })?,
        specifics: SpecificPermissions::from_bits_retain(specific as u32),
    })
}
fn decode_tags(value: Value) -> Result<Vec<TagSummary>, SwarmServiceError> {
    #[derive(serde::Deserialize)]
    struct Tag {
        id: Uuid,
        name: String,
        color: String,
    }
    let tags: Vec<Tag> = serde_json::from_value(value).map_err(storage)?;
    Ok(tags
        .into_iter()
        .map(|tag| TagSummary {
            id: tag.id,
            name: tag.name,
            color: tag.color,
        })
        .collect())
}

#[cfg(test)]
mod permission_tests {
    use super::*;

    #[test]
    fn persisted_hierarchy_is_checked_and_admin_is_explicit() {
        for invalid in [3, 7, -1, 8] {
            assert!(decode_permission(false, invalid, 0).is_err());
        }
        assert_eq!(
            decode_permission(true, 0, 0).unwrap(),
            EffectivePermission::Administrator
        );
        let known = decode_permission(false, 4, 1 << 30).unwrap();
        assert!(known.allows(PermissionRequirement {
            resource_type: citadel_domain::ResourceType::SwarmService,
            level: PermissionLevel::Write,
            specific: None
        }));
        assert!(!known.allows(PermissionRequirement {
            resource_type: citadel_domain::ResourceType::SwarmService,
            level: PermissionLevel::Read,
            specific: Some(citadel_domain::SpecificPermission::Apply)
        }));
    }
}
