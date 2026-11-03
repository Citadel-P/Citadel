use crate::*;
use chrono::{DateTime, Utc};
use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

#[derive(Default)]
struct State {
    operation: Option<UpdateOperation>,
    events: Vec<&'static str>,
    denied: bool,
    drift: bool,
    fail_ack: bool,
    fail_grant: bool,
    busy: bool,
}
#[derive(Clone)]
struct Fixture {
    plan: UpdatePlan,
    state: Arc<Mutex<State>>,
}
impl Fixture {
    fn new() -> Self {
        let now = Utc::now();
        let source = CoreIdentity {
            version: "0.1.1".into(),
            image: format!("{CORE_IMAGE}@sha256:{}", "a".repeat(64)),
            architecture: "amd64".into(),
            catalog_sha256: "b".repeat(64),
        };
        Self {
            state: Default::default(),
            plan: UpdatePlan {
                id: Uuid::now_v7(),
                instance_id: Uuid::now_v7(),
                target: CoreIdentity {
                    version: "0.1.2".into(),
                    image: format!("{CORE_IMAGE}@sha256:{}", "c".repeat(64)),
                    ..source.clone()
                },
                helper_image: source.image.clone(),
                source,
                configuration_sha256: "d".repeat(64),
                registration_id: Uuid::now_v7(),
                reviewed_at: now,
                expires_at: now + chrono::Duration::minutes(10),
                checkpoint_directory: "/opt/update-state".into(),
                agent_requirements: vec![],
            },
        }
    }
    fn snapshot(&self) -> InstallationSnapshot {
        InstallationSnapshot {
            instance_id: self.plan.instance_id,
            source: self.plan.source.clone(),
            registration_id: self.plan.registration_id,
            configuration_sha256: self.plan.configuration_sha256.clone(),
            checkpoint_directory: self.plan.checkpoint_directory.clone(),
            agent_versions: vec![],
        }
    }
    fn service(
        &self,
    ) -> (
        UpdateInstallService,
        tokio::sync::mpsc::Receiver<UpdateInstallCommand>,
    ) {
        let port = Arc::new(self.clone());
        UpdateInstallService::new(
            port.clone(),
            port.clone(),
            Arc::new(ReleaseInspectionService::new(port.clone(), port.clone())),
            UpdateDrainService::new(port.clone(), port.clone()),
            port,
        )
    }
}
impl ReleaseCheckRepository for Fixture {
    fn checks(&self) -> BoxFuture<'_, Result<CheckState, UpdateError>> {
        Box::pin(async { unreachable!() })
    }
    fn record_check<'a>(
        &'a self,
        _: Result<Option<&'a ReleaseNotice>, &'a str>,
        _: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<bool, UpdateError>> {
        Box::pin(async { unreachable!() })
    }
}
impl UpdateRepository for Fixture {
    fn save_review<'a>(
        &'a self,
        _: UpdateActor,
        _: &'a UpdatePlan,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async {
            self.state.lock().unwrap().events.push("review");
            Ok(())
        })
    }
    fn review(&self, _: UpdateActor, id: Uuid) -> BoxFuture<'_, Result<UpdatePlan, UpdateError>> {
        Box::pin(async move {
            assert_eq!(id, self.plan.id);
            Ok(self.plan.clone())
        })
    }
    fn previous(
        &self,
        _: UpdateActor,
        key: Uuid,
    ) -> BoxFuture<'_, Result<Option<UpdateOperation>, UpdateError>> {
        Box::pin(async move {
            Ok(self
                .state
                .lock()
                .unwrap()
                .operation
                .clone()
                .filter(|operation| operation.idempotency_key == key))
        })
    }
    fn claim<'a>(
        &'a self,
        actor: UpdateActor,
        plan: &'a UpdatePlan,
        key: Uuid,
        now: DateTime<Utc>,
    ) -> BoxFuture<'a, Result<(UpdateOperation, bool), UpdateError>> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            if state.denied {
                return Err(UpdateError::Forbidden);
            }
            if let Some(operation) = &state.operation {
                assert_eq!(operation.idempotency_key, key);
                return Ok((operation.clone(), false));
            }
            let operation = UpdateOperation::approve(plan.clone(), actor.actor_id, key, now)?;
            state.operation = Some(operation.clone());
            state.events.push("claim");
            Ok((operation, true))
        })
    }
    fn active(&self) -> BoxFuture<'_, Result<Option<UpdateOperation>, UpdateError>> {
        Box::pin(async { Ok(self.state.lock().unwrap().operation.clone()) })
    }
    fn operation(&self, _: Uuid) -> BoxFuture<'_, Result<UpdateOperation, UpdateError>> {
        Box::pin(async {
            self.state
                .lock()
                .unwrap()
                .operation
                .clone()
                .ok_or(UpdateError::NotFound)
        })
    }
    fn import<'a>(
        &'a self,
        journal: &'a UpdateJournal,
    ) -> BoxFuture<'a, Result<Option<CompletionReceipt>, UpdateError>> {
        Box::pin(async move {
            let mut state = self.state.lock().unwrap();
            journal.validate_for(state.operation.as_ref().unwrap())?;
            state.operation = Some(journal.operation.clone());
            state.events.push("import");
            Ok(None)
        })
    }
    fn release<'a>(
        &'a self,
        _: &'a UpdateJournal,
        _: &'a CoreIdentity,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async { panic!("execution cannot release maintenance") })
    }
}
impl UpdateInstallationRuntime for Fixture {
    fn source<'a>(
        &'a self,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<InstallationSnapshot, UpdateError>> {
        Box::pin(async {
            let mut state = self.state.lock().unwrap();
            state.events.push("source");
            let mut source = self.snapshot();
            if state.drift {
                source.configuration_sha256 = "f".repeat(64);
            }
            Ok(source)
        })
    }
    fn launch<'a>(
        &'a self,
        _: &'a UpdateOperation,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Arc<dyn UpdateHandoffRuntime>, UpdateError>> {
        Box::pin(async {
            self.state.lock().unwrap().events.push("launch");
            Ok(Arc::new(self.clone()) as Arc<dyn UpdateHandoffRuntime>)
        })
    }
}
impl UpdateHandoffRuntime for Fixture {
    fn acknowledged<'a>(
        &'a self,
        _: &'a UpdateOperation,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async {
            let mut state = self.state.lock().unwrap();
            state.events.push("ack");
            if state.fail_ack {
                Err(UpdateError::Unavailable("missing helper".into()))
            } else {
                Ok(())
            }
        })
    }
    fn journal<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<UpdateJournal, UpdateError>> {
        Box::pin(async move {
            let mut current = operation.clone();
            current.advance(0, UpdatePhase::Draining, Utc::now())?;
            Ok(UpdateJournal {
                protocol: UPDATE_PROTOCOL,
                history: vec![operation.clone(), current.clone()],
                operation: current,
                helper_container_id: "a".repeat(64),
                verified_target: None,
                completion_acknowledged: false,
                maintenance_released: false,
            })
        })
    }
    fn authorize_shutdown<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            assert_eq!(operation.phase, UpdatePhase::Preparing);
            let mut state = self.state.lock().unwrap();
            assert_eq!(
                state.operation.as_ref().unwrap().phase,
                UpdatePhase::Draining
            );
            state.events.push("grant");
            if state.fail_grant {
                Err(UpdateError::Unavailable("helper disappeared".into()))
            } else {
                Ok(())
            }
        })
    }
}
impl UpdateDrainRepository for Fixture {
    fn blockers<'a>(
        &'a self,
        operation: &'a UpdateOperation,
    ) -> BoxFuture<'a, Result<Vec<UpdateDrainBlocker>, UpdateError>> {
        Box::pin(async move {
            assert_eq!(
                self.state.lock().unwrap().operation.as_ref(),
                Some(operation)
            );
            Ok(if self.state.lock().unwrap().busy {
                vec![UpdateDrainBlocker {
                    kind: "remote cleanup".into(),
                    count: 1,
                }]
            } else {
                vec![]
            })
        })
    }
}
impl UpdateDrainRuntime for Fixture {
    fn close(&self, _: Uuid) -> Result<(), UpdateError> {
        self.state.lock().unwrap().events.push("close");
        Ok(())
    }
    fn blockers(&self, _: Uuid) -> Result<Vec<UpdateDrainBlocker>, UpdateError> {
        Ok(vec![])
    }
}
impl UpdateChangeNotifier for Fixture {
    fn changed(&self) {
        let mut state = self.state.lock().unwrap();
        assert!(state.operation.is_some());
        state.events.push("notify");
    }
}
impl ReleaseArtifactSource for Fixture {
    fn verify<'a>(
        &'a self,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<VerifiedReleaseArtifact, UpdateError>> {
        Box::pin(async {
            self.state.lock().unwrap().events.push("verify");
            Ok(VerifiedReleaseArtifact {
                version: self.plan.target.version.clone(),
                image: self.plan.target.image.clone(),
            })
        })
    }
}
impl ReleaseInspectionRuntime for Fixture {
    fn inspect<'a>(
        &'a self,
        _: &'a VerifiedReleaseArtifact,
        _: &'a str,
        _: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<UpgradeDescriptor, UpdateError>> {
        Box::pin(async {
            self.state.lock().unwrap().events.push("inspect");
            Ok(UpgradeDescriptor {
                schema_version: 1,
                product: "citadel".into(),
                target_version: self.plan.target.version.clone(),
                target_catalog_sha256: self.plan.target.catalog_sha256.clone(),
                installer_format: INSTALLER_FORMAT,
                update_protocol: UPDATE_PROTOCOL,
                sources: vec![UpgradeSource {
                    version: self.plan.source.version.clone(),
                    catalog_sha256: self.plan.source.catalog_sha256.clone(),
                    architectures: vec!["amd64".into()],
                    agent_versions: vec![],
                }],
            })
        })
    }
}
fn actor() -> UpdateActor {
    UpdateActor {
        subject_id: Uuid::now_v7(),
        actor_id: ActorId::new(Uuid::now_v7()),
    }
}

