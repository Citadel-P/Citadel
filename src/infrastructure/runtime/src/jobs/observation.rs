use super::{JobCatalog, JobOutcome, JobState, JobStatus, catalog::now};
use std::{future::Future, time::Duration};
use tokio::time::Instant;

tokio::task_local! { static CURRENT: JobHandle; }
#[derive(Clone)]
pub struct JobHandle {
    pub(super) catalog: JobCatalog,
    pub(super) id: &'static str,
}
impl JobHandle {
    /// Observation context scoped by the task owner; not inherited by detached tasks.
    pub fn current() -> Option<Self> {
        CURRENT.try_with(Clone::clone).ok()
    }
    pub async fn scope<F: Future>(&self, future: F) -> F::Output {
        CURRENT.scope(self.clone(), future).await
    }
    pub(super) fn update(&self, f: impl FnOnce(&mut JobStatus)) {
        self.update_if_changed(|status| {
            f(status);
            true
        });
    }
    fn update_if_changed(&self, f: impl FnOnce(&mut JobStatus) -> bool) {
        let mut state = self
            .catalog
            .0
            .state
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if !f(state.jobs.get_mut(self.id).expect("registered job")) {
            return;
        }
        super::history::observe_transition(&mut state, self.id);
        state.revision += 1;
        let revision = state.revision;
        drop(state);
        self.catalog.0.changes.send_replace(revision);
    }
    pub fn started(&self) {
        self.update(|s| {
            s.state = JobState::Unavailable;
            s.waiting_for = Some("Worker active; iteration timing unavailable".into());
        });
    }
    pub fn disabled(&self, reason: &'static str) {
        self.update(|s| {
            s.state = JobState::Disabled;
            s.waiting_for = Some(reason.into());
            s.next_check = None;
        });
    }
    pub fn budget(&self, duration: Duration) {
        self.update(|s| s.budget_ms = Some(millis(duration)));
    }
    pub fn progress(&self, count: u64, unit: &'static str) {
        if count == 0 {
            return;
        }
        self.update(|s| {
            s.last_progress_at = Some(now());
            if s.unit.as_deref().is_none_or(|u| u == unit) {
                s.unit = Some(unit.into());
                s.completed_units = Some(s.completed_units.unwrap_or(0).saturating_add(count));
            }
        });
    }
    pub fn stopped(&self, failed: bool) {
        self.update(|s| {
            s.state = if failed {
                JobState::Unavailable
            } else {
                JobState::Stopped
            };
            s.active_iterations = 0;
            s.active_operations = s.active_operations.map(|_| 0);
            s.current_started_at = None;
            s.next_check = None;
            s.waiting_for = None;
            if failed {
                if !matches!(
                    s.last_outcome,
                    Some(JobOutcome::Failed | JobOutcome::Interrupted)
                ) {
                    s.failures += 1;
                }
                s.last_outcome = Some(JobOutcome::Interrupted);
                s.last_failure_at = Some(now());
                s.last_error = Some("Worker exited unexpectedly; inspect Core logs.".into());
            }
        });
    }
    /// A wake helper can update its trigger without erasing the worker's budget.
    pub fn trigger(&self, description: impl Into<String>) {
        let description = description.into();
        self.update_if_changed(|s| {
            if s.trigger.as_ref() == Some(&description) {
                return false;
            }
            s.trigger = Some(description);
            true
        });
    }
    pub fn policy(&self, description: impl Into<String>, budget: Option<Duration>) {
        let description = description.into();
        let budget_ms = budget.map(millis);
        self.update_if_changed(|s| {
            if s.trigger.as_ref() == Some(&description) && s.budget_ms == budget_ms {
                return false;
            }
            s.trigger = Some(description);
            s.budget_ms = budget_ms;
            true
        });
    }

    pub fn waiting(&self, reason: &'static str, deadline: Option<Instant>) {
        let next = deadline
            .map(|d| now().saturating_add(millis(d.saturating_duration_since(Instant::now()))));
        self.update_if_changed(|s| {
            let state = if s.active_iterations > 0 {
                JobState::Running
            } else if s.last_outcome == Some(JobOutcome::Failed) {
                JobState::BackingOff
            } else if s.last_outcome == Some(JobOutcome::Interrupted) {
                JobState::Unavailable
            } else {
                JobState::Waiting
            };
            if s.waiting_for.as_deref() == Some(reason) && s.next_check == next && s.state == state
            {
                return false;
            }
            s.waiting_for = Some(reason.into());
            s.next_check = next;
            s.state = state;
            true
        });
    }

