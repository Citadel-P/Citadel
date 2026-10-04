//! Durable job failure snapshots. Per-rule receipts make interrupted evaluation retryable.
use super::*;

pub async fn enqueue(
    tx: &mut Transaction<'_, Postgres>,
    observation: &AlertObservation,
) -> Result<(), sqlx::Error> {
    let payload = serde_json::to_value(observation)
        .map_err(|error| sqlx::Error::Protocol(error.to_string()))?;
    sqlx::query("INSERT INTO alertobservations(id,payload) VALUES($1,$2)")
        .bind(Uuid::now_v7())
        .bind(payload)
        .execute(&mut **tx)
        .await?;
    sqlx::query("SELECT pg_notify('citadel_job_alerts','')")
        .execute(&mut **tx)
        .await?;
    Ok(())
}

impl PostgresAlertRepository {
    pub async fn process_pending_observation(&self, owner: Uuid) -> Result<bool, AlertError> {
        let row = sqlx::query("WITH candidate AS (SELECT id FROM alertobservations WHERE availableat<=CURRENT_TIMESTAMP AND (claimedat IS NULL OR claimedat<CURRENT_TIMESTAMP-INTERVAL '2 minutes') ORDER BY availableat,id FOR UPDATE SKIP LOCKED LIMIT 1) UPDATE alertobservations o SET claimowner=$1,claimedat=CURRENT_TIMESTAMP,attempts=attempts+1 FROM candidate c WHERE o.id=c.id RETURNING o.id,o.payload")
            .bind(owner).fetch_optional(&self.pool).await.map_err(storage)?;
        let Some(row) = row else {
            return Ok(false);
        };
        let id: Uuid = row.try_get("id").map_err(storage)?;
        let result = async {
            let mut observation: AlertObservation =
                serde_json::from_value(row.try_get("payload").map_err(storage)?)
                    .map_err(|e| AlertError::Storage(e.to_string()))?;
            if observation.alert_type == "BuildAgentPoolUnavailable" {
                // Availability is a condition, not a historical failure. A delayed
                // receipt must not reopen an incident after recovery or disablement.
                let state: Option<(Option<DateTime<Utc>>, Option<String>)> = sqlx::query_as("SELECT unavailablesince,lastvalidationmessage FROM buildagentpools WHERE id=$1 AND enabled AND archivedat IS NULL AND provider='SelfManagedVm'")
                    .bind(observation.resource_id).fetch_optional(&self.pool).await.map_err(storage)?;
                observation.observed_at = Utc::now();
                let (since, message) = state.unwrap_or_default();
                observation.info = serde_json::json!({"HumanMessage": message, "UnavailableSince": since});
                observation.matched = since.is_some();
                observation.value = since.map(|at| (observation.observed_at-at).num_milliseconds().max(0) as f64 / 1000.0);
            }
            self.process_observation(&observation, Some(id)).await?;
            sqlx::query("DELETE FROM alertobservations WHERE id=$1 AND claimowner=$2")
                .bind(id)
                .bind(owner)
                .execute(&self.pool)
                .await
                .map_err(storage)?;
            Ok::<_, AlertError>(())
        }
        .await;
        if let Err(error) = result {
            sqlx::query("UPDATE alertobservations SET claimowner=NULL,claimedat=NULL,availableat=CURRENT_TIMESTAMP+LEAST(attempts*5,60)*INTERVAL '1 second' WHERE id=$1 AND claimowner=$2")
                .bind(id).bind(owner).execute(&self.pool).await.map_err(storage)?;
            return Err(error);
        }
        Ok(true)
    }
}