#[tokio::test]
async fn approval_survives_dropped_response_and_retry_never_relaunches() {
    let fixture = Fixture::new();
    let (service, mut queue) = fixture.service();
    let actor = actor();
    let key = Uuid::now_v7();
    let approval = service.approve(actor, fixture.plan.id, key);
    tokio::pin!(approval);
    let command = tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut approval => panic!("worker must claim first") };
    // The command, not the browser future, owns execution.
    service
        .execute(command, &CancellationToken::new())
        .await
        .unwrap();
    let first = approval.await.unwrap();
    let retry = service.approve(actor, fixture.plan.id, key);
    tokio::pin!(retry);
    let command =
        tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut retry => panic!() };
    service
        .execute(command, &CancellationToken::new())
        .await
        .unwrap();
    assert_eq!(retry.await.unwrap().id, first.id);
    let events = &fixture.state.lock().unwrap().events;
    assert_eq!(events.iter().filter(|event| **event == "launch").count(), 1);
    assert_eq!(
        &events[..8],
        &[
            "source", "claim", "notify", "launch", "ack", "import", "notify", "close"
        ]
    );
}

#[tokio::test]
async fn dropped_browser_receiver_does_not_cancel_claimed_execution() {
    let fixture = Fixture::new();
    let (service, mut queue) = fixture.service();
    let command = {
        let approval = service.approve(actor(), fixture.plan.id, Uuid::now_v7());
        tokio::pin!(approval);
        tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut approval => panic!() }
    };
    service
        .execute(command, &CancellationToken::new())
        .await
        .unwrap();
    assert!(fixture.state.lock().unwrap().events.contains(&"grant"));
}