    pub fn active_operations(&self, count: usize) {
        self.update_if_changed(|s| {
            if s.active_operations == Some(count as u64) {
                return false;
            }
            s.active_operations = Some(count as u64);
            true
        });
    }
    pub fn backlog(&self, count: u64, lower_bound: bool) {
        self.update_if_changed(|s| {
            if s.backlog == Some(count) && s.backlog_is_lower_bound == lower_bound {
                return false;
            }
            s.backlog = Some(count);
            s.backlog_is_lower_bound = lower_bound;
            true
        });
    }
    pub fn begin(&self) -> JobIteration {
        self.update(|s| {
            s.state = JobState::Running;
            s.last_started_at = Some(now());
            if s.active_iterations == 0 {
                s.current_started_at = s.last_started_at;
            }
            s.active_iterations += 1;
            s.next_check = None;
            s.waiting_for = None;
        });
        JobIteration {
            handle: self.clone(),
            started: Instant::now(),
            finished: false,
            failure_context: None,
            failed_units: None,
        }
    }
    pub fn iteration() -> Option<JobIteration> {
        Self::current().map(|h| h.begin())
    }
}
pub struct JobIteration {
    handle: JobHandle,
    started: Instant,
    finished: bool,
    failure_context: Option<(&'static str, super::JobTarget)>,
    failed_units: Option<u64>,
}
impl JobIteration {
    /// Only fixed, developer-authored text and known identities enter diagnostics.
    pub fn failure_context(mut self, summary: &'static str, target: super::JobTarget) -> Self {
        self.failure_context = Some((summary, target));
        self
    }
    pub fn failed_units(mut self, count: u64) -> Self {
        self.failed_units = Some(count);
        self
    }
    pub fn finish(mut self, outcome: JobOutcome, work: Option<(u64, &'static str)>) {
        self.record(outcome, work);
        self.finished = true;
    }
    pub fn result<T, E>(self, result: &Result<T, E>) {
        self.finish(
            if result.is_ok() {
                JobOutcome::Completed
            } else {
                JobOutcome::Failed
            },
            None,
        );
    }
    fn record(&self, outcome: JobOutcome, work: Option<(u64, &'static str)>) {
        let elapsed = millis(self.started.elapsed());
        self.handle.update(|s| {
            s.active_iterations = s.active_iterations.saturating_sub(1);
            if s.active_iterations == 0 {
                s.current_started_at = None;
                s.state = match outcome {
                    JobOutcome::Failed => JobState::BackingOff,
                    JobOutcome::Interrupted => JobState::Unavailable,
                    _ => JobState::Waiting,
                };
            }
            s.last_completed_at = Some(now());
            s.last_outcome = Some(outcome);
            s.last_duration_ms = Some(elapsed);
            s.last_completed_units = work.map(|(count, _)| count);
            s.last_unit = work.map(|(_, unit)| unit.into());
            s.last_failed_units = self.failed_units;
            s.iterations += 1;
            s.total_duration_ms = s.total_duration_ms.saturating_add(elapsed);
            s.max_duration_ms = s.max_duration_ms.max(elapsed);
            match outcome {
                JobOutcome::Completed | JobOutcome::Productive | JobOutcome::NoWork => {
                    s.last_success_at = Some(now())
                }
                JobOutcome::Failed | JobOutcome::Interrupted => {
                    s.failures += 1;
                    s.last_failure_at = Some(now());
                    s.last_error = Some("Iteration failed; inspect Core logs for this job.".into());
                    s.last_problem = Some(super::JobProblem {
                        at: s.last_failure_at.unwrap(),
                        summary: self
                            .failure_context
                            .as_ref()
                            .map_or("The worker reported a failed check.", |(summary, _)| {
                                *summary
                            })
                            .into(),
                        target: self
                            .failure_context
                            .as_ref()
                            .map(|(_, target)| target.clone()),
                    });
                }
                JobOutcome::BudgetExhausted => s.budget_exhaustions += 1,
                _ => {}
            }
            if outcome == JobOutcome::Productive {
                s.productive += 1;
                s.productive_duration_ms = s.productive_duration_ms.saturating_add(elapsed);
                s.last_progress_at = Some(now());
            }
            if outcome == JobOutcome::NoWork {
                s.no_work += 1;
                s.no_work_duration_ms = s.no_work_duration_ms.saturating_add(elapsed);
            }
            if let Some((count, unit)) = work {
                if count > 0 {
                    s.last_progress_at = Some(now());
                }
                if s.unit.as_deref().is_none_or(|current| current == unit) {
                    s.unit = Some(unit.into());
                    s.completed_units = Some(s.completed_units.unwrap_or(0).saturating_add(count));
                }
            }
        });
    }
}
impl Drop for JobIteration {
    fn drop(&mut self) {
        if !self.finished {
            self.record(
                if std::thread::panicking() {
                    JobOutcome::Interrupted
                } else {
                    JobOutcome::Cancelled
                },
                None,
            );
        }
    }
}
fn millis(d: Duration) -> u64 {
    u64::try_from(d.as_millis()).unwrap_or(u64::MAX)
}

/// Explicitly group an owned child under its parent's row. Call before spawning;
/// the caller still owns cancellation and joining of this future.
pub fn scope_owned<F: Future>(future: F) -> impl Future<Output = F::Output> {
    let job = JobHandle::current();
    async move {
        match job {
            Some(job) => job.scope(future).await,
            None => future.await,
        }
    }
}
