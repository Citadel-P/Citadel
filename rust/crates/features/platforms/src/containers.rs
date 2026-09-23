//! Container commands operate on persisted identities, never on a client-selected daemon.
use std::{sync::Arc, time::Duration};

use citadel_primitives::ActorId;
use futures_util::future::BoxFuture;
use serde::Deserialize;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::RuntimeCapabilityError;
use crate::RuntimeErrorKind;

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

pub trait ContainerRepository: Send + Sync {
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

    fn observe<'a>(
        &'a self,
        target: &'a ContainerTarget,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<String>, RuntimeCapabilityError>>;
}

/// Process-owned admission for mutations that must outlive an HTTP connection.
pub trait ContainerTaskSpawner: Send + Sync {
    fn spawn(&self, operation: BoxFuture<'static, Result<(), RuntimeCapabilityError>>) -> bool;
    fn shutdown_token(&self) -> CancellationToken;
}

#[derive(Clone)]
pub struct ContainerMutationService {
    store: Arc<dyn ContainerRepository>,
    runtime: Arc<dyn ContainerMutationRuntime>,
    operations: Arc<Semaphore>,
    tasks: Arc<dyn ContainerTaskSpawner>,
    changed: Arc<dyn Fn(&ContainerClaim) + Send + Sync>,
}

impl ContainerMutationService {
    pub fn new(
        store: Arc<dyn ContainerRepository>,
        runtime: Arc<dyn ContainerMutationRuntime>,
        tasks: Arc<dyn ContainerTaskSpawner>,
    ) -> Self {
        Self {
            store,
            runtime,
            tasks,
            operations: Arc::new(Semaphore::new(4)),
            changed: Arc::new(|_| {}),
        }
    }

    pub fn with_notifier(
        mut self,
        changed: impl Fn(&ContainerClaim) + Send + Sync + 'static,
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
                (service.changed)(&claim);
                let _cancel_on_drop = cancellation.clone().drop_guard();
                let work = async {
                    for target in &claim.targets {
                        service
                            .runtime
                            .mutate(target, action, &cancellation)
                            .await?;
                        let state = service.runtime.observe(target, &cancellation).await?;
                        service
                            .store
                            .observed(claim.operation_id, target, state.as_deref())
                            .await?;
                    }
                    Ok::<_, RuntimeCapabilityError>(())
                };
                let result = tokio::time::timeout(CONTAINER_OPERATION_TIMEOUT, work)
                    .await
                    .unwrap_or_else(|_| {
                        Err(error(
                            RuntimeErrorKind::Timeout,
                            "Container operation timed out; runtime state will be reconciled.",
                        ))
                    });
                cancellation.cancel();
                // On partial failure retain the lease until read-only recovery has observed all targets.
                if result.is_ok() {
                    tokio::time::timeout(
                        Duration::from_secs(5),
                        service.store.finish(claim.operation_id),
                    )
                    .await
                    .map_err(|_| {
                        error(
                            RuntimeErrorKind::Timeout,
                            "Container operation finalization timed out.",
                        )
                    })??;
                    (service.changed)(&claim);
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

    pub async fn reconcile(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<(), RuntimeCapabilityError> {
        for claim in self.store.stale().await? {
            if cancellation.is_cancelled() {
                break;
            }
            let work = async {
                for target in &claim.targets {
                    let state = self.runtime.observe(target, cancellation).await?;
                    self.store
                        .observed(claim.operation_id, target, state.as_deref())
                        .await?;
                }
                self.store.finish(claim.operation_id).await?;
                (self.changed)(&claim);
                Ok::<_, RuntimeCapabilityError>(())
            };
            // A disconnected node is not evidence that its containers have stopped or disappeared.
            if !matches!(
                tokio::time::timeout(CONTAINER_OPERATION_TIMEOUT, work).await,
                Ok(Ok(()))
            ) && chrono::Utc::now()
                .timestamp()
                .saturating_sub(claim.started_at)
                >= 300
            {
                self.store.abandon(claim.operation_id).await?;
                (self.changed)(&claim);
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
