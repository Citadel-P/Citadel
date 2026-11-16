use super::*;
use std::time::Duration;

#[test]
fn resource_classification_is_registration_metadata_not_an_id_convention() {
    let catalog = JobCatalog::new(uuid::Uuid::now_v7());
    catalog.register("build-looking-id", "Maintenance", JobOwnership::Coordinator);
    catalog.register(
        "renamed-worker",
        "SwarmService",
        JobOwnership::TargetSession,
    );
    let snapshot = catalog.snapshot();
    assert_eq!(snapshot.jobs[0].resource_type, "Maintenance");
    assert_eq!(snapshot.jobs[1].resource_type, "SwarmService");
}

#[tokio::test(start_paused = true)]
async fn concurrent_iterations_and_cancellation_are_truthful() {
    let jobs = JobCatalog::new(uuid::Uuid::now_v7());
    let handle = jobs.register("fixture", "Other", JobOwnership::Coordinator);
    handle.started();
    let first = handle.begin();
    let second = handle.begin();
    tokio::time::advance(Duration::from_millis(25)).await;
    first.finish(JobOutcome::Productive, Some((3, "rows")));
    let state = jobs.snapshot().jobs.remove(0);
    assert_eq!(state.state, JobState::Running);
    assert_eq!(state.active_iterations, 1);
    assert_eq!(state.completed_units, Some(3));
    drop(second);
    let state = jobs.snapshot().jobs.remove(0);
    assert_eq!(state.state, JobState::Waiting);
    assert_eq!(state.last_outcome, Some(JobOutcome::Cancelled));
    assert_eq!(state.productive, 1);
    assert_eq!(state.no_work, 0);
    assert_eq!(state.iterations, 2);
    assert_eq!(state.total_duration_ms, 50);
}
#[test]
fn failure_is_retained_after_success_and_budget_is_not_failure() {
    let jobs = JobCatalog::new(uuid::Uuid::now_v7());
    let handle = jobs.register("fixture", "Other", JobOwnership::Coordinator);
    handle.begin().finish(JobOutcome::Failed, None);
    handle.begin().finish(JobOutcome::BudgetExhausted, None);
    handle.begin().finish(JobOutcome::NoWork, Some((0, "rows")));
    let state = jobs.snapshot().jobs.remove(0);
    assert_eq!(state.failures, 1);
    assert_eq!(state.budget_exhaustions, 1);
    assert_eq!(state.no_work, 1);
    assert!(state.last_error.is_some());
    assert!(state.last_failure_at.is_some());
}
#[test]
#[should_panic(expected = "duplicate job registration")]
fn duplicate_registration_is_rejected() {
    let jobs = JobCatalog::new(uuid::Uuid::now_v7());
    jobs.register("fixture", "Other", JobOwnership::Coordinator);
    jobs.register("fixture", "Other", JobOwnership::Coordinator);
}
#[tokio::test]
async fn notifications_coalesce_without_subscribers_or_backpressure() {
    let jobs = JobCatalog::new(uuid::Uuid::now_v7());
    let handle = jobs.register("fixture", "Other", JobOwnership::ProcessLocal);
    for _ in 0..1000 {
        handle.begin().finish(JobOutcome::NoWork, None);
    }
    let mut changes = jobs.subscribe();
    for _ in 0..1000 {
        handle.begin().finish(JobOutcome::Completed, None);
    }
    changes.changed().await.unwrap();
    assert_eq!(jobs.snapshot().jobs[0].iterations, 2000);
    assert!(!changes.has_changed().unwrap());
}

#[tokio::test]
async fn owned_children_share_one_row_but_unscoped_tasks_do_not() {
    let jobs = JobCatalog::new(uuid::Uuid::now_v7());
    let job = jobs.register("sessions", "Other", JobOwnership::TargetSession);
    job.scope(async {
        assert!(
            tokio::spawn(async { JobHandle::current().is_none() })
                .await
                .unwrap()
        );
        let child = tokio::spawn(scope_owned(async {
            observe_work(async { Ok::<_, ()>(3) }, |n| Some((*n, "samples")))
                .await
                .unwrap();
        }));
        child.await.unwrap();
    })
    .await;
    assert_eq!(jobs.snapshot().jobs.len(), 1);
    assert_eq!(jobs.snapshot().jobs[0].completed_units, Some(3));
}

