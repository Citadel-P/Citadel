use super::*;
use citadel_primitives::AuthorizedResource;
pub(super) fn map_deployment(
    row: PgRow,
) -> Result<AuthorizedResource<citadel_deployments::Deployment>, DeploymentError> {
    let spec = DeploymentSpec::from_storage_value(row.try_get("spec").map_err(storage)?)?;
    let tags = serde_json::from_value::<Vec<TagSummary>>(row.try_get("tags").map_err(storage)?)
        .map_err(|error| DeploymentError::Storage(format!("invalid Deployment Tags: {error}")))?;
    let permission = decode_permission(
        row.try_get("permission_administrator").map_err(storage)?,
        row.try_get("permission_level").map_err(storage)?,
        row.try_get("permission_specific").map_err(storage)?,
    )?;
    let auto_status = row
        .try_get::<Option<String>, _>("autoupdatestate_status")
        .map_err(storage)?
        .map(|status| status.parse::<citadel_primitives::AutoUpdateStatus>())
        .transpose()
        .map_err(storage)?;
    let auto_last_checked: Option<DateTime<Utc>> = row
        .try_get("autoupdatestate_lastcheckedat")
        .map_err(storage)?;
    let auto_current_digest: Option<String> = row
        .try_get("autoupdatestate_currentdigest")
        .map_err(storage)?;
    let auto_remote_digest: Option<String> = row
        .try_get("autoupdatestate_remotedigest")
        .map_err(storage)?;
    let auto_last_error: Option<String> =
        row.try_get("autoupdatestate_lasterror").map_err(storage)?;
    Ok(AuthorizedResource {
        resource: Deployment {
            id: row.try_get("id").map_err(storage)?,
            name: row.try_get("name").map_err(storage)?,
            description: row.try_get("description").map_err(storage)?,
            platform_id: row.try_get("platformid").map_err(storage)?,

            status: row
                .try_get::<&str, _>("status")
                .map_err(storage)?
                .parse()
                .map_err(storage)?,
            control_state: row
                .try_get::<&str, _>("controlstate")
                .map_err(storage)?
                .parse()
                .map_err(storage)?,
            row_version: row.try_get("rowversion").map_err(storage)?,
            auto_update_state: auto_status.map(|status| AutoUpdateState {
                last_checked_at: auto_last_checked.unwrap_or_else(dotnet_min_datetime),
                status,
                current_digest: auto_current_digest,
                remote_digest: auto_remote_digest,
                last_error: auto_last_error,
            }),
            spec,

            audit: citadel_primitives::AuditMetadata {
                created_at: row.try_get("createdat").map_err(storage)?,
                created_by_actor_id: citadel_primitives::ActorId::new(
                    row.try_get("createdbyactorid").map_err(storage)?,
                ),
            },

            platform_status: row
                .try_get::<String, _>("platform_status")
                .map_err(storage)?
                .parse()
                .map_err(storage)?,
            platform_name: row.try_get("platform_name").map_err(storage)?,
            image_name: row.try_get("image_name").map_err(storage)?,
            image_id: row.try_get("image_id").map_err(storage)?,
            container_id: row.try_get("container_id").map_err(storage)?,
            docker_container_id: row.try_get("dockercontainerid").map_err(storage)?,
            docker_image_id: row.try_get("dockerimageid").map_err(storage)?,
            tags,
            latest_activity: row
                .try_get::<Option<Value>, _>("latest_activity")
                .map_err(storage)?
                .map(serde_json::from_value)
                .transpose()
                .map_err(storage)?,
        },
        effective_permission: permission,
    })
}

pub(super) fn decode_permission(
    administrator: bool,
    level: i32,
    specifics: i32,
) -> Result<EffectivePermission, DeploymentError> {
    if administrator {
        return Ok(EffectivePermission::Administrator);
    }
    Ok(EffectivePermission::Granted {
        level: PermissionLevel::from_i32(level).ok_or_else(|| {
            DeploymentError::Storage(format!("unknown persisted PermissionLevel value {level}"))
        })?,
        specifics: SpecificPermissions::from_bits_retain(specifics as u32),
    })
}

pub(super) fn platform_kind(descriptor: &Value) -> &str {
    descriptor
        .get("$type")
        .or_else(|| descriptor.get("type"))
        .and_then(Value::as_str)
        .unwrap_or_default()
}

pub(super) fn dotnet_min_datetime() -> DateTime<Utc> {
    DateTime::from_timestamp(-62_135_596_800, 0)
        .expect("the .NET DateTime minimum is representable by chrono")
}

pub(super) fn storage(error: impl std::fmt::Display) -> DeploymentError {
    DeploymentError::Storage(error.to_string())
}

pub(super) fn ensure_idle(row: &PgRow) -> Result<(), DeploymentError> {
    let state: String = row.try_get("controlstate").map_err(storage)?;
    if state == "Idle" {
        Ok(())
    } else {
        Err(DeploymentError::Conflict(
            "The Deployment has an operation in progress.".to_owned(),
        ))
    }
}

pub(super) fn database_error(error: sqlx::Error) -> DeploymentError {
    if let sqlx::Error::Database(database) = &error
        && database.code().as_deref() == Some("23505")
    {
        return DeploymentError::Conflict("Name already exists.".to_owned());
    }
    storage(error)
}