#[tokio::test]
async fn changed_source_or_revoked_administrator_prevents_claim_and_launch() {
    for denied in [false, true] {
        let fixture = Fixture::new();
        {
            let mut state = fixture.state.lock().unwrap();
            state.denied = denied;
            state.drift = !denied;
        }
        let (service, mut queue) = fixture.service();
        let approval = service.approve(actor(), fixture.plan.id, Uuid::now_v7());
        tokio::pin!(approval);
        let command = tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut approval => panic!() };
        service
            .execute(command, &CancellationToken::new())
            .await
            .unwrap();
        assert!(approval.await.is_err());
        let state = fixture.state.lock().unwrap();
        assert!(state.operation.is_none());
        assert!(!state.events.contains(&"launch"));
    }
}

#[tokio::test]
async fn missing_acknowledgement_preserves_claim_and_never_grants_shutdown() {
    let fixture = Fixture::new();
    fixture.state.lock().unwrap().fail_ack = true;
    let (service, mut queue) = fixture.service();
    let approval = service.approve(actor(), fixture.plan.id, Uuid::now_v7());
    tokio::pin!(approval);
    let command = tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut approval => panic!() };
    assert!(
        service
            .execute(command, &CancellationToken::new())
            .await
            .is_err()
    );
    assert!(approval.await.is_ok());
    let state = fixture.state.lock().unwrap();
    assert!(state.operation.is_some());
    assert!(!state.events.contains(&"grant"));
}