#[tokio::test]
async fn panic_and_shutdown_never_claim_success_and_clear_active_work() {
    for panic in [true, false] {
        let jobs = JobCatalog::new(uuid::Uuid::now_v7());
        let cancel = tokio_util::sync::CancellationToken::new();
        let mut owner = crate::TaskSupervisor::new(cancel.clone()).with_jobs(jobs.clone());
        let token = cancel.clone();
        owner.spawn("fixture", "Other", async move {
            let job = JobHandle::current().unwrap();
            job.active_operations(2);
            let _iteration = job.begin();
            if panic {
                panic!("private error must not reach the snapshot");
            }
            token.cancelled().await;
            Ok::<_, std::convert::Infallible>(())
        });
        tokio::task::yield_now().await;
        if panic {
            assert!(owner.wait_for_exit().await.is_err());
        }
        owner.shutdown(Duration::from_secs(1)).await.unwrap();
        let snapshot = jobs.snapshot();
        let job = &snapshot.jobs[0];
        assert_eq!(job.active_iterations, 0);
        assert_eq!(job.active_operations, Some(0));
        assert_eq!(job.last_success_at, None);
        assert_ne!(job.state, JobState::Running);
        assert!(!format!("{snapshot:?}").contains("private error"));
    }
}

#[tokio::test]
async fn unchanged_waiting_metadata_does_not_emit_fake_progress() {
    let catalog = JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register("listener", "Other", JobOwnership::ProcessLocal);
    job.started();
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::Unavailable);
    job.policy("Input or flush", Some(Duration::from_secs(5)));
    job.waiting("Waiting for input", None);
    job.active_operations(0);
    job.backlog(0, false);
    let changes = catalog.subscribe();
    for _ in 0..1000 {
        job.policy("Input or flush", Some(Duration::from_secs(5)));
        job.waiting("Waiting for input", None);
        job.active_operations(0);
        job.backlog(0, false);
    }
    assert!(!changes.has_changed().unwrap());
    job.trigger("Input or fallback");
    assert_eq!(catalog.snapshot().jobs[0].budget_ms, Some(5000));
    assert_eq!(catalog.snapshot().jobs[0].iterations, 0);
}

#[tokio::test]
async fn interrupted_owned_work_stays_failed_until_an_actual_recovery() {
    let catalog = JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register("sessions", "Other", JobOwnership::TargetSession);
    let task = job
        .scope(async {
            tokio::spawn(scope_owned(async {
                let _iteration = JobHandle::iteration();
                panic!("private transport details");
            }))
        })
        .await;
    assert!(task.await.unwrap_err().is_panic());
    job.waiting("Awaiting session retry", None);
    let failed = catalog.snapshot().jobs.remove(0);
    assert_eq!(failed.state, JobState::Unavailable);
    assert_eq!(failed.last_outcome, Some(JobOutcome::Interrupted));
    assert_eq!(failed.failures, 1);
    assert_eq!(failed.last_success_at, None);
    job.begin().finish(JobOutcome::Completed, None);
    let recovered = catalog.snapshot().jobs.remove(0);
    assert_eq!(recovered.state, JobState::Waiting);
    assert_eq!(recovered.last_outcome, Some(JobOutcome::Completed));
    assert!(recovered.last_failure_at.is_some());
    assert!(
        !recovered
            .last_error
            .unwrap()
            .contains("private transport details")
    );
}

#[tokio::test]
async fn lifecycle_and_retry_states_keep_their_distinct_meanings() {
    let catalog = JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register("fixture", "Other", JobOwnership::Coordinator);
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::NotStarted);
    job.started();
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::Unavailable);
    job.disabled("Feature disabled");
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::Disabled);
    job.waiting("Awaiting a check", None);
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::Waiting);
    let iteration = job.begin();
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::Running);
    iteration.finish(JobOutcome::Failed, None);
    job.waiting(
        "Retry after failure",
        Some(tokio::time::Instant::now() + Duration::from_secs(5)),
    );
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::BackingOff);
    assert!(catalog.snapshot().jobs[0].next_check.is_some());
    job.begin().finish(JobOutcome::NoWork, Some((0, "rows")));
    assert_eq!(catalog.snapshot().jobs[0].state, JobState::Waiting);
    job.waiting("Awaiting a check", None);
    job.stopped(false);
    let stopped = catalog.snapshot().jobs.remove(0);
    assert_eq!(stopped.state, JobState::Stopped);
    assert_eq!(stopped.waiting_for, None);
    assert_eq!(stopped.next_check, None);
}

