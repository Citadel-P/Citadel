#![forbid(unsafe_code)]

mod error;
pub mod policies;
pub mod repositories;
mod repository;
pub mod restores;
pub mod runs;
mod runtime;
mod schedule;
mod service;
#[cfg(test)]
mod tests;
mod validation;
use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;
use citadel_domain::ActorId;
pub use error::BackupError;
use futures_util::future::BoxFuture;
pub use policies::{BackupPolicy, BackupPolicyConfiguration};
pub use repositories::{
    BackupRepository, BackupRepositoryConfiguration, BackupRepositoryOperationResult,
    BackupRepositoryValidation,
};
pub use repository::BackupPersistence;
pub use restores::{BackupRestoreRequest, BackupRestoreRun, RestoreClaim, RestoreExecutionResult};
pub use runs::{
    BackupClaim, BackupExecutionResult, BackupLog, BackupRun, BackupRunItem, BackupRunItemResult,
};
pub use runtime::{
    BackupEntitlements, BackupExecutor, BackupRunAuthorizer, BackupSecretResolver,
    BackupSourceItem, BackupSourcePlan, BackupSourcePlanner, CitadelSystemBackupBuilder,
};
use schedule::{schedule_is_due, valid_cron};
use serde::{Deserialize, Serialize};
use serde_json::Value;
pub use service::BackupService;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;
use validation::{discriminator, required_string, required_uuid, reqwest_url, validate_name};

pub mod permissions;