#[tokio::test(start_paused = true)]
async fn busy_drain_or_failed_grant_retains_ownership_without_relaunch() {
    for busy in [false, true] {
        let fixture = Fixture::new();
        {
            let mut state = fixture.state.lock().unwrap();
            state.busy = busy;
            state.fail_grant = !busy;
        }
        let (service, mut queue) = fixture.service();
        let approval = service.approve(actor(), fixture.plan.id, Uuid::now_v7());
        tokio::pin!(approval);
        let command = tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut approval => panic!() };
        assert!(
            service
                .execute(command, &CancellationToken::new())
                .await
                .is_err()
        );
        assert!(approval.await.is_ok());
        let state = fixture.state.lock().unwrap();
        assert!(state.operation.is_some());
        assert_eq!(state.events.contains(&"grant"), !busy);
        assert_eq!(
            state
                .events
                .iter()
                .filter(|event| **event == "launch")
                .count(),
            1
        );
    }
}
#[tokio::test]
async fn cancelled_approval_never_claims_or_starts_a_helper() {
    let fixture = Fixture::new();
    let (service, mut queue) = fixture.service();
    let approval = service.approve(actor(), fixture.plan.id, Uuid::now_v7());
    tokio::pin!(approval);
    let command = tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut approval => panic!() };
    let token = CancellationToken::new();
    token.cancel();
    service.execute(command, &token).await.unwrap();
    assert!(matches!(approval.await, Err(UpdateError::Cancelled)));
    assert!(fixture.state.lock().unwrap().operation.is_none());
}

#[tokio::test]
async fn review_verifies_the_transition_and_saves_only_a_short_lived_plan() {
    let fixture = Fixture::new();
    let (service, mut queue) = fixture.service();
    let review = service.review(actor(), &fixture.plan.target.version);
    tokio::pin!(review);
    let command =
        tokio::select! { command = queue.recv() => command.unwrap(), _ = &mut review => panic!() };
    service
        .execute(command, &CancellationToken::new())
        .await
        .unwrap();
    let plan = review.await.unwrap();
    assert_eq!(plan.source, fixture.plan.source);
    assert_eq!(plan.target, fixture.plan.target);
    assert_eq!(
        plan.expires_at - plan.reviewed_at,
        chrono::Duration::minutes(15)
    );
    let state = fixture.state.lock().unwrap();
    assert!(state.operation.is_none());
    assert_eq!(state.events, ["source", "verify", "inspect", "review"]);
}
