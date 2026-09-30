pub use citadel_backups::spec::*;
pub use citadel_backups::{
    BackupCoverageStatus, BackupRepositoryStatus, BackupRepositoryValidationStatus,
    BackupRestoreStatus, BackupRunItemStatus, BackupRunStatus, BackupSnapshotAvailability,
};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BackupRepositoryType {
    FileSystem,
    S3Compatible,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BackupRunTrigger {
    Manual,
    Schedule,
    Automation,
    Webhook,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, Serialize, utoipa::ToSchema)]
pub enum BackupQueueTrigger {
    #[default]
    Manual,
    Schedule,
    Webhook,
}
impl BackupQueueTrigger {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "Manual",
            Self::Schedule => "Schedule",
            Self::Webhook => "Webhook",
        }
    }
}
