//! Container commands operate on persisted identities, never on a client-selected daemon.
use std::{sync::Arc, time::Duration};

use citadel_primitives::ActorId;
use futures_util::{StreamExt, future::BoxFuture};
use serde::Deserialize;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::RuntimeCapabilityError;
use crate::RuntimeErrorKind;

mod coordination;
pub use coordination::{
    ContainerOperationCoordinator, EVENT_CONFIRMATION_WINDOW, OperationRegistration,
};

#[derive(Debug, Default)]
pub struct ContainerCompletion {
    pub deployment_ids: Vec<Uuid>,
    pub stack_ids: Vec<Uuid>,
    pub container_patches: Vec<ContainerStatePatch>,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerStatePatch {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub container_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub control_state: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated: Option<i64>,
    #[serde(default)]
    pub docker_node_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerMutationNotice {
    Claimed,
    Completed,
    Released,
}

pub const CONTAINER_IO_CONCURRENCY: usize = 8;

pub const MAX_CONTAINER_BATCH: usize = 100;
pub const CONTAINER_OPERATION_TIMEOUT: Duration = Duration::from_secs(30);

pub trait ContainerInspectionPort: Send + Sync {
    fn inspection<'a>(
        &'a self,
        docker_id: &'a str,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<serde_json::Value, RuntimeCapabilityError>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerAction {
    Start,
    Stop,
    Restart,
    Pause,
    Unpause,
    Delete(DeleteContainerOptions),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerSelectionKind {
    Containers,
    Deployments,
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteContainerOptions {
    #[serde(default)]
    pub v: bool,
    #[serde(default)]
    pub force: bool,
    #[serde(default)]
    pub link: bool,
}

#[derive(Debug, Clone)]
pub struct ContainerTarget {
    pub id: Uuid,
    pub platform_id: Uuid,
    pub docker_id: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ContainerClaim {
    pub operation_id: Uuid,
    pub started_at: i64,
    pub targets: Vec<ContainerTarget>,
    pub deployment_ids: Vec<Uuid>,
    pub stack_ids: Vec<Uuid>,
}

/// Confirmed runtime observations only; errors never become absence.
#[derive(Debug)]
pub struct ContainerObservation {
    pub target: ContainerTarget,
    pub state: Option<String>,
    /// Captured before inspection, retained until its conditional commit.
    pub generation: Option<Arc<crate::jobs::SnapshotGeneration>>,
}

#[derive(Debug, Default)]
pub struct ContainerObservations {
    pub observed: Vec<ContainerObservation>,
    pub errors: Vec<(Uuid, RuntimeCapabilityError)>,
}
impl ContainerObservations {
    pub fn push(
        &mut self,
        target: &ContainerTarget,
        result: Result<Option<String>, RuntimeCapabilityError>,
    ) {
        match result {
            Ok(state) => self.observed.push(ContainerObservation {
                target: target.clone(),
                state,
                generation: None,
            }),
            Err(error) => self.errors.push((target.id, error)),
        }
    }
    pub fn result(self) -> Result<(), RuntimeCapabilityError> {
        container_batch_result(self.errors)
    }
}

/// Keep every failed identity and cause, with deterministic ordering. The caller
/// retains the durable claim on any error, even when other targets succeeded.
pub fn container_batch_result(
    mut errors: Vec<(Uuid, RuntimeCapabilityError)>,
) -> Result<(), RuntimeCapabilityError> {
    errors.sort_by_key(|(id, _)| *id);
    let Some((_, first)) = errors.first() else {
        return Ok(());
    };
    Err(RuntimeCapabilityError::new(
        first.kind,
        errors
            .iter()
            .map(|(id, error)| format!("{id}: {error}"))
            .collect::<Vec<_>>()
            .join("; "),
        false,
    ))
}

pub trait ContainerRepository: Send + Sync {
    fn finish_claim<'a>(
        &'a self,
        claim: &'a ContainerClaim,
    ) -> BoxFuture<'a, Result<ContainerCompletion, RuntimeCapabilityError>> {
        self.finish_committed(claim.operation_id)
    }
    fn coordinator(&self) -> Option<Arc<ContainerOperationCoordinator>> {
        None
    }
    fn finish_committed(
        &self,
        claim: Uuid,
    ) -> BoxFuture<'_, Result<ContainerCompletion, RuntimeCapabilityError>> {
        Box::pin(async move {
            self.finish(claim).await?;
            Ok(ContainerCompletion::default())
        })
    }

    fn resolve_ids<'a>(
        &'a self,
        ids: &'a [String],
    ) -> BoxFuture<'a, Result<Vec<Uuid>, RuntimeCapabilityError>>;
    /// All permissions and parent claims must succeed atomically before any runtime mutation.
    fn claim<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
        action: ContainerAction,
    ) -> BoxFuture<'a, Result<ContainerClaim, RuntimeCapabilityError>> {
        self.claim_selection(
            actor,
            administrator,
            ids,
            action,
            ContainerSelectionKind::Containers,
        )
    }

    fn claim_selection<'a>(
        &'a self,
        actor: ActorId,
        administrator: bool,
        ids: &'a [Uuid],
        action: ContainerAction,
        selection: ContainerSelectionKind,
    ) -> BoxFuture<'a, Result<ContainerClaim, RuntimeCapabilityError>>;

    /// Updates only the indicated lease. None means the container no longer exists.
    fn observed<'a>(
        &'a self,
        claim: Uuid,
        target: &'a ContainerTarget,
        state: Option<&'a str>,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;

    /// Implementations can commit the confirmed selection in one transaction.
    fn observed_batch<'a>(
        &'a self,
        claim: Uuid,
        observations: &'a [ContainerObservation],
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            for observation in observations {
                self.observed(claim, &observation.target, observation.state.as_deref())
                    .await?;
            }
            Ok(())
        })
    }

    fn finish(&self, claim: Uuid) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>>;
    fn abandon(&self, claim: Uuid) -> BoxFuture<'_, Result<(), RuntimeCapabilityError>>;

    /// Recovery is read/inspect only; an ambiguous destructive command is never replayed.
    fn stale(&self) -> BoxFuture<'_, Result<Vec<ContainerClaim>, RuntimeCapabilityError>>;
}

