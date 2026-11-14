use super::*;
use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Fixture {
    core: Mutex<Option<crate::service::installation::tests::Fixture>>,
    checks: Mutex<CheckState>,
    state: Mutex<AgentUpdateState>,
    targets: Mutex<Vec<AgentUpdateTarget>>,
    review: Mutex<Option<AgentUpdateReview>>,
    installs: Mutex<Vec<String>>,
    team: AtomicBool,
    admin: AtomicBool,
    fail: AtomicBool,
    expire_on_install: AtomicBool,
    cancel_on_install: AtomicBool,
    cancel_on_verify: AtomicBool,
    resume_fails: AtomicBool,
    cleanup_fails: AtomicBool,
    success_save_fails: AtomicBool,
    cleanup_save_fails: AtomicBool,
    cancel_on_cleanup: AtomicBool,
    cleanups: Mutex<Vec<String>>,
    claimed: Mutex<Option<DateTime<Utc>>>,
    reservation: Mutex<Option<Uuid>>,
    probes: Mutex<Vec<String>>,
    pause_probe: AtomicBool,
    probe_started: tokio::sync::Notify,
    continue_probe: tokio::sync::Notify,
    release_checks: AtomicUsize,
    availability_observations: Mutex<Vec<Vec<String>>>,
}
fn target(id: &str, version: &str) -> AgentUpdateTarget {
    AgentUpdateTarget {
        id: id.into(),
        resource_id: Uuid::now_v7(),
        kind: AgentUpdateKind::Platform,
        name: id.into(),
        installed_version: Some(version.into()),
        connected: true,
        supported: true,
        reason: None,
        nodes: 1,
        drain_nodes: vec![],
        revision: "source-container:image".into(),
    }
}
fn actor() -> UpdateActor {
    UpdateActor {
        subject_id: Uuid::from_u128(1),
        actor_id: ActorId::new(Uuid::from_u128(2)),
    }
}

