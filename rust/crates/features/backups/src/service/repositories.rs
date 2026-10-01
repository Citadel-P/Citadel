use super::*;

impl BackupService {
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
        let result = self
            .executor
            .repository(&repository, operation, location, platform_id, cancellation)
            .await;
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
