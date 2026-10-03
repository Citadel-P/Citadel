use crate::*;

#[derive(Debug, Clone)]
pub struct BackupRepository {
    pub id: Uuid,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub repository_type: String,
    pub spec: crate::spec::BackupRepositorySpec,
    pub password_secret_id: Uuid,
    pub status: BackupRepositoryStatus,
    pub control_state: citadel_primitives::ResourceControlState,
    pub current_run_id: Option<Uuid>,
    pub control_started_at: Option<i64>,
    pub last_pruned_at: Option<DateTime<Utc>>,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    pub archived_at: Option<DateTime<Utc>>,
    pub row_version: i64,
    pub audit: citadel_primitives::AuditMetadata,
}

#[derive(Debug, Clone)]
pub struct BackupRepositoryOperationResult {
    pub repository: BackupRepository,
    pub validation: BackupRepositoryValidation,
    pub succeeded: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct BackupRepositoryValidation {
    pub id: Uuid,
    pub backup_repository_id: Uuid,
    pub location: String,
    pub platform_id: Option<Uuid>,
    pub status: BackupRepositoryValidationStatus,
    pub last_validated_at: DateTime<Utc>,
    pub last_error_code: Option<String>,
    pub last_error_message: Option<String>,
}
