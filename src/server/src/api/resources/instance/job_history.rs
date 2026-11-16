use super::jobs::JobsSnapshotView;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use uuid::Uuid;
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryEventView {
    pub sequence: u64,
    pub job_id: String,
    pub at: u64,
    pub kind: String,
    pub problem: Option<super::jobs::JobProblemView>,
}
impl From<citadel_instance::JobHistoryEvent> for JobHistoryEventView {
    fn from(v: citadel_instance::JobHistoryEvent) -> Self {
        Self {
            sequence: v.sequence,
            job_id: v.job_id,
            at: v.at,
            kind: v.kind,
            problem: v.problem.map(Into::into),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobHistorySessionView {
    pub core_id: Uuid,
    pub version: String,
    pub snapshot: JobsSnapshotView,
    pub stopped_at: Option<u64>,
    pub heartbeat_at: u64,
    pub lost_events: u64,
}
impl From<citadel_instance::JobHistorySession> for JobHistorySessionView {
    fn from(v: citadel_instance::JobHistorySession) -> Self {
        Self {
            core_id: v.core_id,
            version: v.version,
            snapshot: v.snapshot.into(),
            stopped_at: v.stopped_at,
            heartbeat_at: v.heartbeat_at,
            lost_events: v.lost_events,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryRecordView {
    pub boot_id: Uuid,
    pub event: JobHistoryEventView,
}
impl From<citadel_instance::JobHistoryRecord> for JobHistoryRecordView {
    fn from(v: citadel_instance::JobHistoryRecord) -> Self {
        Self {
            boot_id: v.boot_id,
            event: v.event.into(),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryBucketView {
    pub boot_id: Uuid,
    pub job_id: String,
    pub at: u64,
    pub through: u64,
    /// Width of this completion window: five minutes or one hour.
    pub resolution_ms: u64,
    pub iterations: u64,
    pub productive: u64,
    pub no_work: u64,
    pub failures: u64,
    pub elapsed_ms: u64,
    pub productive_ms: u64,
    pub no_work_ms: u64,
    pub units: Option<u64>,
    pub unit: Option<String>,
    pub backlog: Option<u64>,
    pub backlog_is_lower_bound: bool,
    pub gap: bool,
}
impl From<citadel_instance::JobHistoryBucket> for JobHistoryBucketView {
    fn from(v: citadel_instance::JobHistoryBucket) -> Self {
        Self {
            boot_id: v.boot_id,
            job_id: v.job_id,
            at: v.at,
            through: v.through,
            resolution_ms: v.resolution_ms,
            iterations: v.iterations,
            productive: v.productive,
            no_work: v.no_work,
            failures: v.failures,
            elapsed_ms: v.elapsed_ms,
            productive_ms: v.productive_ms,
            no_work_ms: v.no_work_ms,
            units: v.units,
            unit: v.unit,
            backlog: v.backlog,
            backlog_is_lower_bound: v.backlog_is_lower_bound,
            gap: v.gap,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryView {
    pub observed_at: u64,
    pub sessions: Vec<JobHistorySessionView>,
    pub events: Vec<JobHistoryRecordView>,
    pub buckets: Vec<JobHistoryBucketView>,
    pub truncated: bool,
}
impl From<citadel_instance::JobHistory> for JobHistoryView {
    fn from(v: citadel_instance::JobHistory) -> Self {
        Self {
            observed_at: v.observed_at,
            sessions: v.sessions.into_iter().map(Into::into).collect(),
            events: v.events.into_iter().map(Into::into).collect(),
            buckets: v.buckets.into_iter().map(Into::into).collect(),
            truncated: v.truncated,
        }
    }
}