pub trait ContainerMutationRuntime: Send + Sync {
    fn mutate<'a>(
        &'a self,
        target: &'a ContainerTarget,
        action: ContainerAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>>;

    fn mutate_batch<'a>(
        &'a self,
        targets: &'a [ContainerTarget],
        action: ContainerAction,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), RuntimeCapabilityError>> {
        Box::pin(async move {
            for target in targets {
                self.mutate(target, action, cancellation).await?;
            }
            Ok(())
        })
    }

    fn observe<'a>(
        &'a self,
        target: &'a ContainerTarget,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>>;
    fn observe_batch<'a>(
        &'a self,
        targets: &'a [ContainerTarget],
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, ContainerObservations> {
        Box::pin(async move {
            let mut results = ContainerObservations::default();
            let mut pending = futures_util::stream::iter(targets.to_vec())
                .map(|target| async move {
                    let result = self.observe(&target, cancellation).await;
                    (target, result)
                })
                .buffer_unordered(CONTAINER_IO_CONCURRENCY);
            while let Some((target, result)) = pending.next().await {
                results.push(&target, result);
            }
            results
        })
    }
}

/// Process-owned admission for mutations that must outlive an HTTP connection.
pub trait ContainerTaskSpawner: Send + Sync {
    fn spawn(&self, operation: BoxFuture<'static, Result<(), RuntimeCapabilityError>>) -> bool;
    fn shutdown_token(&self) -> CancellationToken;
}

type ContainerMutationCallback =
    dyn Fn(&ContainerClaim, ContainerMutationNotice, &[ContainerStatePatch]) + Send + Sync;

#[derive(Clone)]
pub struct ContainerMutationService {
    store: Arc<dyn ContainerRepository>,
    runtime: Arc<dyn ContainerMutationRuntime>,
    operations: Arc<Semaphore>,
    tasks: Arc<dyn ContainerTaskSpawner>,
    changed: Arc<ContainerMutationCallback>,
    coordinator: Arc<ContainerOperationCoordinator>,
}

impl ContainerMutationService {
    pub fn new(
        store: Arc<dyn ContainerRepository>,
        runtime: Arc<dyn ContainerMutationRuntime>,
        tasks: Arc<dyn ContainerTaskSpawner>,
    ) -> Self {
        Self {
            coordinator: store.coordinator().unwrap_or_default(),
            store,
            runtime,
            tasks,
            operations: Arc::new(Semaphore::new(4)),
            changed: Arc::new(|_, _, _| {}),
        }
    }

    pub fn with_notifier(
        mut self,
        changed: impl Fn(&ContainerClaim, ContainerMutationNotice, &[ContainerStatePatch])
        + Send
        + Sync
        + 'static,
    ) -> Self {
        self.changed = Arc::new(changed);
        self
    }