#[test]
fn diagnostic_journal_retains_quick_recovery_and_reports_overflow() {
    let catalog = JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register("writer", "Core", JobOwnership::Coordinator);
    for _ in 0..300 {
        job.begin().finish(JobOutcome::Failed, None);
        job.begin().finish(JobOutcome::Failed, None); // same failure episode
        job.begin()
            .finish(JobOutcome::Productive, Some((3, "samples")));
    }
    let capture = catalog.diagnostic_capture(0);
    assert_eq!(capture.events.len(), 512);
    assert_eq!(capture.lost_events, 88);
    assert_eq!(capture.through, 600);
    assert_eq!(capture.events.last().unwrap().kind, "recovered");
    assert_eq!(capture.snapshot.jobs[0].failures, 600);
    assert_eq!(capture.snapshot.jobs[0].completed_units, Some(900));
    assert!(
        catalog
            .diagnostic_capture(capture.through)
            .events
            .is_empty()
    );
    let revision = catalog.snapshot().revision;
    catalog.history_committed();
    assert_eq!(catalog.snapshot().history_revision, 1);
    assert!(catalog.snapshot().revision > revision);
}
#[test]
fn last_check_work_and_problem_context_do_not_confuse_totals_or_recovery() {
    let catalog = super::JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register("collector", "Platform", super::JobOwnership::Coordinator);
    let target = super::JobTarget {
        resource_type: "Platform".into(),
        resource_id: uuid::Uuid::now_v7(),
        operation_id: None,
        container_id: None,
    };
    job.begin()
        .failure_context("Some measurements could not be collected.", target.clone())
        .failed_units(2)
        .finish(super::JobOutcome::Failed, Some((10, "samples")));
    let failure = catalog.snapshot().jobs.remove(0);
    assert_eq!(failure.last_completed_units, Some(10));
    assert_eq!(failure.last_failed_units, Some(2));
    assert_eq!(
        failure.last_problem.as_ref().unwrap().target.as_ref(),
        Some(&target)
    );
    assert_eq!(
        catalog.diagnostic_capture(0).events[0].problem,
        failure.last_problem
    );
    job.begin()
        .finish(super::JobOutcome::Productive, Some((12, "samples")));
    let next = catalog.snapshot().jobs.remove(0);
    assert_eq!(next.last_completed_units, Some(12));
    assert_eq!(next.completed_units, Some(22));
    assert_eq!(next.last_failed_units, None);
    assert_eq!(
        next.last_problem, failure.last_problem,
        "a later check does not resolve a resource problem"
    );
    job.begin().finish(super::JobOutcome::Failed, None);
    let unknown = catalog.snapshot().jobs.remove(0);
    assert!(
        unknown.last_problem.unwrap().target.is_none(),
        "an unknown failure must not reuse the earlier target"
    );
    assert_eq!(unknown.last_completed_units, None);
    assert_eq!(unknown.last_unit, None);
}

#[test]
fn operation_problems_keep_scheduler_measurements_unchanged_and_deduplicate() {
    let catalog = super::JobCatalog::new(uuid::Uuid::now_v7());
    let job = catalog.register("builds", "Build", super::JobOwnership::Coordinator);
    let target = super::JobTarget {
        resource_type: "Build".into(),
        resource_id: uuid::Uuid::now_v7(),
        operation_id: Some(uuid::Uuid::now_v7()),
        container_id: None,
    };
    job.begin()
        .finish(super::JobOutcome::Productive, Some((1, "claims")));
    let before = catalog.snapshot().jobs.remove(0);
    job.report_problem("The controller reported an error.", target.clone());
    job.report_problem("The controller reported an error.", target.clone());
    let after = catalog.snapshot().jobs.remove(0);
    assert_eq!(after.iterations, before.iterations);
    assert_eq!(after.total_duration_ms, before.total_duration_ms);
    assert_eq!(after.state, before.state);
    assert_eq!(after.failures, before.failures);
    let events = catalog.diagnostic_capture(0).events;
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "problem");
    assert_eq!(
        events[0].problem.as_ref().unwrap().target.as_ref(),
        Some(&target)
    );
    job.report_problem(
        "The controller reported an error.",
        super::JobTarget {
            operation_id: Some(uuid::Uuid::now_v7()),
            container_id: None,
            ..target.clone()
        },
    );
    assert_eq!(catalog.diagnostic_capture(0).events.len(), 2);
    job.report_problem("The controller reported an error.", target);
    assert_eq!(
        catalog.diagnostic_capture(0).events.len(),
        2,
        "interleaved reports for unchanged targets must not add an event every sweep"
    );
    job.stopped(true);
    let events = catalog.diagnostic_capture(0).events;
    assert_eq!(events.last().unwrap().kind, "failed");
    assert!(
        events.last().unwrap().problem.is_none(),
        "an unexpected exit must not borrow a previous operation's identity"
    );
}
