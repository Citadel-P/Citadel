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
            let observation: AlertObservation =
                serde_json::from_value(row.try_get("payload").map_err(storage)?)
                    .map_err(|e| AlertError::Storage(e.to_string()))?;
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
