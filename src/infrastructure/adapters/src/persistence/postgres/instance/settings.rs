use super::repository::{PostgresInstanceRepository, storage};
use crate::persistence::postgres::activities::store::{insert_activity, invalid_activity};
use chrono::{DateTime, Utc};
use citadel_activities::{ActivityEvent, ActivityEventInfo};
use citadel_identity::ActorPrincipal;
use citadel_identity::MfaPolicy;
use citadel_instance::{InstallationSettings, InstanceError, PatchInstallationSettings};
use citadel_primitives::PatchField;
use sqlx::{PgPool, Row};

pub(super) async fn load(pool: &PgPool) -> Result<InstallationSettings, InstanceError> {
    let row = sqlx::query("SELECT initializedat IS NOT NULL AS initialized, updatechecksenabled, updatecheckintervalhours, mfapolicy, instanceconfiguration FROM instancesetupstates WHERE id = 1")
        .fetch_one(pool).await.map_err(storage)?;
    map(&row)
}

pub(super) async fn patch(
    store: &PostgresInstanceRepository,
    principal: &ActorPrincipal,
    patch: &PatchInstallationSettings,
    now: DateTime<Utc>,
) -> Result<InstallationSettings, InstanceError> {
    let _fence = store.authorization.read_fence().await;
    let mut tx = store.pool.begin().await.map_err(storage)?;
    super::authorization::authorize(&mut tx, principal).await?;
    if sqlx::query_scalar::<_, bool>("SELECT EXISTS(SELECT 1 FROM instancerestarts)")
        .fetch_one(&mut *tx)
        .await
        .map_err(storage)?
    {
        return Err(InstanceError::Conflict(
            "Core is restarting. Wait for it to reconnect before changing settings.".into(),
        ));
    }
    let row = sqlx::query("SELECT initializedat IS NOT NULL AS initialized, updatechecksenabled, updatecheckintervalhours, mfapolicy, instanceconfiguration FROM instancesetupstates WHERE id = 1 FOR UPDATE")
        .fetch_one(&mut *tx).await.map_err(storage)?;
    let old = map(&row)?;
    if !old.initialized {
        return Err(InstanceError::Conflict(
            "Complete Citadel setup first.".into(),
        ));
    }
    let configuration = match &patch.configuration {
        PatchField::Value(value) => value.apply(&old.configuration, &patch.deployment_overrides)?,
        PatchField::Missing => old.configuration.clone(),
        PatchField::Null => {
            return Err(InstanceError::Validation(
                "configuration cannot be null.".into(),
            ));
        }
    };
    configuration
        .effective(&patch.deployment_overrides)
        .validate_deployment(patch.deployment_core_agent_requires_http)?;
    let next = InstallationSettings {
        configuration,
        initialized: true,
        update_checks_enabled: match patch.update_checks_enabled {
            PatchField::Value(value) => value,
            _ => old.update_checks_enabled,
        },
        update_check_interval_hours: match patch.update_check_interval_hours {
            PatchField::Value(value) => value,
            _ => old.update_check_interval_hours,
        },
        mfa_policy: match patch.mfa_policy {
            PatchField::Value(value) => value,
            _ => old.mfa_policy,
        },
    };
    if next != old {
        // Store only typed administrator choices. An unrelated MFA/Updates edit
        // must not pin today's built-in defaults as choices for every control.
        let configuration_patch = match &patch.configuration {
            PatchField::Value(value) => serde_json::to_value(value)
                .map_err(|error| InstanceError::Storage(error.to_string()))?,
            _ => serde_json::json!({}),
        };
        sqlx::query("UPDATE instancesetupstates SET updatechecksenabled=$1, mfapolicy=$2, updatedat=$3, updatecheckintervalhours=$4, instanceconfiguration=instanceconfiguration || $5 WHERE id=1")
            .bind(next.update_checks_enabled).bind(next.mfa_policy.as_database_str()).bind(now).bind(next.update_check_interval_hours).bind(sqlx::types::Json(configuration_patch))
            .execute(&mut *tx).await.map_err(storage)?;
        let instance =
            crate::persistence::postgres::licensing::store::get_or_create_identity(&mut tx, now)
                .await
                .map_err(|error| InstanceError::Storage(error.to_string()))?;
        if next.update_checks_enabled != old.update_checks_enabled
            || next.update_check_interval_hours != old.update_check_interval_hours
        {
            let event = ActivityEvent::new_updates_event(
                instance.instance_id,
                principal.actor_id,
                ActivityEventInfo::UpdateChecksChanged {
                    previous_update_checks_enabled: old.update_checks_enabled,
                    update_checks_enabled: next.update_checks_enabled,
                    previous_update_check_interval_hours: Some(old.update_check_interval_hours),
                    update_check_interval_hours: Some(next.update_check_interval_hours),
                },
                citadel_activities::ActivityStatus::Success,
                now,
            )
            .map_err(|error| storage(invalid_activity(error)))?;
            insert_activity(&mut tx, &event).await.map_err(storage)?;
        }
        if next.configuration != old.configuration || next.mfa_policy != old.mfa_policy {
            let event = ActivityEvent::new_instance_event(
                instance.instance_id,
                principal.actor_id,
                ActivityEventInfo::InstanceConfigurationUpdated {
                    previous_configuration: serde_json::to_value(&old.configuration)
                        .map_err(storage)?,
                    configuration: serde_json::to_value(&next.configuration).map_err(storage)?,
                    previous_mfa_policy: old.mfa_policy.as_database_str().into(),
                    mfa_policy: next.mfa_policy.as_database_str().into(),
                },
                citadel_activities::ActivityStatus::Success,
                now,
            )
            .map_err(|error| storage(invalid_activity(error)))?;
            insert_activity(&mut tx, &event).await.map_err(storage)?;
        }
    }
    tx.commit().await.map_err(storage)?;
    Ok(next)
}

fn map(row: &sqlx::postgres::PgRow) -> Result<InstallationSettings, InstanceError> {
    let policy: String = row.try_get("mfapolicy").map_err(storage)?;
    Ok(InstallationSettings {
        configuration: row
            .try_get::<sqlx::types::Json<citadel_instance::InstanceConfiguration>, _>(
                "instanceconfiguration",
            )
            .map_err(storage)?
            .0,
        initialized: row.try_get("initialized").map_err(storage)?,
        update_checks_enabled: row.try_get("updatechecksenabled").map_err(storage)?,
        update_check_interval_hours: row.try_get("updatecheckintervalhours").map_err(storage)?,
        mfa_policy: MfaPolicy::from_database_str(&policy)
            .ok_or_else(|| InstanceError::Storage("Invalid installation MFA policy.".into()))?,
    })
}