#[tokio::test]
async fn no_agents_skips_release_verification_and_clears_irrelevant_errors() {
    let fixture = Fixture::new();
    fixture.targets.lock().unwrap().clear();
    fixture.state.lock().unwrap().check_error = Some("Previous release check failed.".into());
    let release = fixture.state.lock().unwrap().release.clone();
    let service = fixture.service();
    let token = CancellationToken::new();

    let (state, targets) = service.status(&token).await.unwrap();
    assert!(targets.is_empty());
    assert!(state.check_error.is_none());
    service.check(&token).await.unwrap();
    assert_eq!(fixture.release_checks.load(Ordering::SeqCst), 0);
    let state = fixture.state.lock().unwrap().clone();
    assert!(state.check_error.is_none());
    assert_eq!(state.release, release);
    assert_eq!(
        *fixture.availability_observations.lock().unwrap(),
        vec![Vec::<String>::new()]
    );

    // Adding an Agent makes a real verification failure relevant again.
    fixture
        .targets
        .lock()
        .unwrap()
        .push(target("new-agent", "0.1.1"));
    fixture.state.lock().unwrap().release = None;
    service.check(&token).await.unwrap();
    assert_eq!(fixture.release_checks.load(Ordering::SeqCst), 1);
    assert!(
        service
            .status(&token)
            .await
            .unwrap()
            .0
            .check_error
            .is_some()
    );
    assert_eq!(fixture.availability_observations.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn empty_agent_inventory_can_resolve_alerts_for_the_previous_core_release() {
    let fixture = Fixture::new();
    fixture.targets.lock().unwrap().clear();
    let mut service = fixture.service();
    service.core_version = "0.1.3".into();
    service.check(&CancellationToken::new()).await.unwrap();
    assert_eq!(fixture.release_checks.load(Ordering::SeqCst), 0);
    assert_eq!(
        *fixture.availability_observations.lock().unwrap(),
        vec![Vec::<String>::new()]
    );
}

impl Fixture {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            core: Mutex::new(None),
            checks: Mutex::new(CheckState::default()),
            state: Mutex::new(AgentUpdateState {
                release: Some(AgentUpdateRelease {
                    version: "0.1.2".into(),
                    image: format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "a".repeat(64)),
                }),
                check_error: None,
                schedule: UpdateSchedule::default(),
                operation: None,
                run: None,
            }),
            targets: Mutex::new(vec![
                target("A", "0.1.2"),
                target("B", "0.1.2"),
                target("C", "0.1.1"),
            ]),
            review: Mutex::new(None),
            installs: Mutex::new(vec![]),
            team: AtomicBool::new(false),
            admin: AtomicBool::new(true),
            fail: AtomicBool::new(false),
            expire_on_install: AtomicBool::new(false),
            cancel_on_install: AtomicBool::new(false),
            cancel_on_verify: AtomicBool::new(false),
            resume_fails: AtomicBool::new(false),
            cleanup_fails: AtomicBool::new(false),
            success_save_fails: AtomicBool::new(false),
            cleanup_save_fails: AtomicBool::new(false),
            cancel_on_cleanup: AtomicBool::new(false),
            cleanups: Mutex::new(vec![]),
            claimed: Mutex::new(None),
            reservation: Mutex::new(None),
            probes: Mutex::new(vec![]),
            pause_probe: AtomicBool::new(false),
            probe_started: tokio::sync::Notify::new(),
            continue_probe: tokio::sync::Notify::new(),
            release_checks: AtomicUsize::new(0),
            availability_observations: Mutex::new(vec![]),
        })
    }
    fn service(self: &Arc<Self>) -> AgentUpdateService {
        AgentUpdateService::new(
            "0.1.2".into(),
            self.clone(),
            self.clone(),
            self.clone(),
            Arc::new(NoopUpdateChangeNotifier),
        )
    }
    fn claim(
        &self,
        review: &AgentUpdateReview,
        key: Uuid,
        scheduled: bool,
    ) -> AgentUpdateOperation {
        let now = Utc::now();
        let op = AgentUpdateOperation {
            id: Uuid::now_v7(),
            request_id: key,
            review_id: review.id,
            actor_id: review.actor_id,
            scheduled,
            release: review.release.clone(),
            targets: review
                .targets
                .iter()
                .map(|t| AgentUpdateResult {
                    target: t.clone(),
                    phase: AgentUpdatePhase::Pending,
                    message: None,
                    cleanup_completed: false,
                })
                .collect(),
            revision: 0,
            created_at: now,
            updated_at: now,
            finished: false,
        };
        self.state.lock().unwrap().operation = Some(op.clone());
        op
    }
}
impl AgentUpdateEntitlements for Fixture {
    fn automated_operations(&self) -> BoxFuture<'_, Result<bool, UpdateError>> {
        Box::pin(async { Ok(self.team.load(Ordering::SeqCst)) })
    }
}
impl AgentUpdateRuntime for Fixture {
    fn cleanup<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let saved = self.state.lock().unwrap().operation.clone().unwrap();
            assert_eq!(saved.id, operation);
            assert!(!saved.finished, "ownership must survive cleanup");
            assert_eq!(
                saved
                    .targets
                    .iter()
                    .find(|t| t.target.id == target.id)
                    .unwrap()
                    .phase,
                AgentUpdatePhase::Succeeded,
                "success must be durable before deletion"
            );
            self.cleanups.lock().unwrap().push(target.id.clone());
            if self.cancel_on_cleanup.load(Ordering::SeqCst) {
                cancel.cancel();
            }
            if cancel.is_cancelled() {
                Err(UpdateError::Cancelled)
            } else if self.cleanup_fails.load(Ordering::SeqCst) {
                Err(UpdateError::Unavailable(
                    "Cleanup acknowledgement lost".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
    fn resume<'a>(
        &'a self,
        _: &'a AgentUpdateTarget,
        _: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            if cancel.is_cancelled() {
                Err(UpdateError::Cancelled)
            } else if self.resume_fails.load(Ordering::SeqCst) {
                Err(UpdateError::Unavailable(
                    "Node disconnected during resume".into(),
                ))
            } else {
                Ok(())
            }
        })
    }
    fn targets<'a>(
        &'a self,
        selected: Option<&'a [String]>,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<AgentUpdateTarget>, UpdateError>> {
        Box::pin(async move {
            if self.pause_probe.swap(false, Ordering::SeqCst) {
                self.probe_started.notify_one();
                self.continue_probe.notified().await;
            }
            let targets: Vec<_> = self
                .targets
                .lock()
                .unwrap()
                .iter()
                .filter(|t| selected.is_none_or(|ids| ids.contains(&t.id)))
                .cloned()
                .collect();
            self.probes
                .lock()
                .unwrap()
                .extend(targets.iter().map(|t| t.id.clone()));
            Ok(targets)
        })
    }
    fn release<'a>(
        &'a self,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentUpdateRelease, UpdateError>> {
        Box::pin(async {
            self.release_checks.fetch_add(1, Ordering::SeqCst);
            self.state
                .lock()
                .unwrap()
                .release
                .clone()
                .ok_or_else(|| UpdateError::Unavailable("Agent release is unavailable.".into()))
        })
    }
    fn install<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        _: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            self.installs.lock().unwrap().push(target.id.clone());
            if self.expire_on_install.load(Ordering::SeqCst) {
                self.team.store(false, Ordering::SeqCst);
            }
            if self.fail.load(Ordering::SeqCst) {
                return Err(UpdateError::Unavailable("replacement failed".into()));
            }
            for t in self
                .targets
                .lock()
                .unwrap()
                .iter_mut()
                .filter(|t| t.id == target.id)
            {
                t.installed_version = Some(release.version.clone());
            }
            if self.cancel_on_install.load(Ordering::SeqCst) {
                cancel.cancel();
                return Err(UpdateError::Unavailable(
                    "Connection closed during handoff".into(),
                ));
            }
            Ok(())
        })
    }
    fn verify<'a>(
        &'a self,
        target: &'a AgentUpdateTarget,
        release: &'a AgentUpdateRelease,
        _operation: Uuid,
        cancel: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            if self.cancel_on_verify.load(Ordering::SeqCst) {
                cancel.cancel();
                return Err(UpdateError::Cancelled);
            }
            Ok(self.targets.lock().unwrap().iter().any(|t| {
                t.id == target.id
                    && t.connected
                    && t.installed_version.as_ref() == Some(&release.version)
            }))
        })
    }
}
impl AgentUpdateRepository for Fixture {
    fn observe_agent_availability<'a>(
        &'a self,
        _: &'a AgentUpdateRelease,
        targets: &'a [AgentUpdateTarget],
        _: chrono::DateTime<chrono::Utc>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            self.availability_observations
                .lock()
                .unwrap()
                .push(targets.iter().map(|target| target.id.clone()).collect());
            Ok(())
        })
    }

    fn operation(&self, id: Uuid) -> BoxFuture<'_, Result<AgentUpdateOperation, UpdateError>> {
        Box::pin(async move {
            self.state
                .lock()
                .unwrap()
                .operation
                .clone()
                .filter(|op| op.id == id)
                .ok_or(UpdateError::NotFound)
        })
    }
    fn state(&self) -> BoxFuture<'_, Result<AgentUpdateState, UpdateError>> {
        Box::pin(async { Ok(self.state.lock().unwrap().clone()) })
    }
    fn save_release<'a>(
        &'a self,
        release: Option<&'a AgentUpdateRelease>,
        error: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            if let Some(r) = release {
                state.release = Some(r.clone());
            }
            state.check_error = error.map(Into::into);
            Ok(())
        })
    }
    fn save_schedule<'a>(
        &'a self,
        _: UpdateActor,
        schedule: &'a UpdateSchedule,
        _: bool,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            self.state.lock().unwrap().schedule = schedule.clone();
            *self.reservation.lock().unwrap() = None;
            Ok(())
        })
    }
    fn save_review<'a>(
        &'a self,
        _: UpdateActor,
        review: &'a AgentUpdateReview,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            *self.review.lock().unwrap() = Some(review.clone());
            Ok(())
        })
    }
    fn approve(
        &self,
        actor: UpdateActor,
        review: Uuid,
        request: Uuid,
    ) -> BoxFuture<'_, Result<AgentUpdateOperation, UpdateError>> {
        Box::pin(async move {
            self.authorize(actor.actor_id).await?;
            if let Some(op) = self.state.lock().unwrap().operation.clone() {
                if op.request_id == request
                    && op.review_id == review
                    && op.actor_id == actor.actor_id
                {
                    return Ok(op);
                }
                if !op.finished {
                    return Err(UpdateError::Conflict("busy".into()));
                }
            }
            let saved = self
                .review
                .lock()
                .unwrap()
                .clone()
                .filter(|r| r.id == review && r.expires_at > Utc::now())
                .ok_or(UpdateError::NotFound)?;
            Ok(self.claim(&saved, request, false))
        })
    }
    fn reserve_schedule<'a>(
        &'a self,
        schedule: &'a UpdateSchedule,
        minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<Option<UpdateScheduleRun>, UpdateError>> {
        Box::pin(async move {
            if self.state.lock().unwrap().schedule != *schedule {
                return Ok(None);
            }
            let mut last = self.claimed.lock().unwrap();
            if *last == Some(minute) {
                return Ok(None);
            }
            *last = Some(minute);
            let run = UpdateScheduleRun::new(schedule.clone(), minute);
            *self.reservation.lock().unwrap() = Some(run.id);
            self.state.lock().unwrap().run = Some(run.clone());
            Ok(Some(run))
        })
    }
    fn save_run<'a>(
        &'a self,
        run: &'a UpdateScheduleRun,
    ) -> BoxFuture<'a, Result<UpdateScheduleRun, UpdateError>> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            let previous = state.run.as_ref().ok_or(UpdateError::NotFound)?;
            if previous.id != run.id || previous.revision != run.revision {
                return Err(UpdateError::Conflict("stale run".into()));
            }
            let mut next = run.clone();
            next.revision += 1;
            state.run = Some(next.clone());
            Ok(next)
        })
    }
    fn claim_core<'a>(
        &'a self,
        run: &'a UpdateScheduleRun,
    ) -> BoxFuture<'a, Result<UpdateOperation, UpdateError>> {
        Box::pin(async move {
            let core = self
                .core
                .lock()
                .unwrap()
                .clone()
                .ok_or(UpdateError::NotFound)?;
            let mut core_state = core.state.lock().unwrap();
            assert!(
                core_state.operation.is_none(),
                "a claimed Core replacement must not be replayed"
            );
            let mut state = self.state.lock().unwrap();
            let saved = state.run.as_mut().unwrap();
            assert_eq!(saved.revision, run.revision);
            let op = UpdateOperation::approve(
                run.core_plan.clone().unwrap(),
                ActorId::new(Uuid::from_u128(99)),
                run.id,
                Utc::now(),
            )?;
            saved.core_operation = Some(op.id);
            saved.revision += 1;
            core_state.operation = Some(op.clone());
            core_state.events.push("scheduled-claim");
            Ok(op)
        })
    }
    fn claim_agents<'a>(
        &'a self,
        run: &'a UpdateScheduleRun,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let review = AgentUpdateReview {
                id: run.id,
                actor_id: actor().actor_id,
                release: run.release.clone().unwrap(),
                targets: run.targets.clone(),
                expires_at: Utc::now() + chrono::Duration::minutes(5),
            };
            let op = self.claim(&review, run.id, true);
            let mut next = run.clone();
            next.agent_operation = Some(op.id);
            next.revision += 1;
            self.state.lock().unwrap().run = Some(next);
            Ok(())
        })
    }
    fn authorize(&self, _: ActorId) -> BoxFuture<'_, Result<(), UpdateError>> {
        Box::pin(async {
            if self.admin.load(Ordering::SeqCst) {
                Ok(())
            } else {
                Err(UpdateError::Forbidden)
            }
        })
    }
    fn save_operation<'a>(
        &'a self,
        op: &'a AgentUpdateOperation,
    ) -> BoxFuture<'a, Result<AgentUpdateOperation, UpdateError>> {
        Box::pin(async move {
            if self.cleanup_save_fails.load(Ordering::SeqCst)
                && op.targets.iter().any(|t| t.cleanup_completed)
            {
                return Err(UpdateError::Unavailable(
                    "Cleanup acknowledgement commit failed".into(),
                ));
            }

            if self.success_save_fails.load(Ordering::SeqCst)
                && op
                    .targets
                    .iter()
                    .any(|t| t.phase == AgentUpdatePhase::Succeeded)
            {
                return Err(UpdateError::Unavailable("Success commit failed".into()));
            }

            let mut state = self.state.lock().unwrap();
            assert_eq!(state.operation.as_ref().unwrap().revision, op.revision);
            let mut op = op.clone();
            op.revision += 1;
            state.operation = Some(op.clone());
            Ok(op)
        })
    }
}
#[tokio::test]
async fn core_current_does_not_hide_one_outdated_agent_and_replay_does_not_duplicate() {
    let f = Fixture::new();
    let service = f.service();
    let cancel = CancellationToken::new();
    let (_, targets) = service.status(&cancel).await.unwrap();
    assert_eq!(
        targets
            .iter()
            .filter(|t| t.eligibility(f.state.lock().unwrap().release.as_ref())
                == AgentUpdateEligibility::Available)
            .count(),
        1
    );
    let review = service
        .review(actor(), &["C".into(), "C".into()], &cancel)
        .await
        .unwrap();
    assert_eq!(review.targets.len(), 1);
    let key = Uuid::now_v7();
    let op = service.approve(actor(), review.id, key).await.unwrap();
    assert_eq!(
        service.approve(actor(), review.id, key).await.unwrap().id,
        op.id
    );
    assert!(
        service
            .approve(actor(), review.id, Uuid::now_v7())
            .await
            .is_err()
    );
    service.tick(&cancel).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    assert!(f.state.lock().unwrap().operation.as_ref().unwrap().finished);
    assert!(
        service
            .status(&cancel)
            .await
            .unwrap()
            .1
            .iter()
            .all(|t| t.installed_version.as_deref() == Some("0.1.2"))
    );
}
#[tokio::test]
async fn changed_and_offline_targets_are_skipped_without_dispatch() {
    let f = Fixture::new();
    let service = f.service();
    let cancel = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into()], &cancel)
        .await
        .unwrap();
    service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    f.targets.lock().unwrap()[2].revision = "different installation".into();
    service.tick(&cancel).await.unwrap();
    assert!(f.installs.lock().unwrap().is_empty());
    assert_eq!(
        f.state.lock().unwrap().operation.as_ref().unwrap().targets[0].phase,
        AgentUpdatePhase::Skipped
    );
}
#[tokio::test]
async fn failure_stops_remaining_targets_and_interrupted_dispatch_is_never_repeated() {
    let f = Fixture::new();
    f.targets.lock().unwrap().push(target("D", "0.1.1"));
    let service = f.service();
    let cancel = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into(), "D".into()], &cancel)
        .await
        .unwrap();
    service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    f.fail.store(true, Ordering::SeqCst);
    service.tick(&cancel).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    let result = f.state.lock().unwrap().operation.clone().unwrap();
    assert_eq!(result.targets[1].phase, AgentUpdatePhase::NotStarted);
    {
        let mut state = f.state.lock().unwrap();
        let op = state.operation.as_mut().unwrap();
        op.finished = false;
        op.targets[0].phase = AgentUpdatePhase::Updating;
        op.targets[1].phase = AgentUpdatePhase::Pending;
    }
    service.tick(&cancel).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
}
#[tokio::test]
async fn community_manual_is_free_but_schedule_and_non_admin_starts_are_denied() {
    let f = Fixture::new();
    let service = f.service();
    let cancel = CancellationToken::new();
    let schedule = UpdateSchedule {
        include_core: false,
        targets: vec!["A".into()],
        enabled: true,
        cron: "* * * * *".into(),
        time_zone: "UTC".into(),
    };
    assert!(matches!(
        service
            .save_schedule(actor(), schedule.clone(), &cancel)
            .await,
        Err(UpdateError::Forbidden)
    ));
    f.state.lock().unwrap().schedule = schedule.clone();
    service.tick(&cancel).await.unwrap();
    assert!(f.state.lock().unwrap().operation.is_none());
    service
        .save_schedule(
            actor(),
            UpdateSchedule {
                include_core: false,
                enabled: false,
                ..schedule
            },
            &cancel,
        )
        .await
        .unwrap();
    f.admin.store(false, Ordering::SeqCst);
    assert!(matches!(
        service.review(actor(), &["C".into()], &cancel).await,
        Err(UpdateError::Forbidden)
    ));
}
#[tokio::test]
async fn schedule_can_include_current_agents_and_manual_selection_does_not_modify_it() {
    let f = Fixture::new();
    f.team.store(true, Ordering::SeqCst);
    let service = f.service();
    let cancel = CancellationToken::new();
    let schedule = UpdateSchedule {
        include_core: false,
        targets: vec!["A".into()],
        enabled: true,
        cron: "* * * * *".into(),
        time_zone: "UTC".into(),
    };
    service
        .save_schedule(actor(), schedule.clone(), &cancel)
        .await
        .unwrap();
    let review = service
        .review(actor(), &["C".into()], &cancel)
        .await
        .unwrap();
    service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    service.tick(&cancel).await.unwrap();
    assert_eq!(f.state.lock().unwrap().schedule, schedule);
    f.targets.lock().unwrap()[0].installed_version = Some("0.1.1".into());
    service.tick(&cancel).await.unwrap();
    service.tick(&cancel).await.unwrap();
    service.tick(&cancel).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C", "A"]);
}
#[test]
fn stale_versions_and_unsupported_methods_never_claim_an_eligible_update() {
    let release = AgentUpdateRelease {
        version: "0.1.2".into(),
        image: String::new(),
    };
    let mut t = target("one", "0.1.1");
    t.installed_version = None;
    assert_eq!(
        t.eligibility(Some(&release)),
        AgentUpdateEligibility::Unknown
    );
    t.installed_version = Some("0.1.1".into());
    t.supported = false;
    assert_eq!(
        t.eligibility(Some(&release)),
        AgentUpdateEligibility::ManualRequired
    );
    t.connected = false;
    assert_eq!(
        t.eligibility(Some(&release)),
        AgentUpdateEligibility::Offline
    );
}

