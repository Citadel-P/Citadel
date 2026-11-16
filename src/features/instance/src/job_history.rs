use crate::{InstanceError, JobsSnapshot};
use futures_util::future::BoxFuture;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryEvent {
    pub sequence: u64,
    pub job_id: String,
    pub at: u64,
    pub kind: String,
    #[serde(default)]
    pub problem: Option<crate::JobProblem>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistorySession {
    pub core_id: Uuid,
    pub version: String,
    pub snapshot: JobsSnapshot,
    pub stopped_at: Option<u64>,
    pub heartbeat_at: u64,
    pub lost_events: u64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryRecord {
    pub boot_id: Uuid,
    pub event: JobHistoryEvent,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistoryBucket {
    pub boot_id: Uuid,
    pub job_id: String,
    pub at: u64,
    pub through: u64,
    #[serde(default = "five_minutes")]
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
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobHistory {
    pub observed_at: u64,
    pub sessions: Vec<JobHistorySession>,
    pub events: Vec<JobHistoryRecord>,
    pub buckets: Vec<JobHistoryBucket>,
    pub truncated: bool,
}
pub struct JobDiagnosticCapture {
    pub snapshot: JobsSnapshot,
    pub events: Vec<JobHistoryEvent>,
    pub through: u64,
    pub lost_events: u64,
}
pub trait JobDiagnosticSource: Send + Sync {
    fn capture(&self, after: u64) -> JobDiagnosticCapture;
    fn committed(&self);
}
pub struct JobHistoryBatch {
    pub sequence: u64,
    pub session: JobHistorySession,
    pub events: Vec<JobHistoryEvent>,
    pub buckets: Vec<JobHistoryBucket>,
    pub checkpoint: bool,
}
fn five_minutes() -> u64 {
    300_000
}

/// Bounded history maintenance, independent of diagnostic ingestion.
pub trait JobHistoryMaintenance: Send + Sync {
    fn maintain(&self) -> BoxFuture<'_, Result<u64, InstanceError>>;
}

pub trait JobHistoryRepository: Send + Sync {
    fn persist<'a>(
        &'a self,
        batch: &'a JobHistoryBatch,
    ) -> BoxFuture<'a, Result<(), InstanceError>>;
    fn read<'a>(
        &'a self,
        job: Option<&'a str>,
        hours: u32,
    ) -> BoxFuture<'a, Result<JobHistory, InstanceError>>;
}

/// Flush-window deltas, attributed to completion time, never CPU measurements.
pub fn history_buckets(
    previous: Option<&JobsSnapshot>,
    current: &JobsSnapshot,
) -> Vec<JobHistoryBucket> {
    current
        .jobs
        .iter()
        .filter_map(|job| {
            let old = previous.and_then(|p| p.jobs.iter().find(|j| j.id == job.id));
            let delta = |now: u64, before: Option<u64>| now.saturating_sub(before.unwrap_or(0));
            let iterations = delta(job.iterations, old.map(|j| j.iterations));
            if iterations == 0 {
                return None;
            }
            Some(JobHistoryBucket {
                boot_id: current.boot_id,
                job_id: job.id.clone(),
                at: current.observed_at / 300_000 * 300_000,
                through: current.observed_at,
                resolution_ms: 300_000,
                iterations,
                productive: delta(job.productive, old.map(|j| j.productive)),
                no_work: delta(job.no_work, old.map(|j| j.no_work)),
                failures: delta(job.failures, old.map(|j| j.failures)),
                elapsed_ms: delta(job.total_duration_ms, old.map(|j| j.total_duration_ms)),
                productive_ms: delta(
                    job.productive_duration_ms,
                    old.map(|j| j.productive_duration_ms),
                ),
                no_work_ms: delta(job.no_work_duration_ms, old.map(|j| j.no_work_duration_ms)),
                units: job
                    .completed_units
                    .map(|v| delta(v, old.and_then(|j| j.completed_units))),
                unit: job.unit.clone(),
                backlog: job.backlog,
                backlog_is_lower_bound: job.backlog_is_lower_bound,
                gap: current
                    .observed_at
                    .saturating_sub(previous.map_or(current.started_at, |p| p.observed_at))
                    > 90_000,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn bucket_deltas_separate_empty_checks_and_mark_missing_intervals() {
        let mut current: JobsSnapshot = serde_json::from_value(serde_json::json!({
            "bootId": Uuid::now_v7(), "startedAt": 0, "observedAt": 600_000, "revision": 1,
            "jobs": [{"id":"test","name":"Test","resourceType":"Core","ownership":"processLocal","state":"waiting", "activeIterations":0,"iterations":10,"productive":2,"noWork":8,"failures":0,"budgetExhaustions":0,"totalDurationMs":100,"productiveDurationMs":80,"noWorkDurationMs":20,"maxDurationMs":50,"completedUnits":100,"backlogIsLowerBound":false}]
        })).unwrap();
        let baseline = current.clone();
        current.observed_at += 30_000;
        current.jobs[0].iterations += 1;
        current.jobs[0].no_work += 1;
        current.jobs[0].no_work_duration_ms += 3;
        current.jobs[0].total_duration_ms += 3;
        let bucket = history_buckets(Some(&baseline), &current).remove(0);
        assert_eq!(bucket.iterations, 1);
        assert_eq!(bucket.productive_ms, 0);
        assert_eq!(bucket.no_work_ms, 3);
        assert_eq!(bucket.units, Some(0));
        assert!(!bucket.gap);
        current.observed_at += 100_000;
        assert!(history_buckets(Some(&baseline), &current)[0].gap);
        assert!(history_buckets(Some(&current), &current).is_empty());
    }
}
