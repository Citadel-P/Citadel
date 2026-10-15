//! Transactional update-resolution handoff to the existing durable alert queue.
use super::*;
use citadel_alerts::update_resolution::{AppliedStackImage, VerifiedUpdate, is_update_available};

pub(crate) fn stack_images(state: &citadel_stacks::StackUpdateState) -> VerifiedUpdate {
    let (citadel_stacks::StackUpdateState::Git {
        recreate_stack_on_new_image_state,
        ..
    }
    | citadel_stacks::StackUpdateState::WebEditor {
        recreate_stack_on_new_image_state,
    }) = state;
    VerifiedUpdate::StackImages(
        recreate_stack_on_new_image_state
            .auto_update_states
            .iter()
            .map(|image| AppliedStackImage {
                service: image.service_name.clone(),
                image: image.image_name.clone(),
                digest: image.current_digest.clone(),
            })
            .collect(),
    )
}

pub(crate) async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    resource: Uuid,
    verified: &VerifiedUpdate,
) -> Result<(), sqlx::Error> {
    let rows = sqlx::query("SELECT id,alertruleid,info,deduplicationkey,resourcename,resourcetype FROM alertevents WHERE resourceid=$1 AND type=$2 AND resolvedat IS NULL AND openincidentkey IS NOT NULL")
        .bind(resource).bind(verified.alert_type()).fetch_all(&mut **tx).await?;
    for row in rows {
        let info: Value = row.try_get("info")?;
        if !verified.satisfies(&info) {
            continue;
        }
        let rule: Uuid = row.try_get("alertruleid")?;
        let key: String = row.try_get("deduplicationkey")?;
        let Some(component) = key.strip_prefix(&format!("{rule}:{resource}:")) else {
            continue;
        };
        super::observations::enqueue(
            tx,
            &AlertObservation {
                alert_type: verified.alert_type().into(),
                // Fence resolution to this occurrence, even if the target recurs.
                info: serde_json::json!({"ResolvedEventId": row.try_get::<Uuid, _>("id")?}),
                resource_id: resource,
                resource_name: row.try_get("resourcename")?,
                resource_type: row.try_get("resourcetype")?,
                deduplication_component: component.into(),
                observed_at: Utc::now(),
                value: None,
                matched: false,
            },
        )
        .await?;
    }
    Ok(())
}

/// Serialize late update notifications with workload commits. A notification
/// queued before Apply must not reopen an update that is already installed.
pub(super) async fn already_applied(
    tx: &mut Transaction<'_, Postgres>,
    observation: &AlertObservation,
) -> Result<bool, AlertError> {
    if !observation.matched || !is_update_available(&observation.alert_type) {
        return Ok(false);
    }
    let verified = if observation.alert_type == "DeploymentImageUpdateAvailable" {
        let digest: Option<Option<String>> = sqlx::query_scalar(
            "SELECT autoupdatestate_currentdigest FROM deployments WHERE id=$1 FOR SHARE",
        )
        .bind(observation.resource_id)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
        digest.flatten().map(VerifiedUpdate::DeploymentImage)
    } else {
        let state: Option<Value> =
            sqlx::query_scalar("SELECT stackupdatestate FROM stacks WHERE id=$1 FOR SHARE")
                .bind(observation.resource_id)
                .fetch_optional(&mut **tx)
                .await
                .map_err(storage)?;
        let state = state
            .map(citadel_stacks::StackUpdateState::from_storage_value)
            .transpose()
            .map_err(|error| AlertError::Storage(error.to_string()))?;
        match state {
            Some(state) if observation.alert_type == "StackImageUpdateAvailable" => {
                Some(stack_images(&state))
            }
            Some(citadel_stacks::StackUpdateState::Git {
                recreate_stack_on_new_commit_state,
                ..
            }) => Some(VerifiedUpdate::StackGit(
                recreate_stack_on_new_commit_state.current_commit_sha,
            )),
            _ => None,
        }
    };
    Ok(verified.is_some_and(|state| state.satisfies(&observation.info)))
}

impl PostgresAlertRepository {
    pub(super) async fn resolve_applied_update(
        &self,
        observation: &AlertObservation,
    ) -> Result<(), AlertError> {
        // Resolution is housekeeping for an existing event: disabling its rule,
        // quiet hours or licensing changes must not leave an installed update open.
        let event_id: Uuid = serde_json::from_value(
            observation
                .info
                .get("ResolvedEventId")
                .cloned()
                .unwrap_or(Value::Null),
        )
        .map_err(|error| {
            AlertError::Storage(format!("Invalid update resolution event: {error}"))
        })?;
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let rule: Option<Uuid> = sqlx::query_scalar("SELECT alertruleid FROM alertevents WHERE id=$1 AND resourceid=$2 AND type=$3 AND resolvedat IS NULL")
            .bind(event_id).bind(observation.resource_id).bind(&observation.alert_type).fetch_optional(&mut *tx).await.map_err(storage)?;
        let Some(rule) = rule else {
            return Ok(());
        };
        // Match evaluation locks rule/resource before counters and event rows.
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
            .bind(format!("{rule}:{}", observation.resource_id))
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let changed = sqlx::query("UPDATE alertevents SET resolvedat=$4,resolvedbyactorid='00000000-0000-0000-0000-000000000001',resolutionnote='Update applied successfully.',updatedat=$4,openincidentkey=NULL WHERE id=$5 AND resourceid=$1 AND type=$2 AND deduplicationkey=alertruleid::text || ':' || resourceid::text || ':' || $3 AND resolvedat IS NULL")
            .bind(observation.resource_id).bind(&observation.alert_type).bind(&observation.deduplication_component)
            .bind(observation.observed_at).bind(event_id).execute(&mut *tx).await.map_err(storage)?.rows_affected() > 0;
        if changed {
            sqlx::query("UPDATE alertrulestates s SET consecutivematches=0 WHERE s.resourceid=$1 AND s.alertruleid=$2 AND NOT EXISTS(SELECT 1 FROM alertevents e WHERE e.alertruleid=s.alertruleid AND e.resourceid=s.resourceid AND e.resolvedat IS NULL)")
                .bind(observation.resource_id).bind(rule).execute(&mut *tx).await.map_err(storage)?;
        }
        tx.commit().await.map_err(storage)?;
        if changed {
            self.changed();
        }
        Ok(())
    }
}