#[tokio::test]
async fn aliases_share_one_target_and_offline_cache_does_not_override_live_identity() {
    let f = Fixture::new();
    let mut alias = f.targets.lock().unwrap()[2].clone();
    alias.resource_id = Uuid::now_v7();
    alias.kind = AgentUpdateKind::BuildPool;
    f.targets.lock().unwrap().push(alias.clone());
    let service = f.service();
    let cancel = CancellationToken::new();
    assert_eq!(service.status(&cancel).await.unwrap().1.len(), 3);
    alias.connected = false;
    alias.revision = "stale cached container".into();
    f.targets.lock().unwrap().push(alias);
    let review = service
        .review(actor(), &["C".into(), "C".into()], &cancel)
        .await
        .unwrap();
    assert_eq!(review.targets.len(), 1);
    service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    service.tick(&cancel).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
}
#[tokio::test]
async fn policy_and_authorization_are_checked_again_before_dispatch() {
    for policy in [false, true] {
        let f = Fixture::new();
        let cancel = CancellationToken::new();
        let service = f.service();
        let review = service
            .review(actor(), &["C".into()], &cancel)
            .await
            .unwrap();
        service
            .approve(actor(), review.id, Uuid::now_v7())
            .await
            .unwrap();
        let service = if policy {
            service.with_updates_allowed(false)
        } else {
            f.admin.store(false, Ordering::SeqCst);
            service
        };
        service.tick(&cancel).await.unwrap();
        assert!(f.installs.lock().unwrap().is_empty());
        assert_eq!(
            f.state.lock().unwrap().operation.as_ref().unwrap().targets[0].phase,
            AgentUpdatePhase::NotStarted
        );
    }
}

