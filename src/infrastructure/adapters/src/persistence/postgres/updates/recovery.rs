use super::*;

impl PostgresUpdateRepository {
    pub(super) async fn import_impl<'a>(
        &'a self,
        journal: &'a UpdateJournal,
    ) -> Result<Option<CompletionReceipt>, UpdateError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let current = sqlx::query_scalar::<_, sqlx::types::Json<UpdateOperation>>(
            "SELECT operation FROM instanceupdates WHERE id=$1 FOR UPDATE",
        )
        .bind(journal.operation.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        .ok_or(UpdateError::NotFound)?
        .0;
        journal.validate_for(&current)?;
        if instance(&mut tx).await? != current.instance_id {
            return Err(UpdateError::Conflict(
                "Journal belongs to another instance.".into(),
            ));
        }
        let next = &journal.operation;
        let receipt = (next.phase == UpdatePhase::Succeeded).then(|| CompletionReceipt {
            instance_id: next.instance_id,
            operation_id: next.id,
            fence: next.fence,
            revision: next.revision,
            target: next.plan.target.clone(),
        });
        for transition in journal
            .history
            .iter()
            .filter(|v| v.revision > current.revision)
        {
            operation_activity(&mut tx, transition).await?;
        }
        sqlx::query(
            "UPDATE instanceupdates SET operation=$2,revision=$3,completion=$4 WHERE id=$1",
        )
        .bind(next.id)
        .bind(sqlx::types::Json(next))
        .bind(i64::try_from(next.revision).map_err(storage)?)
        .bind(receipt.as_ref().map(sqlx::types::Json))
        .execute(&mut *tx)
        .await
        .map_err(storage)?;
        tx.commit().await.map_err(storage)?;
        Ok(receipt)
    }

    pub(super) async fn release_impl<'a>(
        &'a self,
        journal: &'a UpdateJournal,
        running: &'a CoreIdentity,
    ) -> Result<(), UpdateError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let row =
            sqlx::query("SELECT operation,completion FROM instanceupdates WHERE id=$1 FOR UPDATE")
                .bind(journal.operation.id)
                .fetch_optional(&mut *tx)
                .await
                .map_err(storage)?
                .ok_or(UpdateError::NotFound)?;
        let current = row
            .try_get::<sqlx::types::Json<UpdateOperation>, _>("operation")
            .map_err(storage)?
            .0;
        journal.validate_for(&current)?;
        if matches!(current.phase, UpdatePhase::Failed | UpdatePhase::Postponed) {
            journal.permits_source_resume(&current, running)?;
        } else {
            let receipt = row
                .try_get::<Option<sqlx::types::Json<CompletionReceipt>>, _>("completion")
                .map_err(storage)?
                .ok_or_else(|| UpdateError::Conflict("Completion is not committed.".into()))?
                .0;
            receipt.permits_resume(journal, running)?;
        }
        if instance(&mut tx).await? != current.instance_id {
            return Err(UpdateError::Conflict(
                "Update belongs to another instance.".into(),
            ));
        }
        sqlx::query("UPDATE instanceupdates SET ownsinstallation=FALSE WHERE id=$1")
            .bind(current.id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(storage)
    }
}
