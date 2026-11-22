//! Installation-wide configuration, policy and persisted administrator choices.
#![forbid(unsafe_code)]
mod commands;
pub mod environment;
mod error;
mod job_history;
mod jobs;
pub use job_history::{
    JobDiagnosticCapture, JobDiagnosticSource, JobHistory, JobHistoryBatch, JobHistoryBucket,
    JobHistoryEvent, JobHistoryMaintenance, JobHistoryRecord, JobHistoryRepository,
    JobHistorySession, history_buckets,
};
pub use jobs::{
    InstanceJobsReader, JobOutcome, JobOwnership, JobProblem, JobState, JobStatus, JobTarget,
    JobsSnapshot,
};
mod model;
mod read_models;
mod repository;
mod restart;
mod service;
mod validation;

pub use commands::{PatchInstallationSettings, PatchInstanceConfiguration};
pub use error::InstanceError;
pub use model::{
    InstallationPolicy, InstallationSettings, InstanceConfiguration, InstanceOverrides, LogFormat,
};
pub use read_models::DatabaseConnections;
pub use repository::{InstanceRepository, InstanceRestartRepository};
pub use restart::{InstanceRestartService, RestartStatus};
pub use service::InstanceService;

#[cfg(test)]
mod tests;

mod schedules;
pub use schedules::{
    SCHEDULE_PAGE_SIZE, ScheduleActivity, ScheduleBatchGuard, ScheduleCatalogPolicy, ScheduleCount,
    ScheduleDefinition, ScheduleMode, SchedulePolicyBlocker, SchedulePurpose, ScheduleReader,
    ScheduledRun, calendar_deadline,
};