#[tokio::test]
async fn team_expiry_allows_started_target_to_finish_but_prevents_the_next() {
    let f = Fixture::new();
    f.team.store(true, Ordering::SeqCst);
    f.expire_on_install.store(true, Ordering::SeqCst);
    f.targets.lock().unwrap().push(target("D", "0.1.1"));
    let service = f.service();
    let cancel = CancellationToken::new();
    service
        .save_schedule(
            actor(),
            UpdateSchedule {
                include_core: false,
                enabled: true,
                targets: vec!["C".into(), "D".into()],
                cron: "* * * * *".into(),
                time_zone: "UTC".into(),
            },
            &cancel,
        )
        .await
        .unwrap();
    service.tick(&cancel).await.unwrap();
    service.tick(&cancel).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    let state = f.state.lock().unwrap();
    let operation = state.operation.as_ref().unwrap();
    assert_eq!(operation.targets[0].phase, AgentUpdatePhase::Succeeded);
    assert_eq!(operation.targets[1].phase, AgentUpdatePhase::NotStarted);
    assert!(operation.finished);
}

#[tokio::test]
async fn shutdown_after_dispatch_preserves_the_attempt_for_restart_reconciliation() {
    let f = Fixture::new();
    f.targets.lock().unwrap().push(target("D", "0.1.1"));
    let cancel = CancellationToken::new();
    let service = f.service();
    let review = service
        .review(actor(), &["C".into(), "D".into()], &cancel)
        .await
        .unwrap();
    let approved = service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    f.cancel_on_install.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.tick(&cancel).await,
        Err(UpdateError::Cancelled)
    ));
    let operation = service.operation(approved.id).await.unwrap();
    assert!(!operation.finished);
    assert_eq!(operation.targets[0].phase, AgentUpdatePhase::Updating);
    assert_eq!(operation.targets[1].phase, AgentUpdatePhase::Pending);

    // Even a final tick during shutdown must not rewrite Updating as NotStarted.
    assert!(matches!(
        service.tick(&cancel).await,
        Err(UpdateError::Cancelled)
    ));
    assert_eq!(service.operation(approved.id).await.unwrap(), operation);
    f.cancel_on_install.store(false, Ordering::SeqCst);
    f.service().tick(&CancellationToken::new()).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C", "D"]);
    let operation = service.operation(approved.id).await.unwrap();
    assert!(operation.finished);
    assert!(
        operation
            .targets
            .iter()
            .all(|t| t.phase == AgentUpdatePhase::Succeeded)
    );
}

