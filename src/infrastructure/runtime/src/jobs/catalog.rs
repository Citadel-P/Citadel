use super::{JobHandle, JobOwnership, JobState, JobStatus, JobsSnapshot};
use std::{
    collections::BTreeMap,
    sync::{Arc, Mutex},
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::sync::watch;
use uuid::Uuid;

pub(super) fn now() -> u64 {
    u64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis(),
    )
    .unwrap_or(u64::MAX)
}
#[derive(Clone)]
pub struct JobCatalog(pub(super) Arc<Inner>);
pub(super) struct Inner {
    pub boot_id: Uuid,
    pub started_at: u64,
    pub state: Mutex<State>,
    pub changes: watch::Sender<u64>,
}
pub(super) struct State {
    pub jobs: BTreeMap<&'static str, JobStatus>,
    pub revision: u64,
    pub history_revision: u64,
    pub event_sequence: u64,
    pub events: std::collections::VecDeque<super::JobEvent>,
    pub failing: std::collections::BTreeSet<&'static str>,
}
impl JobCatalog {
    pub fn new(boot_id: Uuid) -> Self {
        Self(Arc::new(Inner {
            boot_id,
            started_at: now(),
            state: Mutex::new(State {
                jobs: BTreeMap::new(),
                revision: 0,
                history_revision: 0,
                event_sequence: 0,
                events: Default::default(),
                failing: Default::default(),
            }),
            changes: watch::channel(0).0,
        }))
    }
    pub fn subscribe(&self) -> watch::Receiver<u64> {
        self.0.changes.subscribe()
    }
    pub fn snapshot(&self) -> JobsSnapshot {
        let state = self
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        JobsSnapshot {
            boot_id: self.0.boot_id,
            started_at: self.0.started_at,
            observed_at: now(),
            revision: state.revision,
            history_revision: state.history_revision,
            jobs: state.jobs.values().cloned().collect(),
        }
    }
    /// IDs are static registrations, never resource IDs or caller input.
    pub fn register(
        &self,
        id: &'static str,
        resource_type: impl Into<&'static str>,
        ownership: JobOwnership,
    ) -> JobHandle {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        assert!(
            !state.jobs.contains_key(id),
            "duplicate job registration: {id}"
        );
        assert!(
            state.jobs.len() < 128,
            "logical job catalog capacity exceeded"
        );
        let name = id.replace('-', " ");
        state.jobs.insert(
            id,
            JobStatus {
                id: id.into(),
                name,
                ownership,
                resource_type: std::borrow::Cow::Borrowed(resource_type.into()),
                state: JobState::NotStarted,
                trigger: None,
                waiting_for: None,
                budget_ms: None,
                next_check: None,
                active_iterations: 0,
                active_operations: None,
                current_started_at: None,
                last_started_at: None,
                last_completed_at: None,
                last_success_at: None,
                last_progress_at: None,
                last_failure_at: None,
                last_error: None,
                last_outcome: None,
                last_duration_ms: None,
                last_completed_units: None,
                last_unit: None,
                last_failed_units: None,
                last_problem: None,
                iterations: 0,
                productive: 0,
                no_work: 0,
                failures: 0,
                budget_exhaustions: 0,
                total_duration_ms: 0,
                productive_duration_ms: 0,
                no_work_duration_ms: 0,
                max_duration_ms: 0,
                completed_units: None,
                unit: None,
                backlog: None,
                backlog_is_lower_bound: false,
            },
        );
        state.revision += 1;
        let revision = state.revision;
        drop(state);
        self.0.changes.send_replace(revision);
        JobHandle {
            catalog: self.clone(),
            id,
        }
    }
}
