use super::*;

impl BackupService {
    pub async fn update_repository_credentials(
        &self,
        id: Uuid,
        credentials: &BackupRepositoryCredentials,
        location: &str,
        platform_id: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<BackupRepository, BackupError> {
        let _tracking = self
            .tracking_slot(cancellation)
            .await
            .ok_or_else(|| BackupError::Conflict("Repository operation was cancelled.".into()))?;
        let operation_id = Uuid::now_v7();
        if !self
            .store
            .acquire_repository_operation(
                id,
                operation_id,
                "Validate",
                Utc::now() + self.repository_operation_lease,
            )
            .await?
        {
            return Err(BackupError::Conflict(
                "Backup Repository is already in use.".into(),
            ));
        }
        let token = cancellation.child_token();
        let execution = async {
            if token.is_cancelled() {
                return Err(BackupError::CleanupPending);
            }
            let mut candidate = self.store.get_repository(id).await?;
            credentials.apply(&mut candidate.spec)?;
            // Validate the candidate without replacing the persisted configuration or readiness.
            self.executor.repository(&candidate, operation_id, "Validate", location, platform_id, &token)
                .await.map_err(|error| if matches!(error, BackupError::CleanupPending | BackupError::Conflict(_)) { error } else { BackupError::Validation(
                    "Could not validate the new S3 credentials. Check the selected secrets and repository connectivity. Existing credentials were not changed.".into()) })?;
            if cancellation.is_cancelled() {
                return Err(BackupError::Conflict(
                    "Credential update was cancelled; existing credentials were not changed."
                        .into(),
                ));
            }
            self.store
                .commit_repository_credentials(id, operation_id, candidate.row_version, credentials)
                .await
        };
        let (result, owned) = self
            .with_lease(id, operation_id, operation_id, &token, execution)
            .await;
        if !owned || matches!(result, Err(BackupError::CleanupPending)) {
            return Err(BackupError::CleanupPending);
        }
        let released = self
            .store
            .release_repository_operation(id, operation_id)
            .await;
        if result.is_ok() {
            self.changed();
        }
        let repository = result?;
        released?;
        Ok(repository)
    }

    pub async fn repository_operation(
        &self,
        id: Uuid,
        operation: &str,
        location: &str,
        platform_id: Option<Uuid>,
        cancellation: &CancellationToken,
    ) -> Result<BackupRepositoryOperationResult, BackupError> {
        if !matches!(operation, "Validate" | "Initialize" | "Check" | "Prune") {
            return Err(BackupError::Validation(
                "Backup Repository operation is invalid.".into(),
            ));
        }
        let _tracking = self
            .tracking_slot(cancellation)
            .await
            .ok_or_else(|| BackupError::Conflict("Repository operation was cancelled.".into()))?;
        let operation_id = Uuid::now_v7();
        if !self
            .store
            .acquire_repository_operation(
                id,
                operation_id,
                operation,
                Utc::now() + self.repository_operation_lease,
            )
            .await?
        {
            return Err(BackupError::Conflict(
                "Backup Repository is already in use.".into(),
            ));
        }
        // Read the location only after acquisition prevents configuration edits.
        let repository = match self.store.get_repository(id).await {
            Ok(repository) => repository,
            Err(error) => {
                self.store
                    .release_repository_operation(id, operation_id)
                    .await?;
                return Err(error);
            }
        };
        if operation == "Initialize" && repository.status == BackupRepositoryStatus::Ready {
            self.store
                .release_repository_operation(id, operation_id)
                .await?;
            return Err(BackupError::Conflict(
                "Backup Repository is already ready. Use Validate or Check instead of Initialize."
                    .into(),
            ));
        }
        let token = cancellation.child_token();
        let (result, owned) = self
            .with_lease(
                id,
                operation_id,
                operation_id,
                &token,
                self.executor.repository(
                    &repository,
                    operation_id,
                    operation,
                    location,
                    platform_id,
                    &token,
                ),
            )
            .await;
        if !owned || matches!(result, Err(BackupError::CleanupPending)) {
            return Err(BackupError::CleanupPending);
        }
        if let Err(BackupError::Conflict(message)) = &result {
            self.store
                .release_repository_operation(id, operation_id)
                .await?;
            return Err(BackupError::Conflict(message.clone()));
        }
        let error_message = result.as_ref().err().map(ToString::to_string);
        let recorded = self
            .store
            .record_repository_operation(
                id,
                BackupRepositoryOperation {
                    operation_id,
                    operation,
                    location,
                    platform_id,
                    succeeded: result.is_ok(),
                    message: error_message.as_deref(),
                },
            )
            .await;
        let released = self
            .store
            .release_repository_operation(id, operation_id)
            .await;
        let (repository, validation) = recorded?;
        released?;
        self.changed();
        // Validate returns the recorded readiness result. Mutating operations
        // must preserve configuration errors as client errors after releasing the lease.
        if operation != "Validate"
            && let Err(BackupError::Validation(message)) = &result
        {
            return Err(BackupError::Validation(message.clone()));
        }
        Ok(BackupRepositoryOperationResult {
            repository,
            validation,
            succeeded: result.is_ok(),
            error_message,
        })
    }
}
