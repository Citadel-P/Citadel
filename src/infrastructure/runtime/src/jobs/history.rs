//! Loss-tolerant diagnostics. No execution decisions depend on this journal.
use super::{
    JobCatalog, JobOutcome, JobState, JobsSnapshot,
    catalog::{State, now},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobEvent {
    pub sequence: u64,
    pub job_id: String,
    pub at: u64,
    pub kind: String,
    #[serde(default)]
    pub problem: Option<super::JobProblem>,
}
pub struct DiagnosticCapture {
    pub snapshot: JobsSnapshot,
    pub events: Vec<JobEvent>,
    pub through: u64,
    pub lost_events: u64,
}
pub(super) fn observe_transition(state: &mut State, id: &'static str) {
    let job = &state.jobs[id];
    let failure = matches!(
        job.last_outcome,
        Some(JobOutcome::Failed | JobOutcome::Interrupted)
    );
    let recovered = matches!(
        job.last_outcome,
        Some(JobOutcome::Completed | JobOutcome::Productive | JobOutcome::NoWork)
    );
    let kind = if failure && state.failing.insert(id) {
        Some("failed")
    } else if recovered && state.failing.remove(id) {
        Some("recovered")
    } else if job.state == JobState::Stopped
        && !state
            .events
            .back()
            .is_some_and(|e| e.job_id == id && e.kind == "stopped")
    {
        Some("stopped")
    } else {
        None
    };
    if let Some(kind) = kind {
        // An unexpected worker exit has no new measured failure context. Its old
        // report remains in the snapshot, but must not be attributed to this exit.
        let problem = (kind == "failed" && job.last_outcome == Some(JobOutcome::Failed))
            .then(|| job.last_problem.clone())
            .flatten();
        push_event(state, id, kind, problem);
    }
}

fn push_event(state: &mut State, id: &'static str, kind: &str, problem: Option<super::JobProblem>) {
    state.event_sequence += 1;
    if state.events.len() == 512 {
        state.events.pop_front();
    }
    state.events.push_back(JobEvent {
        sequence: state.event_sequence,
        job_id: id.into(),
        at: now(),
        kind: kind.into(),
        problem,
    });
}

impl super::JobHandle {
    /// Report an owned operation's error without treating its execution as a scheduler check.
    /// This does not change check counters, timings, scheduling or operation state.
    pub fn report_problem(&self, summary: &'static str, target: super::JobTarget) {
        let mut state = self
            .catalog
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let repeated = state.jobs[self.id]
            .last_problem
            .as_ref()
            .is_some_and(|p| p.summary == summary && p.target.as_ref() == Some(&target))
            // A batch can alternate between several failing resources. Reuse the
            // bounded journal to avoid writing each unchanged problem every sweep.
            || state.events.iter().rev().filter(|event| event.job_id == self.id)
                .filter_map(|event| event.problem.as_ref())
                .find(|problem| problem.target.as_ref() == Some(&target))
                .is_some_and(|problem| problem.summary == summary);
        let problem = super::JobProblem {
            at: now(),
            summary: summary.into(),
            target: Some(target),
        };
        state
            .jobs
            .get_mut(self.id)
            .expect("registered job")
            .last_problem = Some(problem.clone());
        if !repeated {
            push_event(&mut state, self.id, "problem", Some(problem));
        }
        state.revision += 1;
        let revision = state.revision;
        drop(state);
        self.catalog.0.changes.send_replace(revision);
    }
}
impl JobCatalog {
    pub fn diagnostic_capture(&self, after: u64) -> DiagnosticCapture {
        let state = self
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let first = state
            .events
            .front()
            .map_or(state.event_sequence + 1, |e| e.sequence);
        DiagnosticCapture {
            snapshot: JobsSnapshot {
                boot_id: self.0.boot_id,
                started_at: self.0.started_at,
                observed_at: now(),
                revision: state.revision,
                history_revision: state.history_revision,
                jobs: state.jobs.values().cloned().collect(),
            },
            events: state
                .events
                .iter()
                .filter(|e| e.sequence > after)
                .cloned()
                .collect(),
            through: state.event_sequence,
            lost_events: first.saturating_sub(after.saturating_add(1)),
        }
    }
    /// Called only after a diagnostic batch commits. Wakes existing realtime subscribers.
    pub fn history_committed(&self) {
        let mut state = self
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        state.history_revision += 1;
        state.revision += 1;
        let revision = state.revision;
        drop(state);
        self.0.changes.send_replace(revision);
    }
}
