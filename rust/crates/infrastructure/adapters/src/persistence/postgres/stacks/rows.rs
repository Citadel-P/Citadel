use super::*;

pub(super) fn map_stack(row: PgRow) -> Result<StackDetails, StackError> {
    let descriptor: Value = row.try_get("platformdescriptor").map_err(storage)?;
    let level: i32 = row.try_get("permission_level").map_err(storage)?;
    let specific: i32 = row.try_get("permission_specific").map_err(storage)?;
    Ok(StackDetails {
        stack: citadel_stacks::Stack {
            id: row.try_get("id").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            description: row.try_get("description").map_err(storage)?,
            stack_source: parse_stack_source(row.try_get("stacksource").map_err(storage)?)?,
            stack_update_state: StackUpdateState::from_storage_value(
                row.try_get("stackupdatestate").map_err(storage)?,
            )?,
            drift_policy: StackDriftPolicy::from_storage_value(
                row.try_get("driftpolicy").map_err(storage)?,
            )?,
            created_at: row.try_get("createdat").map_err(storage)?,
            created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
            control_state: row.try_get("controlstate").map_err(storage)?,
            current_stack_release_id: row.try_get("currentstackreleaseid").map_err(storage)?,
            row_version: row.try_get("rowversion").map_err(storage)?,
        },
        status: StackReleaseStatus::parse(row.try_get("release_status").map_err(storage)?)?,
        platform_type: crate::persistence::postgres::platforms::classification::platform_kind(
            descriptor
                .get("$type")
                .or_else(|| descriptor.get("type"))
                .and_then(Value::as_str)
                .unwrap_or("Docker"),
        )
        .map_err(storage)?,
        platform_id: Some(row.try_get("platformid").map_err(storage)?),
        version: Some(row.try_get("version").map_err(storage)?),
        spec: Some(StackSpec::from_storage_value(
            row.try_get("spec").map_err(storage)?,
        )?),
        source: row
            .try_get::<Option<Value>, _>("source")
            .map_err(storage)?
            .map(StackReleaseSource::from_storage_value)
            .transpose()?,
        resource_bindings: row
            .try_get::<Option<Value>, _>("resourcebindings")
            .map_err(storage)?
            .map(ResourceBindingSnapshot::list_from_storage_value)
            .transpose()?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        platform_name: Some(row.try_get("platform_name").map_err(storage)?),
        tags: decode_tags(row.try_get("tags").map_err(storage)?)?,
        latest_activity: row.try_get("latest_activity").map_err(storage)?,
        effective_permission: decode_permission(
            row.try_get("permission_administrator").map_err(storage)?,
            level,
            specific,
        )?,
    })
}

pub(super) fn map_release(row: PgRow) -> Result<StackReleaseDetails, StackError> {
    Ok(StackReleaseDetails {
        release: citadel_stacks::StackRelease {
            id: row.try_get("id").map_err(storage)?,
            stack_id: row.try_get("stackid").map_err(storage)?,
            platform_id: row.try_get("platformid").map_err(storage)?,
            status: StackReleaseStatus::parse(row.try_get("status").map_err(storage)?)?,
            version: row.try_get("version").map_err(storage)?,
            spec: StackSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?,
            source: row
                .try_get::<Option<Value>, _>("source")
                .map_err(storage)?
                .map(StackReleaseSource::from_storage_value)
                .transpose()?,
            resource_bindings: row
                .try_get::<Option<Value>, _>("resourcebindings")
                .map_err(storage)?
                .map(ResourceBindingSnapshot::list_from_storage_value)
                .transpose()?,
            created_at: row.try_get("createdat").map_err(storage)?,
            created_by_actor_id: row.try_get("createdbyactorid").map_err(storage)?,
        },
        actor_name: row.try_get("actor_name").map_err(storage)?,
        actor_type: row.try_get("actor_type").map_err(storage)?,
        platform_status: row.try_get("platform_status").map_err(storage)?,
        platform_name: Some(row.try_get("platform_name").map_err(storage)?),
    })
}
pub(super) fn ensure_idle(row: &PgRow) -> Result<(), StackError> {
    if row
        .try_get::<Option<String>, _>("controlstate")
        .map_err(storage)?
        .as_deref()
        != Some("Idle")
    {
        return Err(StackError::Conflict(
            "The Stack has an operation in progress.".to_owned(),
        ));
    }
    Ok(())
}
pub(super) fn parse_stack_source(value: &str) -> Result<StackSource, StackError> {
    match value {
        "WebEditor" => Ok(StackSource::WebEditor),
        "Git" => Ok(StackSource::Git),
        _ => Err(StackError::Storage(format!(
            "invalid Stack source '{value}'"
        ))),
    }
}
pub(super) fn storage(error: impl std::fmt::Display) -> StackError {
    StackError::Storage(error.to_string())
}
pub(super) fn database_error(error: sqlx::Error) -> StackError {
    match &error {
        sqlx::Error::Database(db) if db.is_unique_violation() => {
            StackError::Conflict("A Stack with the same identity already exists.".to_owned())
        }
        sqlx::Error::Database(db) if db.is_foreign_key_violation() => {
            StackError::Validation("A referenced Stack resource does not exist.".to_owned())
        }
        _ => storage(error),
    }
}

pub(super) fn decode_permission(
    administrator: bool,
    level: i32,
    specific: i32,
) -> Result<EffectivePermission, StackError> {
    if administrator {
        return Ok(EffectivePermission::Administrator);
    }
    Ok(EffectivePermission::Granted {
        level: PermissionLevel::from_i32(level).ok_or_else(|| {
            StackError::Storage(format!("unknown persisted PermissionLevel value {level}"))
        })?,
        specifics: SpecificPermissions::from_bits_retain(specific as u32),
    })
}
fn decode_tags(value: Value) -> Result<Vec<TagSummary>, StackError> {
    serde_json::from_value(value).map_err(storage)
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
            resource_type: citadel_primitives::ResourceType::Stack,
            level: PermissionLevel::Write,
            specific: None
        }));
        assert!(!known.allows(PermissionRequirement {
            resource_type: citadel_primitives::ResourceType::Stack,
            level: PermissionLevel::Read,
            specific: Some(citadel_primitives::SpecificPermission::Apply)
        }));
    }
}
