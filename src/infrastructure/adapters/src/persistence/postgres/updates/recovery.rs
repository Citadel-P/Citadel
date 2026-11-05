use super::*;

impl PostgresUpdateRepository {
    pub(super) async fn import_impl<'a>(
        &'a self,
        journal: &'a UpdateJournal,
    ) -> Result<Option<CompletionReceipt>, UpdateError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        let row = sqlx::query("SELECT * FROM instanceupdates WHERE id=$1 FOR UPDATE")
            .bind(journal.operation.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(UpdateError::NotFound)?;
        let current = super::queries::read_operation(&row)?;
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
        if !row
            .try_get::<bool, _>("ownsinstallation")
            .map_err(storage)?
        {
            let saved = row
                .try_get::<Option<sqlx::types::Json<CompletionReceipt>>, _>("completion")
                .map_err(storage)?;
            // Exact terminal replay is read-only, even if a newer claim now owns
            // the instance. Released attempts can never advance or write Activities.
            if !matches!(
                current.phase,
                UpdatePhase::Succeeded | UpdatePhase::Failed | UpdatePhase::Postponed
            ) || next != &current
                || saved.as_ref().map(|v| &v.0) != receipt.as_ref()
            {
                return Err(UpdateError::Conflict(
                    "The update no longer owns this installation.".into(),
                ));
            }
            tx.rollback().await.map_err(storage)?;
            return Ok(receipt);
        }
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
        let row = sqlx::query("SELECT * FROM instanceupdates WHERE id=$1 FOR UPDATE")
            .bind(journal.operation.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(UpdateError::NotFound)?;
        let current = super::queries::read_operation(&row)?;
        journal.validate_for(&current)?;
        if journal.operation != current {
            return Err(UpdateError::Conflict(
                "Update outcome is not committed.".into(),
            ));
        }
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
        if !row
            .try_get::<bool, _>("ownsinstallation")
            .map_err(storage)?
        {
            tx.rollback().await.map_err(storage)?;
            return Ok(());
        }
        sqlx::query("UPDATE instanceupdates SET ownsinstallation=FALSE WHERE id=$1")
            .bind(current.id)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(storage)
    }
}

impl PostgresUpdateRepository {
    pub(super) async fn abort_source_impl(
        &self,
        operation: &UpdateOperation,
    ) -> Result<(), UpdateError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        sqlx::query("SET LOCAL lock_timeout='5s'")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        sqlx::query("SET LOCAL statement_timeout='15s'")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let row = sqlx::query("SELECT * FROM instanceupdates WHERE id=$1 FOR UPDATE")
            .bind(operation.id)
            .fetch_optional(&mut *tx)
            .await
            .map_err(storage)?
            .ok_or(UpdateError::NotFound)?;
        let current = super::queries::read_operation(&row)?;
        if current != *operation
            || !matches!(
                current.phase,
                UpdatePhase::Preparing
                    | UpdatePhase::Draining
                    | UpdatePhase::Failed
                    | UpdatePhase::Postponed
            )
        {
            return Err(UpdateError::Conflict(
                "Source recovery ownership changed.".into(),
            ));
        }
        let probe: sqlx::types::Json<super::checkpoint::Probe> =
            sqlx::query_scalar(super::checkpoint::CHECKPOINT_PROBE_SQL)
                .fetch_one(&mut *tx)
                .await
                .map_err(storage)?;
        let migrations = super::checkpoint::checked_restore_migrations(probe.0, &current)?;
        if citadel_database::MigrationRunner::validate_applied_catalog(&migrations)
            .map_err(storage)?
            != current.plan.source.catalog_sha256
        {
            return Err(UpdateError::Conflict(
                "Source catalog changed; recovery remains required.".into(),
            ));
        }
        let mut failed = current;
        if !failed.phase.terminal() {
            failed.advance(failed.revision, UpdatePhase::Failed, Utc::now())?;
            failed.message = Some("Update stopped before source shutdown. The unchanged source was verified and maintenance released.".into());
            operation_activity(&mut tx, &failed).await?;
        }
        sqlx::query("UPDATE instanceupdates SET operation=$2,revision=$3,ownsinstallation=FALSE WHERE id=$1")
            .bind(failed.id).bind(sqlx::types::Json(&failed)).bind(i64::try_from(failed.revision).map_err(storage)?)
            .execute(&mut *tx).await.map_err(storage)?;
        tx.commit().await.map_err(storage)
    }
}

impl PostgresUpdateRepository {
    pub(super) async fn interrupt_impl(
        &self,
        operation: &UpdateOperation,
    ) -> Result<(), UpdateError> {
        let mut tx = self.pool.begin().await.map_err(storage)?;
        sqlx::query("SET LOCAL lock_timeout='5s'")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        sqlx::query("SET LOCAL statement_timeout='15s'")
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        let row = sqlx::query(
            "SELECT * FROM instanceupdates WHERE id=$1 AND ownsinstallation FOR UPDATE",
        )
        .bind(operation.id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(storage)?
        .ok_or(UpdateError::NotFound)?;
        let current = super::queries::read_operation(&row)?;
        if current != *operation || instance(&mut tx).await? != current.instance_id {
            return Err(UpdateError::Conflict("Update ownership changed.".into()));
        }
        if !matches!(
            current.phase,
            UpdatePhase::Preparing | UpdatePhase::Draining
        ) {
            return Ok(());
        }
        let mut failed = current;
        failed.advance(failed.revision, UpdatePhase::Failed, Utc::now())?;
        failed.message = Some("Update preparation was interrupted. Maintenance is retained. Run recover-update-source for this operation using the original Core; ambiguous shutdown requires checkpoint recovery.".into());
        operation_activity(&mut tx, &failed).await?;
        sqlx::query("UPDATE instanceupdates SET operation=$2,revision=$3 WHERE id=$1")
            .bind(failed.id)
            .bind(sqlx::types::Json(&failed))
            .bind(i64::try_from(failed.revision).map_err(storage)?)
            .execute(&mut *tx)
            .await
            .map_err(storage)?;
        tx.commit().await.map_err(storage)
    }
}
