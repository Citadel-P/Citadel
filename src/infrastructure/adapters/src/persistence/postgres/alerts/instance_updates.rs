//! Revalidate queued availability against its latest persisted observation.
use super::*;

pub(super) fn is_instance_update(kind: &str) -> bool {
    matches!(kind, "CoreUpdateAvailable" | "AgentUpdatesAvailable")
}

pub(super) async fn is_current(
    tx: &mut Transaction<'_, Postgres>,
    observation: &AlertObservation,
) -> Result<bool, AlertError> {
    let query = if observation.alert_type == "CoreUpdateAvailable" {
        "SELECT availability FROM instanceupdatechecks WHERE id=1 FOR SHARE"
    } else {
        "SELECT availability FROM agentupdatesettings WHERE id=1 FOR SHARE"
    };
    let current: Option<Option<Value>> = sqlx::query_scalar(query)
        .fetch_optional(&mut **tx)
        .await
        .map_err(storage)?;
    Ok(current.flatten()
        == Some(serde_json::to_value(observation).map_err(|e| AlertError::Storage(e.to_string()))?))
}

impl PostgresAlertRepository {
    pub(super) async fn resolve_instance_update(
        &self,
        observation: &AlertObservation,
    ) -> Result<(), AlertError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        if !is_current(&mut tx, observation).await? {
            return Ok(());
        }
        // Housekeeping must also work when a rule is disabled, quiet, or unlicensed.
        // The state-row lock fences late queue entries and concurrent new releases.
        let through = if observation.alert_type == "CoreUpdateAvailable" {
            observation.info["InstalledVersion"].as_str()
        } else if observation.info["CompleteInventory"] == true {
            Some(observation.deduplication_component.as_str())
        } else {
            None
        };
        let through = through.and_then(|version| citadel_updates::release_version(version).ok());
        let events: Vec<(Uuid, Value)> = sqlx::query_as("SELECT id,info FROM alertevents WHERE resourceid=$1 AND type=$2 AND resolvedat IS NULL")
            .bind(observation.resource_id).bind(&observation.alert_type).fetch_all(&mut *tx).await.map_err(storage)?;
        let ids: Vec<_> = events
            .into_iter()
            .filter(|(_, info)| {
                info["TargetVersion"] == observation.deduplication_component
                    || through.as_ref().is_some_and(|through| {
                        info["TargetVersion"]
                            .as_str()
                            .and_then(|v| citadel_updates::release_version(v).ok())
                            .is_some_and(|v| v <= *through)
                    })
            })
            .map(|(id, _)| id)
            .collect();
        let changed = sqlx::query("UPDATE alertevents SET resolvedat=$2,resolvedbyactorid='00000000-0000-0000-0000-000000000001',resolutionnote='Advertised update no longer required.',updatedat=$2,openincidentkey=NULL WHERE id=ANY($1) AND resolvedat IS NULL")
            .bind(ids).bind(observation.observed_at).execute(&mut *tx).await.map_err(storage)?.rows_affected();
        tx.commit().await.map_err(storage)?;
        if changed > 0 {
            self.changed();
        }
        Ok(())
    }
}