#[tokio::test]
async fn cancelled_verification_keeps_the_dispatched_attempt_owned() {
    let f = Fixture::new();
    let service = f.service();
    let cancel = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into()], &cancel)
        .await
        .unwrap();
    let approved = service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    f.state.lock().unwrap().operation.as_mut().unwrap().targets[0].phase =
        AgentUpdatePhase::Updating;
    f.targets.lock().unwrap()[2].installed_version = Some("0.1.2".into());
    f.cancel_on_verify.store(true, Ordering::SeqCst);
    assert!(matches!(
        service.tick(&cancel).await,
        Err(UpdateError::Cancelled)
    ));
    let operation = service.operation(approved.id).await.unwrap();
    assert!(!operation.finished);
    assert_eq!(operation.targets[0].phase, AgentUpdatePhase::Updating);
    f.cancel_on_verify.store(false, Ordering::SeqCst);
    f.service().tick(&CancellationToken::new()).await.unwrap();
    assert!(service.operation(approved.id).await.unwrap().finished);
    assert!(f.installs.lock().unwrap().is_empty());
}

#[tokio::test]
async fn lost_resume_acknowledgement_retains_claim_until_reconnect_without_redispatch() {
    for failed_rollout in [false, true] {
        let f = Fixture::new();
        f.targets.lock().unwrap().push(target("D", "0.1.1"));
        f.fail.store(failed_rollout, Ordering::SeqCst);
        f.resume_fails.store(true, Ordering::SeqCst);
        let service = f.service();
        let cancel = CancellationToken::new();
        let review = service
            .review(actor(), &["C".into(), "D".into()], &cancel)
            .await
            .unwrap();
        let approved = service
            .approve(actor(), review.id, Uuid::now_v7())
            .await
            .unwrap();
        assert!(service.tick(&cancel).await.is_err());
        let operation = service.operation(approved.id).await.unwrap();
        assert!(!operation.finished);
        assert_eq!(operation.targets[0].phase, AgentUpdatePhase::Updating);
        assert_eq!(operation.targets[1].phase, AgentUpdatePhase::Pending);
        assert!(
            operation.targets[0]
                .message
                .as_deref()
                .unwrap()
                .contains("ownership is retained")
        );
        assert!(
            service
                .approve(actor(), review.id, Uuid::now_v7())
                .await
                .is_err()
        );
        assert!(f.service().tick(&cancel).await.is_err());
        assert_eq!(*f.installs.lock().unwrap(), ["C"]);
        f.resume_fails.store(false, Ordering::SeqCst);
        f.service().tick(&cancel).await.unwrap();
        let operation = service.operation(approved.id).await.unwrap();
        assert!(operation.finished);
        if failed_rollout {
            assert_eq!(*f.installs.lock().unwrap(), ["C"]);
            assert_eq!(operation.targets[0].phase, AgentUpdatePhase::Failed);
            assert_eq!(operation.targets[1].phase, AgentUpdatePhase::NotStarted);
        } else {
            assert_eq!(*f.installs.lock().unwrap(), ["C", "D"]);
            assert!(
                operation
                    .targets
                    .iter()
                    .all(|t| t.phase == AgentUpdatePhase::Succeeded)
            );
        }
    }
}

