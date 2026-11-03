use super::*;
use citadel_updates::{UpdateDrainBlocker, UpdateDrainRepository};

impl UpdateDrainRepository for PostgresUpdateRepository {
    fn blockers<'a>(
        &'a self,
        expected: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<Vec<UpdateDrainBlocker>, UpdateError>> {
        Box::pin(async move {
            let mut tx = self.pool.begin().await.map_err(storage)?;
            sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("SET LOCAL statement_timeout = '2s'")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            sqlx::query("SET LOCAL lock_timeout = '200ms'")
                .execute(&mut *tx)
                .await
                .map_err(storage)?;
            let row = sqlx::query("SELECT * FROM instanceupdates WHERE id=$1")
                .bind(expected.id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(UpdateError::NotFound)?;
            let current = super::queries::read_operation(&row)?;
            if current != *expected
                || !row
                    .try_get::<bool, _>("ownsinstallation")
                    .map_err(storage)?
                || instance(&mut tx).await? != expected.instance_id
                || !matches!(
                    current.phase,
                    UpdatePhase::Preparing | UpdatePhase::Draining
                )
                || current.checkpoint.is_some()
                || current.revision
                    != if current.phase == UpdatePhase::Preparing {
                        0
                    } else {
                        1
                    }
            {
                return Err(UpdateError::Conflict(
                    "Persisted update ownership changed or is invalid.".into(),
                ));
            }
            // One snapshot includes both active runs and independently retained
            // cleanup evidence. Expired leases and terminal run labels do not prove
            // remote work has stopped. Queued work remains queued, without blocking.
            let rows: Vec<(String, i64)> = sqlx::query_as(include_str!("drain.sql"))
                .fetch_all(&mut *tx)
                .await
                .map_err(storage)?;
            let blockers = rows
                .into_iter()
                .map(|(kind, count)| {
                    Ok(UpdateDrainBlocker {
                        kind,
                        count: u64::try_from(count).map_err(storage)?,
                    })
                })
                .collect::<Result<Vec<_>, UpdateError>>()?;
            tx.commit().await.map_err(storage)?;
            Ok(blockers)
        })
    }
}
