use super::*;

impl PostgresAlertRepository {
    pub(super) fn claim_delivery_impl(
        &self,
        owner: Uuid,
        stale_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<Option<AlertDeliveryClaim>, AlertError>> {
        Box::pin(async move {
            let claimed = sqlx::query(
                r#"WITH candidate AS (
    SELECT queue.id
    FROM alertdeliveryoutbox queue
    JOIN alertchannels channel ON channel.id=queue.alertchannelid AND channel.isactive
    WHERE (queue.status='Pending' AND queue.nextattemptat<=CURRENT_TIMESTAMP)
       OR (queue.status='Delivering' AND queue.claimedat<$2)
    ORDER BY queue.nextattemptat,queue.id
    FOR UPDATE OF queue SKIP LOCKED
    LIMIT 1
)
UPDATE alertdeliveryoutbox queue
SET status='Delivering',claimowner=$1,claimedat=CURRENT_TIMESTAMP,updatedat=CURRENT_TIMESTAMP
FROM candidate
WHERE queue.id=candidate.id
RETURNING queue.id,queue.alerteventid,queue.alertchannelid,queue.attemptcount"#,
            )
            .bind(owner)
            .bind(stale_before)
            .fetch_optional(&self.pool)
            .await
            .map_err(storage)?;
            let Some(claimed) = claimed else {
                return Ok(None);
            };
            let id: Uuid = claimed.try_get("id").map_err(storage)?;
            let event_id: Uuid = claimed.try_get("alerteventid").map_err(storage)?;
            let channel_id: Uuid = claimed.try_get("alertchannelid").map_err(storage)?;
            let attempt_count: i32 = claimed.try_get("attemptcount").map_err(storage)?;
            let event = sqlx::query("SELECT * FROM alertevents WHERE id=$1")
                .bind(event_id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
                .and_then(map_event)?;
            let channel = sqlx::query("SELECT * FROM alertchannels WHERE id=$1 AND isactive")
                .bind(channel_id)
                .fetch_one(&self.pool)
                .await
                .map_err(storage)
                .and_then(map_channel)?;
            Ok(Some(AlertDeliveryClaim {
                id,
                attempt_count,
                channel,
                event,
            }))
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn complete_delivery_impl(
        &self,
        id: Uuid,
        owner: Uuid,
    ) -> BoxFuture<'_, Result<bool, AlertError>> {
        Box::pin(async move {
            let changed = sqlx::query("DELETE FROM alertdeliveryoutbox WHERE id=$1 AND status='Delivering' AND claimowner=$2")
                .bind(id)
                .bind(owner)
                .execute(&self.pool)
                .await
                .map_err(storage)?
                .rows_affected();
            Ok(changed == 1)
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn retry_delivery_impl<'a>(
        &'a self,
        id: Uuid,
        owner: Uuid,
        next_attempt_at: DateTime<Utc>,
        dead_letter: bool,
        error: &'a str,
    ) -> BoxFuture<'a, Result<bool, AlertError>> {
        Box::pin(async move {
            let error = error.chars().take(2_048).collect::<String>();
            let status = if dead_letter { "DeadLetter" } else { "Pending" };
            let changed = sqlx::query("UPDATE alertdeliveryoutbox SET status=$3,attemptcount=attemptcount+1,nextattemptat=$4,lasterror=$5,claimowner=NULL,claimedat=NULL,updatedat=CURRENT_TIMESTAMP WHERE id=$1 AND status='Delivering' AND claimowner=$2")
                .bind(id)
                .bind(owner)
                .bind(status)
                .bind(next_attempt_at)
                .bind(error)
                .execute(&self.pool)
                .await
                .map_err(storage)?
                .rows_affected();
            Ok(changed == 1)
        })
    }
}

impl PostgresAlertRepository {
    pub(super) fn maintain_deliveries_impl(
        &self,
        stale_before: DateTime<Utc>,
        dead_before: DateTime<Utc>,
    ) -> BoxFuture<'_, Result<(), AlertError>> {
        Box::pin(async move {
            let mut transaction = self.pool.begin().await.map_err(storage)?;
            sqlx::query("DELETE FROM alertdeliveryoutbox queue USING alertchannels channel WHERE queue.alertchannelid=channel.id AND NOT channel.isactive")
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            sqlx::query(
                "DELETE FROM alertdeliveryoutbox WHERE status='DeadLetter' AND updatedat<$1",
            )
            .bind(dead_before)
            .execute(&mut *transaction)
            .await
            .map_err(storage)?;
            sqlx::query("UPDATE alertdeliveryoutbox SET status='Pending',claimowner=NULL,claimedat=NULL,updatedat=CURRENT_TIMESTAMP WHERE status='Delivering' AND claimedat<$1")
                .bind(stale_before)
                .execute(&mut *transaction)
                .await
                .map_err(storage)?;
            transaction.commit().await.map_err(storage)?;
            Ok(())
        })
    }
}

pub(super) async fn enqueue_deliveries(
    transaction: &mut Transaction<'_, Postgres>,
    event_id: Uuid,
    rule_id: Uuid,
) -> Result<(), AlertError> {
    sqlx::query(
        r#"INSERT INTO alertdeliveryoutbox(id,alerteventid,alertchannelid)
SELECT gen_random_uuid(),$1,relation.alertchannelid
FROM alertrulechannels relation
JOIN alertchannels channel ON channel.id=relation.alertchannelid AND channel.isactive
WHERE relation.alertruleid=$2
ON CONFLICT(alerteventid,alertchannelid) DO NOTHING"#,
    )
    .bind(event_id)
    .bind(rule_id)
    .execute(&mut **transaction)
    .await
    .map_err(storage)?;
    Ok(())
}
