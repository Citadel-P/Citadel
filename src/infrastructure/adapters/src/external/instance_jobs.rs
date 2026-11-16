//! Process observations only; no database or resource discovery.
use citadel_instance as f;
use citadel_runtime::jobs as r;
pub struct RuntimeInstanceJobs(pub r::JobCatalog);
impl f::InstanceJobsReader for RuntimeInstanceJobs {
    fn changes(&self) -> tokio::sync::watch::Receiver<u64> {
        self.0.subscribe()
    }
    fn snapshot(&self) -> f::JobsSnapshot {
        map_snapshot(self.0.snapshot())
    }
}
pub fn map_snapshot(value: r::JobsSnapshot) -> f::JobsSnapshot {
    f::JobsSnapshot {
        boot_id: value.boot_id,
        started_at: value.started_at,
        observed_at: value.observed_at,
        revision: value.revision,
        history_revision: value.history_revision,
        jobs: value
            .jobs
            .into_iter()
            .map(|s| f::JobStatus {
                id: s.id,
                name: s.name,
                resource_type: s
                    .resource_type
                    .parse()
                    .unwrap_or(citadel_primitives::JobCategory::Other),
                ownership: match s.ownership {
                    r::JobOwnership::ProcessLocal => f::JobOwnership::ProcessLocal,
                    r::JobOwnership::Coordinator => f::JobOwnership::Coordinator,
                    r::JobOwnership::TargetSession => f::JobOwnership::TargetSession,
                },
                state: match s.state {
                    r::JobState::Disabled => f::JobState::Disabled,
                    r::JobState::NotStarted => f::JobState::NotStarted,
                    r::JobState::Waiting => f::JobState::Waiting,
                    r::JobState::Running => f::JobState::Running,
                    r::JobState::BackingOff => f::JobState::BackingOff,
                    r::JobState::Stopped => f::JobState::Stopped,
                    r::JobState::Unavailable => f::JobState::Unavailable,
                },
                trigger: s.trigger,
                waiting_for: s.waiting_for,
                budget_ms: s.budget_ms,
                next_check: s.next_check,
                active_iterations: s.active_iterations,
                active_operations: s.active_operations,
                current_started_at: s.current_started_at,
                last_started_at: s.last_started_at,
                last_completed_at: s.last_completed_at,
                last_success_at: s.last_success_at,
                last_progress_at: s.last_progress_at,
                last_failure_at: s.last_failure_at,
                last_error: s.last_error,
                last_outcome: s.last_outcome.map(|v| match v {
                    r::JobOutcome::Completed => f::JobOutcome::Completed,
                    r::JobOutcome::Productive => f::JobOutcome::Productive,
                    r::JobOutcome::NoWork => f::JobOutcome::NoWork,
                    r::JobOutcome::BudgetExhausted => f::JobOutcome::BudgetExhausted,
                    r::JobOutcome::Failed => f::JobOutcome::Failed,
                    r::JobOutcome::Cancelled => f::JobOutcome::Cancelled,
                    r::JobOutcome::Interrupted => f::JobOutcome::Interrupted,
                }),
                last_duration_ms: s.last_duration_ms,
                last_completed_units: s.last_completed_units,
                last_unit: s.last_unit,
                last_failed_units: s.last_failed_units,
                last_problem: s.last_problem.map(map_problem),
                iterations: s.iterations,
                productive: s.productive,
                no_work: s.no_work,
                failures: s.failures,
                budget_exhaustions: s.budget_exhaustions,
                total_duration_ms: s.total_duration_ms,
                productive_duration_ms: s.productive_duration_ms,
                no_work_duration_ms: s.no_work_duration_ms,
                max_duration_ms: s.max_duration_ms,
                completed_units: s.completed_units,
                unit: s.unit,
                backlog: s.backlog,
                backlog_is_lower_bound: s.backlog_is_lower_bound,
            })
            .collect(),
    }
}

impl f::JobDiagnosticSource for RuntimeInstanceJobs {
    fn capture(&self, after: u64) -> f::JobDiagnosticCapture {
        let c = self.0.diagnostic_capture(after);
        f::JobDiagnosticCapture {
            snapshot: map_snapshot(c.snapshot),
            events: c
                .events
                .into_iter()
                .map(|e| f::JobHistoryEvent {
                    sequence: e.sequence,
                    job_id: e.job_id,
                    at: e.at,
                    kind: e.kind,
                    problem: e.problem.map(map_problem),
                })
                .collect(),
            through: c.through,
            lost_events: c.lost_events,
        }
    }
    fn committed(&self) {
        self.0.history_committed();
    }
}
pub fn map_problem(p: r::JobProblem) -> f::JobProblem {
    f::JobProblem {
        at: p.at,
        summary: p.summary,
        target: p.target.and_then(|t| {
            Some(f::JobTarget {
                resource_type: serde_json::from_value(serde_json::Value::String(
                    t.resource_type.into_owned(),
                ))
                .ok()?,
                resource_id: t.resource_id,
                operation_id: t.operation_id,
                container_id: t.container_id,
            })
        }),
    }
}
/// A diagnostic identity follows this Core's data volume, independently of its boot.
pub fn diagnostic_core_id(root: &std::path::Path) -> std::io::Result<uuid::Uuid> {
    use std::io::Write;
    let path = root.join("core-diagnostics-id");
    if !path.exists() {
        let id = uuid::Uuid::now_v7();
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(mut file) => {
                file.write_all(id.to_string().as_bytes())?;
                file.sync_all()?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e),
        }
    }
    let value = std::fs::read_to_string(path)?;
    uuid::Uuid::parse_str(value.trim()).map_err(std::io::Error::other)
}