#[tokio::test]
async fn dispatch_probes_only_selected_agents_then_refreshes_availability() {
    let f = Fixture::new();
    f.targets.lock().unwrap().push(target("D", "0.1.1"));
    let service = f.service();
    let cancel = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into(), "D".into()], &cancel)
        .await
        .unwrap();
    service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    service.tick(&cancel).await.unwrap();
    // Preparation/dispatch stay scoped; the final inventory refresh updates the aggregate alert.
    assert_eq!(
        *f.probes.lock().unwrap(),
        ["C", "D", "C", "D", "A", "B", "C", "D"]
    );
    assert_eq!(*f.installs.lock().unwrap(), ["C", "D"]);
}

#[tokio::test]
async fn schedule_edits_after_claim_do_not_rewrite_the_frozen_selection() {
    let f = Fixture::new();
    f.team.store(true, Ordering::SeqCst);
    let schedule = UpdateSchedule {
        include_core: false,
        enabled: true,
        targets: vec!["C".into()],
        cron: "* * * * *".into(),
        time_zone: "UTC".into(),
    };
    f.state.lock().unwrap().schedule = schedule.clone();
    f.pause_probe.store(true, Ordering::SeqCst);
    let service = f.service();
    let worker = tokio::spawn(async move { service.tick(&CancellationToken::new()).await });
    f.probe_started.notified().await;
    assert!(
        f.reservation.lock().unwrap().is_some(),
        "occurrence is owned before any RPC"
    );
    f.save_schedule(
        actor(),
        &UpdateSchedule {
            include_core: false,
            enabled: false,
            ..schedule
        },
        true,
    )
    .await
    .unwrap();
    f.continue_probe.notify_one();
    worker.await.unwrap().unwrap();
    assert_eq!(
        f.state.lock().unwrap().operation.as_ref().unwrap().targets[0]
            .target
            .id,
        "C"
    );
    assert!(f.installs.lock().unwrap().is_empty());
}

impl ReleaseSource for Fixture {
    fn releases(&self, _: ReleaseChannel) -> BoxFuture<'_, Result<Vec<Release>, UpdateError>> {
        Box::pin(async {
            Ok(vec![Release {
                tag_name: "v0.1.2".into(),
                draft: false,
                prerelease: false,
            }])
        })
    }
}
impl ReleaseCheckRepository for Fixture {
    fn database_version(&self) -> BoxFuture<'_, Result<u32, UpdateError>> {
        Box::pin(async { Ok(180006) })
    }

    fn observe_core_availability<'a>(
        &'a self,
        _: &'a str,
        _: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async { Ok(()) })
    }

    fn checks(&self) -> BoxFuture<'_, Result<CheckState, UpdateError>> {
        Box::pin(async { Ok(self.checks.lock().unwrap().clone()) })
    }
    fn record_check<'a>(
        &'a self,
        result: Result<Option<&'a ReleaseNotice>, &'a str>,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            let mut checks = self.checks.lock().unwrap();
            checks.last_attempt = Some(now);
            match result {
                Ok(notice) => {
                    checks.available = notice.cloned();
                    checks.error = None;
                    checks.last_success = Some(now);
                }
                Err(error) => checks.error = Some(error.into()),
            }
            Ok(true)
        })
    }
}

fn scheduled_core_fixture() -> (
    Arc<Fixture>,
    crate::service::installation::tests::Fixture,
    AgentUpdateService,
    UpdateInstallService,
    tokio::sync::mpsc::Receiver<UpdateInstallCommand>,
) {
    let f = Fixture::new();
    f.team.store(true, Ordering::SeqCst);
    let core = crate::service::installation::tests::Fixture::new();
    *f.core.lock().unwrap() = Some(core.clone());
    f.state.lock().unwrap().schedule = UpdateSchedule {
        include_core: true,
        enabled: true,
        targets: vec!["C".into()],
        cron: "* * * * *".into(),
        time_zone: "UTC".into(),
    };
    let checker = ReleaseCheckService::new("0.1.1".into(), f.clone(), f.clone());
    let (installer, receiver) = core.service();
    let mut service = f.service().with_core(checker, Some(installer.clone()));
    service.core_version = "0.1.1".into();
    (f, core, service, installer, receiver)
}

