#![forbid(unsafe_code)]

mod error;
pub mod policies;
pub mod repositories;
mod repository;
pub mod restores;
pub mod runs;
mod runtime;
mod service;
pub mod spec;
#[cfg(test)]
mod tests;
mod validation;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use citadel_primitives::schedule::{CronSchedule, schedule_is_due};
pub use error::BackupError;
use futures_util::future::BoxFuture;
pub use policies::{BackupPolicy, BackupPolicyConfiguration};
pub use repositories::{
    BackupRepository, BackupRepositoryConfiguration, BackupRepositoryOperationResult,
    BackupRepositoryValidation,
};
pub use repository::{BackupPersistence, BackupRepositoryOperation};
pub use restores::{BackupRestoreRequest, BackupRestoreRun, RestoreClaim, RestoreExecutionResult};
pub use runs::{
    BackupClaim, BackupExecutionResult, BackupLog, BackupRun, BackupRunItem, BackupRunItemResult,
};
pub use runtime::{
    BackupEntitlements, BackupExecutor, BackupRunAuthorizer, BackupSecretResolver,
    BackupSourceItem, BackupSourcePlan, BackupSourcePlanner, CitadelSystemBackupBuilder,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use service::BackupService;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use validation::validate_name;

pub mod permissions;

mod status;
pub use status::{
    BackupCoverageStatus, BackupRepositoryStatus, BackupRepositoryValidationStatus,
    BackupRestoreStatus, BackupRunItemStatus, BackupRunStatus, BackupSnapshotAvailability,
};
