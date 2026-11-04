use super::*;
use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, Ordering},
};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

struct Fixture {
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
    claimed: Mutex<Option<DateTime<Utc>>>,
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
impl Fixture {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Mutex::new(AgentUpdateState {
                release: Some(AgentUpdateRelease {
                    version: "0.1.2".into(),
                    image: format!("ghcr.io/citadel-p/citadel.agent@sha256:{}", "a".repeat(64)),
                }),
                check_error: None,
                schedule: AgentUpdateSchedule::default(),
                operation: None,
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
            claimed: Mutex::new(None),
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
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Vec<AgentUpdateTarget>, UpdateError>> {
        Box::pin(async { Ok(self.targets.lock().unwrap().clone()) })
    }
    fn release<'a>(
        &'a self,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<AgentUpdateRelease, UpdateError>> {
        Box::pin(async { Ok(self.state.lock().unwrap().release.clone().unwrap()) })
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
        schedule: &'a AgentUpdateSchedule,
        _: bool,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            self.state.lock().unwrap().schedule = schedule.clone();
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
    fn scheduled<'a>(
        &'a self,
        schedule: &'a AgentUpdateSchedule,
        review: &'a AgentUpdateReview,
        minute: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async move {
            if self.state.lock().unwrap().schedule != *schedule {
                return Ok(false);
            }
            let mut last = self.claimed.lock().unwrap();
            if *last == Some(minute) {
                return Ok(false);
            }
            *last = Some(minute);
            if review
                .targets
                .iter()
                .all(|t| t.eligibility(Some(&review.release)) == AgentUpdateEligibility::Current)
            {
                return Ok(false);
            }
            self.claim(review, Uuid::now_v7(), true);
            Ok(true)
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
    let schedule = AgentUpdateSchedule {
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
            AgentUpdateSchedule {
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
    let schedule = AgentUpdateSchedule {
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
            AgentUpdateSchedule {
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
                .contains("Cleanup will retry")
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