#[tokio::test]
async fn combined_schedule_resumes_after_verified_core_cleanup_without_replaying_or_reselecting() {
    let (f, core, service, installer, mut receiver) = scheduled_core_fixture();
    let worker = tokio::spawn(async move {
        installer
            .execute(receiver.recv().await.unwrap(), &CancellationToken::new())
            .await
    });
    service.tick(&CancellationToken::new()).await.unwrap();
    worker.await.unwrap().unwrap();
    assert!(
        f.state.lock().unwrap().operation.is_none(),
        "no old-version Agent operation before restart"
    );
    assert!(f.installs.lock().unwrap().is_empty());
    assert!(
        !core.state.lock().unwrap().events.contains(&"review"),
        "no administrator impersonation"
    );
    let id = f.state.lock().unwrap().run.as_ref().unwrap().id;
    // A fresh coordinator sees the owned execution, even after schedule edits.
    f.state.lock().unwrap().schedule.targets = vec!["A".into()];
    f.state.lock().unwrap().schedule.enabled = false;
    let checker = ReleaseCheckService::new("0.1.2".into(), f.clone(), f.clone());
    let restarted = f.service().with_core(checker, Some(core.service().0));
    restarted.tick(&CancellationToken::new()).await.unwrap();
    assert!(
        f.state.lock().unwrap().operation.is_none(),
        "cleanup still owns Core"
    );
    {
        let mut state = core.state.lock().unwrap();
        state.operation.as_mut().unwrap().phase = UpdatePhase::Succeeded;
        state.released = true;
    }
    restarted.tick(&CancellationToken::new()).await.unwrap();
    let run = f.state.lock().unwrap().run.clone().unwrap();
    assert_eq!(run.id, id);
    assert_eq!(run.release.as_ref().unwrap().version, "0.1.2");
    assert!(run.agent_operation.is_some());
    restarted.tick(&CancellationToken::new()).await.unwrap();
    restarted.tick(&CancellationToken::new()).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    assert_eq!(
        f.state.lock().unwrap().run.as_ref().unwrap().phase,
        UpdateSchedulePhase::Finished
    );
    assert_eq!(
        core.state
            .lock()
            .unwrap()
            .events
            .iter()
            .filter(|event| **event == "scheduled-claim")
            .count(),
        1
    );
}

#[tokio::test]
async fn blocked_core_transition_stops_combined_occurrence_before_mutation() {
    let (f, core, service, _, _receiver) = scheduled_core_fixture();
    core.state.lock().unwrap().agent_drift = true;
    assert!(service.tick(&CancellationToken::new()).await.is_err());
    assert!(core.state.lock().unwrap().operation.is_none());
    assert!(f.state.lock().unwrap().operation.is_none());
    assert_eq!(
        f.state.lock().unwrap().run.as_ref().unwrap().phase,
        UpdateSchedulePhase::Finished
    );
}

#[tokio::test]
async fn core_failure_and_team_expiry_after_restart_prevent_the_agent_phase() {
    for failed in [false, true] {
        let (f, core, service, installer, mut receiver) = scheduled_core_fixture();
        let worker = tokio::spawn(async move {
            installer
                .execute(receiver.recv().await.unwrap(), &CancellationToken::new())
                .await
        });
        service.tick(&CancellationToken::new()).await.unwrap();
        worker.await.unwrap().unwrap();
        {
            let mut state = core.state.lock().unwrap();
            state.operation.as_mut().unwrap().phase = if failed {
                UpdatePhase::Failed
            } else {
                UpdatePhase::Succeeded
            };
            state.released = true;
        }
        if !failed {
            f.team.store(false, Ordering::SeqCst);
        }
        let checker = ReleaseCheckService::new("0.1.2".into(), f.clone(), f.clone());
        let restarted = f.service().with_core(checker, Some(core.service().0));
        restarted.tick(&CancellationToken::new()).await.unwrap();
        assert!(f.state.lock().unwrap().operation.is_none());
        assert_eq!(
            f.state.lock().unwrap().run.as_ref().unwrap().phase,
            UpdateSchedulePhase::Finished
        );
    }
}

#[tokio::test]
async fn current_core_proceeds_to_selected_agents_and_core_only_has_no_agent_execution() {
    let (f, core, _, _, _) = scheduled_core_fixture();
    let checker = ReleaseCheckService::new("0.1.2".into(), f.clone(), f.clone());
    let service = f.service().with_core(checker, Some(core.service().0));
    service.tick(&CancellationToken::new()).await.unwrap();
    service.tick(&CancellationToken::new()).await.unwrap();
    service.tick(&CancellationToken::new()).await.unwrap();
    assert!(core.state.lock().unwrap().operation.is_none());
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);

    let (f, core, service, installer, mut receiver) = scheduled_core_fixture();
    f.state.lock().unwrap().schedule.targets.clear();
    let worker = tokio::spawn(async move {
        installer
            .execute(receiver.recv().await.unwrap(), &CancellationToken::new())
            .await
    });
    service.tick(&CancellationToken::new()).await.unwrap();
    worker.await.unwrap().unwrap();
    {
        let mut state = core.state.lock().unwrap();
        state.operation.as_mut().unwrap().phase = UpdatePhase::Succeeded;
        state.released = true;
    }
    let restarted = f.service().with_core(
        ReleaseCheckService::new("0.1.2".into(), f.clone(), f.clone()),
        Some(core.service().0),
    );
    restarted.tick(&CancellationToken::new()).await.unwrap();
    assert!(f.state.lock().unwrap().operation.is_none());
    assert_eq!(
        f.state.lock().unwrap().run.as_ref().unwrap().phase,
        UpdateSchedulePhase::Finished
    );
}

#[tokio::test]
async fn agent_failure_after_core_success_keeps_core_and_stops_remaining_agents() {
    let (f, core, service, installer, mut receiver) = scheduled_core_fixture();
    f.targets.lock().unwrap().push(target("D", "0.1.1"));
    f.state.lock().unwrap().schedule.targets.push("D".into());
    let worker = tokio::spawn(async move {
        installer
            .execute(receiver.recv().await.unwrap(), &CancellationToken::new())
            .await
    });
    service.tick(&CancellationToken::new()).await.unwrap();
    worker.await.unwrap().unwrap();
    {
        let mut state = core.state.lock().unwrap();
        state.operation.as_mut().unwrap().phase = UpdatePhase::Succeeded;
        state.released = true;
    }
    let restarted = f.service().with_core(
        ReleaseCheckService::new("0.1.2".into(), f.clone(), f.clone()),
        Some(core.service().0),
    );
    restarted.tick(&CancellationToken::new()).await.unwrap();
    f.fail.store(true, Ordering::SeqCst);
    restarted.tick(&CancellationToken::new()).await.unwrap();
    restarted.tick(&CancellationToken::new()).await.unwrap();
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    assert_eq!(
        core.state.lock().unwrap().operation.as_ref().unwrap().phase,
        UpdatePhase::Succeeded
    );
    assert!(
        f.state
            .lock()
            .unwrap()
            .run
            .as_ref()
            .unwrap()
            .message
            .is_some()
    );
}