    pub async fn execute(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: Vec<String>,
        action: ContainerAction,
    ) -> Result<(), RuntimeCapabilityError> {
        self.execute_selection(
            actor,
            administrator,
            ids,
            action,
            ContainerSelectionKind::Containers,
            self.tasks.shutdown_token(),
        )
        .await
    }

    pub async fn execute_deployments(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: Vec<Uuid>,
        action: ContainerAction,
    ) -> Result<(), RuntimeCapabilityError> {
        if ids.iter().any(Uuid::is_nil) || matches!(action, ContainerAction::Delete(_)) {
            return Err(error(
                RuntimeErrorKind::InvalidRequest,
                "Select valid Deployment IDs and a supported state action.",
            ));
        }
        self.execute_selection(
            actor,
            administrator,
            ids.into_iter().map(|id| id.to_string()).collect(),
            action,
            ContainerSelectionKind::Deployments,
            self.tasks.shutdown_token(),
        )
        .await
    }

    /// A hosted job owns cancellation, unlike a disconnected HTTP request.
    pub async fn execute_background(
        &self,
        actor: ActorId,
        ids: Vec<String>,
        action: ContainerAction,
        cancellation: CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        self.execute_selection(
            actor,
            true,
            ids,
            action,
            ContainerSelectionKind::Containers,
            cancellation,
        )
        .await
    }

    async fn execute_selection(
        &self,
        actor: ActorId,
        administrator: bool,
        ids: Vec<String>,
        action: ContainerAction,
        selection: ContainerSelectionKind,
        cancellation: CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        if ids.is_empty()
            || ids.len() > MAX_CONTAINER_BATCH
            || ids.iter().any(|id| !valid_container_id(id))
        {
            return Err(error(
                RuntimeErrorKind::InvalidRequest,
                "Select between 1 and 100 valid Container IDs.",
            ));
        }
        let permit = self.operations.clone().try_acquire_owned().map_err(|_| {
            error(
                RuntimeErrorKind::ResourceExhausted,
                "Container operations are busy.",
            )
        })?;
        let service = self.clone();
        // A disconnected HTTP caller must not abandon a committed processing claim.
        let (completed, result) = tokio::sync::oneshot::channel();
        if !self.tasks.spawn(Box::pin(async move {
            let result = async move {
                let _permit = permit;
                let mut ids = if selection == ContainerSelectionKind::Deployments {
                    ids.iter()
                        .map(|id| {
                            Uuid::parse_str(id).map_err(|_| {
                                error(RuntimeErrorKind::InvalidRequest, "Invalid Deployment ID.")
                            })
                        })
                        .collect::<Result<Vec<_>, _>>()?
                } else {
                    tokio::time::timeout(Duration::from_secs(10), service.store.resolve_ids(&ids))
                        .await
                        .map_err(|_| {
                            error(
                                RuntimeErrorKind::Timeout,
                                "Resolving the Container IDs timed out.",
                            )
                        })??
                };
                ids.sort_unstable();
                ids.dedup();
                let claim = tokio::time::timeout(
                    Duration::from_secs(10),
                    service
                        .store
                        .claim_selection(actor, administrator, &ids, action, selection),
                )
                .await
                .map_err(|_| {
                    error(
                        RuntimeErrorKind::Timeout,
                        "Claiming the Container operation timed out.",
                    )
                })??;
                let mut registration = service.coordinator.register(&claim, action);
                (service.changed)(&claim, ContainerMutationNotice::Claimed, &[]);
                let _cancel_on_drop = cancellation.clone().drop_guard();
                let work = async {
                    // Even a failed/partial mutation can have successful siblings. Verify
                    // and persist them now; retain the claim for read-only recovery.
                    let mutation = service
                        .runtime
                        .mutate_batch(&claim.targets, action, &cancellation)
                        .await;
                    let verification = service.verify(&claim, registration.as_mut(), &cancellation).await;
                    match (mutation, verification) {
                        (Ok(()), result) | (result, Ok(())) => result,
                        (Err(mutation), Err(verification)) => Err(error(
                            mutation.kind,
                            &format!("Mutation: {mutation}; verification: {verification}"),
                        )),
                    }
                };
                let result = tokio::select! {
                    biased;
                    () = cancellation.cancelled() => Err(error(RuntimeErrorKind::Unavailable,
                        "Container operation canceled; runtime state will be reconciled.")),
                    result = tokio::time::timeout(CONTAINER_OPERATION_TIMEOUT, work) => result.unwrap_or_else(|_| {
                        Err(error(RuntimeErrorKind::Timeout, "Container operation timed out; runtime state will be reconciled."))
                    }),
                };
                cancellation.cancel();
                // On partial failure retain the lease until read-only recovery has observed all targets.
                if result.is_ok() {
                    let completion = tokio::time::timeout(
                        Duration::from_secs(5),
                        service.store.finish_claim(&claim),
                    )
                    .await
                    .map_err(|_| {
                        error(
                            RuntimeErrorKind::Timeout,
                            "Container operation finalization timed out.",
                        )
                    })??;
                    let mut finished = claim.clone();
                    finished.deployment_ids = completion.deployment_ids;
                    finished.stack_ids = completion.stack_ids;
                    (service.changed)(
                        &finished,
                        ContainerMutationNotice::Completed,
                        &completion.container_patches,
                    );
                }
                result
            }
            .await;
            // If the caller disconnected, the process owner still observes errors
            // and drains claim cleanup before closing persistence.
            if let Err(result) = completed.send(result) {
                result?;
            }
            Ok(())
        })) {
            return Err(error(
                RuntimeErrorKind::ResourceExhausted,
                "Container operations are shutting down.",
            ));
        }
        result.await.map_err(|_| {
            error(
                RuntimeErrorKind::Remote,
                "Container operation failed; runtime state will be reconciled.",
            )
        })?
    }