#[tokio::test]
async fn successful_update_cleanup_retries_after_restart_without_replacing_again() {
    let f = Fixture::new();
    f.targets.lock().unwrap().push(target("D", "0.1.1"));
    let service = f.service();
    let token = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into(), "D".into()], &token)
        .await
        .unwrap();
    let approved = service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    f.cleanup_fails.store(true, Ordering::SeqCst);
    assert!(service.tick(&token).await.is_err());
    let saved = service.operation(approved.id).await.unwrap();
    assert!(!saved.finished);
    assert_eq!(saved.targets[0].phase, AgentUpdatePhase::Succeeded);
    assert!(!saved.targets[0].cleanup_completed);
    assert!(
        saved.targets[0]
            .message
            .as_ref()
            .unwrap()
            .contains("retry automatically")
    );
    assert_eq!(saved.targets[1].phase, AgentUpdatePhase::Pending);
    assert!(
        service
            .approve(actor(), review.id, Uuid::now_v7())
            .await
            .is_err()
    );
    // Cleanup resumes using durable success; replacement verification may no longer
    // find a helper and must not be called again.
    f.cancel_on_verify.store(true, Ordering::SeqCst);
    assert!(f.service().tick(&token).await.is_err());
    assert!(!token.is_cancelled());
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    f.cleanup_fails.store(false, Ordering::SeqCst);
    f.service().tick(&token).await.unwrap();
    let saved = service.operation(approved.id).await.unwrap();
    assert!(saved.finished);
    assert!(
        saved
            .targets
            .iter()
            .all(|t| t.cleanup_completed && t.message.is_none())
    );
    assert_eq!(*f.installs.lock().unwrap(), ["C", "D"]);
    assert_eq!(*f.cleanups.lock().unwrap(), ["C", "C", "C", "D"]);
    // Crash after the cleanup acknowledgements commit but before operation finish:
    // a now-offline completed Agent must not block finishing or repeat cleanup.
    f.state.lock().unwrap().operation.as_mut().unwrap().finished = false;
    f.cleanup_fails.store(true, Ordering::SeqCst);
    f.service().tick(&token).await.unwrap();
    assert!(service.operation(approved.id).await.unwrap().finished);
    assert_eq!(*f.cleanups.lock().unwrap(), ["C", "C", "C", "D"]);
}

#[tokio::test]
async fn failed_success_commit_preserves_update_evidence_until_reconciliation() {
    let f = Fixture::new();
    let service = f.service();
    let token = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into()], &token)
        .await
        .unwrap();
    let approved = service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    f.success_save_fails.store(true, Ordering::SeqCst);
    assert!(service.tick(&token).await.is_err());
    let saved = service.operation(approved.id).await.unwrap();
    assert_eq!(saved.targets[0].phase, AgentUpdatePhase::Updating);
    assert!(!saved.finished);
    assert!(f.cleanups.lock().unwrap().is_empty());
    f.success_save_fails.store(false, Ordering::SeqCst);
    f.service().tick(&token).await.unwrap();
    assert!(service.operation(approved.id).await.unwrap().finished);
    assert_eq!(*f.installs.lock().unwrap(), ["C"]);
    assert_eq!(*f.cleanups.lock().unwrap(), ["C"]);
}

#[tokio::test]
async fn failed_rollout_retains_containers_for_recovery() {
    let f = Fixture::new();
    f.fail.store(true, Ordering::SeqCst);
    let service = f.service();
    let token = CancellationToken::new();
    let review = service
        .review(actor(), &["C".into()], &token)
        .await
        .unwrap();
    let approved = service
        .approve(actor(), review.id, Uuid::now_v7())
        .await
        .unwrap();
    service.tick(&token).await.unwrap();
    assert_eq!(
        service.operation(approved.id).await.unwrap().targets[0].phase,
        AgentUpdatePhase::Failed
    );
    assert!(f.cleanups.lock().unwrap().is_empty());
}

#[tokio::test]
async fn cleanup_interruption_before_acknowledgement_commit_preserves_durable_retry() {
    for cancellation in [false, true] {
        let f = Fixture::new();
        let service = f.service();
        let token = CancellationToken::new();
        let review = service
            .review(actor(), &["C".into()], &token)
            .await
            .unwrap();
        let approved = service
            .approve(actor(), review.id, Uuid::now_v7())
            .await
            .unwrap();
        f.cleanup_save_fails.store(!cancellation, Ordering::SeqCst);
        f.cancel_on_cleanup.store(cancellation, Ordering::SeqCst);
        assert!(service.tick(&token).await.is_err());
        let saved = service.operation(approved.id).await.unwrap();
        assert!(!saved.finished);
        assert_eq!(saved.targets[0].phase, AgentUpdatePhase::Succeeded);
        assert!(!saved.targets[0].cleanup_completed);
        f.cleanup_save_fails.store(false, Ordering::SeqCst);
        f.cancel_on_cleanup.store(false, Ordering::SeqCst);
        f.cancel_on_verify.store(true, Ordering::SeqCst);
        f.service().tick(&CancellationToken::new()).await.unwrap();
        assert!(service.operation(approved.id).await.unwrap().finished);
        assert_eq!(*f.installs.lock().unwrap(), ["C"]);
        assert_eq!(*f.cleanups.lock().unwrap(), ["C", "C"]);
    }
}