    async fn verify(
        &self,
        claim: &ContainerClaim,
        registration: Option<&mut OperationRegistration>,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        let mut registration = registration;
        // A lifecycle event may arrive while a slow inspection is in flight.
        // Retry only the still-unconfirmed selection, with a bounded attempt count.
        for attempt in 0..3 {
            let missing = match registration.as_deref_mut() {
                Some(registration) => registration.wait_missing(cancellation).await?,
                None => claim.targets.clone(),
            };
            if missing.is_empty() {
                return Ok(());
            }
            let mut stamps = std::collections::BTreeMap::new();
            for target in &missing {
                let key = (target.platform_id, target.node_id.clone());
                if let std::collections::btree_map::Entry::Vacant(e) = stamps.entry(key) {
                    let stamp = crate::jobs::SnapshotGeneration::capture(
                        target.platform_id,
                        target.node_id.as_deref(),
                        crate::jobs::ProjectionKind::Containers,
                    )
                    .await;
                    e.insert(Arc::new(stamp));
                }
            }
            let mut observations = self.runtime.observe_batch(&missing, cancellation).await;
            for observation in &mut observations.observed {
                observation.generation = stamps
                    .get(&(
                        observation.target.platform_id,
                        observation.target.node_id.clone(),
                    ))
                    .cloned();
            }
            match self
                .store
                .observed_batch(claim.operation_id, &observations.observed)
                .await
            {
                Err(error)
                    if error.kind == RuntimeErrorKind::Conflict
                        && error.retryable
                        && attempt < 2 =>
                {
                    continue;
                }
                result => result?,
            }
            return observations.result();
        }
        unreachable!("the final attempt returns its result")
    }

    pub async fn reconcile(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        for claim in self.store.stale().await? {
            if cancellation.is_cancelled() {
                break;
            }
            let work = async {
                self.verify(&claim, None, cancellation).await?;
                if cancellation.is_cancelled() {
                    return Err(error(RuntimeErrorKind::Unavailable, "Recovery canceled."));
                }
                let completion = self.store.finish_claim(&claim).await?;
                let mut finished = claim.clone();
                finished.deployment_ids = completion.deployment_ids;
                finished.stack_ids = completion.stack_ids;
                (self.changed)(
                    &finished,
                    ContainerMutationNotice::Completed,
                    &completion.container_patches,
                );
                Ok::<_, RuntimeCapabilityError>(())
            };
            // A disconnected node is not evidence that its containers have stopped or disappeared.
            if !matches!(
                tokio::time::timeout(CONTAINER_OPERATION_TIMEOUT, work).await,
                Ok(Ok(()))
            ) && !cancellation.is_cancelled()
                && chrono::Utc::now()
                    .timestamp()
                    .saturating_sub(claim.started_at)
                    >= 300
            {
                self.store.abandon(claim.operation_id).await?;
                (self.changed)(&claim, ContainerMutationNotice::Released, &[]);
            }
        }
        Ok(())
    }
}

fn error(kind: RuntimeErrorKind, message: &str) -> RuntimeCapabilityError {
    RuntimeCapabilityError::new(kind, message, false)
}

fn valid_container_id(id: &str) -> bool {
    if let Ok(id) = Uuid::parse_str(id) {
        return !id.is_nil();
    }
    matches!(id.len(), 12 | 64) && id.bytes().all(|b| b.is_ascii_hexdigit())
}
