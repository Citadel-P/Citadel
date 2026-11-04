use crate::{
    InstallationSnapshot, ReleaseInspectionService, UpdateActor, UpdateChangeNotifier,
    UpdateDrainService, UpdateError, UpdateInstallationRuntime, UpdateOperation, UpdatePlan,
    UpdateRepository,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub enum UpdateInstallCommand {
    Scheduled {
        reservation: OwnedSemaphorePermit,
        run: crate::UpdateScheduleRun,
        repository: Arc<dyn crate::AgentUpdateRepository>,
        entitlements: Arc<dyn crate::AgentUpdateEntitlements>,
        response: oneshot::Sender<Result<UpdateOperation, UpdateError>>,
    },
    Review {
        reservation: OwnedSemaphorePermit,
        actor: UpdateActor,
        version: String,
        database: bool,
        response: oneshot::Sender<Result<UpdatePlan, UpdateError>>,
    },
    Approve {
        reservation: OwnedSemaphorePermit,
        actor: UpdateActor,
        plan_id: Uuid,
        key: Uuid,
        response: oneshot::Sender<Result<UpdateOperation, UpdateError>>,
    },
}

#[derive(Clone)]
pub struct UpdateInstallService {
    repository: Arc<dyn UpdateRepository>,
    runtime: Arc<dyn UpdateInstallationRuntime>,
    inspector: Arc<ReleaseInspectionService>,
    drain: UpdateDrainService,
    notifier: Arc<dyn UpdateChangeNotifier>,
    queue: mpsc::Sender<UpdateInstallCommand>,
    slots: Arc<Semaphore>,
}

impl UpdateInstallService {
    pub fn new(
        repository: Arc<dyn UpdateRepository>,
        runtime: Arc<dyn UpdateInstallationRuntime>,
        inspector: Arc<ReleaseInspectionService>,
        drain: UpdateDrainService,
        notifier: Arc<dyn UpdateChangeNotifier>,
    ) -> (Self, mpsc::Receiver<UpdateInstallCommand>) {
        let (queue, receiver) = mpsc::channel(1);
        (
            Self {
                repository,
                runtime,
                inspector,
                drain,
                notifier,
                queue,
                slots: Arc::new(Semaphore::new(1)),
            },
            receiver,
        )
    }

    pub(crate) async fn qualify_availability(
        &self,
        version: &str,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let _reservation = self.reserve()?;
        self.prepare_scheduled(version, cancellation)
            .await
            .map(|_| ())
    }

    pub async fn review(
        &self,
        actor: UpdateActor,
        version: &str,
    ) -> Result<UpdatePlan, UpdateError> {
        self.review_component(actor, version, false).await
    }

    pub async fn review_database(&self, actor: UpdateActor) -> Result<UpdatePlan, UpdateError> {
        self.review_component(actor, "", true).await
    }

    pub async fn database_update(
        &self,
        cancellation: &CancellationToken,
    ) -> Result<Option<crate::DatabaseUpdatePlan>, UpdateError> {
        self.runtime.database_update(cancellation).await
    }

    async fn review_component(
        &self,
        actor: UpdateActor,
        version: &str,
        database: bool,
    ) -> Result<UpdatePlan, UpdateError> {
        let reservation = self.reserve()?;
        let (response, receiver) = oneshot::channel();
        self.queue
            .try_send(UpdateInstallCommand::Review {
                reservation,
                actor,
                version: version.into(),
                database,
                response,
            })
            .map_err(|_| UpdateError::Conflict("Another update request is running.".into()))?;
        receiver
            .await
            .map_err(|_| UpdateError::Unavailable("Update review was interrupted.".into()))?
    }

    async fn prepare_review(
        &self,
        actor: UpdateActor,
        version: &str,
        database: bool,
        cancellation: &CancellationToken,
    ) -> Result<UpdatePlan, UpdateError> {
        let plan = if database {
            let source = self.runtime.source(cancellation).await?;
            let database = self
                .runtime
                .database_update(cancellation)
                .await?
                .ok_or_else(|| {
                    UpdateError::Conflict("No supported PostgreSQL update is available.".into())
                })?;
            let now = chrono::Utc::now();
            let plan = UpdatePlan {
                database: Some(database),
                id: Uuid::now_v7(),
                instance_id: source.instance_id,
                helper_image: source.source.image.clone(),
                target: source.source.clone(),
                source: source.source,
                configuration_sha256: source.configuration_sha256,
                registration_id: source.registration_id,
                reviewed_at: now,
                expires_at: now + chrono::Duration::minutes(15),
                checkpoint_directory: source.checkpoint_directory,
                agent_requirements: source.agent_versions,
            };
            plan.validate(now)?;
            plan
        } else {
            self.prepare_scheduled(version, cancellation).await?
        };
        self.repository.save_review(actor, &plan).await?;
        Ok(plan)
    }

    /// Qualification is shared; scheduled authorization does not impersonate an administrator.
    pub(crate) async fn prepare_scheduled(
        &self,
        version: &str,
        cancellation: &CancellationToken,
    ) -> Result<UpdatePlan, UpdateError> {
        let source = self.runtime.source(cancellation).await?;
        let target = self
            .inspector
            .inspect(version, &source.source.architecture, cancellation)
            .await?;
        target.descriptor.validate_transition(
            &source.source,
            &target.identity,
            &source.agent_versions,
        )?;
        let now = chrono::Utc::now();
        let plan = UpdatePlan {
            database: None,
            id: Uuid::now_v7(),
            instance_id: source.instance_id,
            helper_image: source.source.image.clone(),
            source: source.source,
            target: target.identity,
            configuration_sha256: source.configuration_sha256,
            registration_id: source.registration_id,
            reviewed_at: now,
            expires_at: now + chrono::Duration::minutes(15),
            checkpoint_directory: source.checkpoint_directory,
            agent_requirements: source.agent_versions,
        };
        plan.validate(now)?;
        Ok(plan)
    }

    /// The supervised worker owns approval and execution before any database
    /// claim is made. Dropping the HTTP receiver cannot cancel a claimed update.
    pub async fn approve(
        &self,
        actor: UpdateActor,
        plan_id: Uuid,
        key: Uuid,
    ) -> Result<UpdateOperation, UpdateError> {
        if plan_id.is_nil() || key.is_nil() {
            return Err(UpdateError::Validation(
                "An update review and request key are required.".into(),
            ));
        }
        if let Some(operation) = self.replay(actor, plan_id, key).await? {
            return Ok(operation);
        }
        let reservation = self.reserve()?;
        let (response, receiver) = oneshot::channel();
        self.queue
            .try_send(UpdateInstallCommand::Approve {
                reservation,
                actor,
                plan_id,
                key,
                response,
            })
            .map_err(|_| UpdateError::Conflict("An update approval is already running.".into()))?;
        receiver.await.map_err(|_| {
            UpdateError::Unavailable(
                "Update approval was interrupted; check its status before retrying.".into(),
            )
        })?
    }

    pub(crate) async fn schedule(
        &self,
        run: crate::UpdateScheduleRun,
        repository: Arc<dyn crate::AgentUpdateRepository>,
        entitlements: Arc<dyn crate::AgentUpdateEntitlements>,
    ) -> Result<UpdateOperation, UpdateError> {
        let reservation = self.reserve()?;
        let (response, receiver) = oneshot::channel();
        self.queue
            .try_send(UpdateInstallCommand::Scheduled {
                reservation,
                run,
                repository,
                entitlements,
                response,
            })
            .map_err(|_| UpdateError::Conflict("Another update request is running.".into()))?;
        receiver.await.map_err(|_| {
            UpdateError::Unavailable(
                "Scheduled approval interrupted; ownership will be reconciled.".into(),
            )
        })?
    }

    fn reserve(&self) -> Result<OwnedSemaphorePermit, UpdateError> {
        self.slots.clone().try_acquire_owned().map_err(|_| {
            UpdateError::Conflict("Another update request is running; check its status.".into())
        })
    }

    async fn replay(
        &self,
        actor: UpdateActor,
        plan_id: Uuid,
        key: Uuid,
    ) -> Result<Option<UpdateOperation>, UpdateError> {
        let previous = self.repository.previous(actor, key).await?;
        if let Some(operation) = &previous
            && (operation.actor_id != actor.actor_id
                || operation.idempotency_key != key
                || operation.plan.id != plan_id)
        {
            return Err(UpdateError::Conflict(
                "The request key belongs to another approval.".into(),
            ));
        }
        Ok(previous)
    }

    pub async fn active(&self) -> Result<Option<UpdateOperation>, UpdateError> {
        self.repository.active().await
    }
    pub async fn operation(&self, id: Uuid) -> Result<UpdateOperation, UpdateError> {
        self.repository.operation(id).await
    }

    pub async fn execute(
        &self,
        command: UpdateInstallCommand,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let (actor, plan_id, key, response, _reservation) = match command {
            UpdateInstallCommand::Scheduled {
                reservation: _reservation,
                run,
                repository,
                entitlements,
                response,
            } => {
                let claim = async {
                    if !entitlements.automated_operations().await? {
                        return Err(UpdateError::Forbidden);
                    }
                    let plan = run.core_plan.as_ref().ok_or_else(|| {
                        UpdateError::Conflict("Missing scheduled Core review.".into())
                    })?;
                    check_source(&self.runtime.source(cancellation).await?, plan)?;
                    if cancellation.is_cancelled() {
                        return Err(UpdateError::Cancelled);
                    }
                    repository.claim_core(&run).await
                }
                .await;
                let operation = match claim {
                    Ok(op) => op,
                    Err(error) => {
                        let _ = response.send(Err(error));
                        return Ok(());
                    }
                };
                self.notifier.changed();
                let _ = response.send(Ok(operation.clone()));
                return self.execute_claimed(operation, cancellation).await;
            }
            UpdateInstallCommand::Review {
                reservation: _reservation,
                actor,
                version,
                database,
                response,
            } => {
                let result = self
                    .prepare_review(actor, &version, database, cancellation)
                    .await;
                let _ = response.send(result);
                return Ok(());
            }
            UpdateInstallCommand::Approve {
                reservation,
                actor,
                plan_id,
                key,
                response,
            } => (actor, plan_id, key, response, reservation),
        };
        let claim = async {
            // Replay is authorized by the repository and does not depend on a
            // review that may have expired or been pruned after acceptance.
            if let Some(operation) = self.replay(actor, plan_id, key).await? {
                return Ok((operation, false));
            }
            let plan = self.repository.review(actor, plan_id).await?;
            let source = self.runtime.source(cancellation).await?;
            check_source(&source, &plan)?;
            if let Some(database) = &plan.database
                && self.runtime.database_update(cancellation).await?.as_ref() != Some(database)
            {
                return Err(UpdateError::Conflict(
                    "PostgreSQL changed since review. Review again.".into(),
                ));
            }
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            self.repository
                .claim(actor, &plan, key, chrono::Utc::now())
                .await
        }
        .await;
        let (operation, created) = match claim {
            Ok(value) => value,
            Err(error) => {
                let _ = response.send(Err(error));
                return Ok(());
            }
        };
        self.notifier.changed();
        let _ = response.send(Ok(operation.clone()));
        if !created {
            return Ok(());
        }
        self.execute_claimed(operation, cancellation).await
    }

    async fn execute_claimed(
        &self,
        operation: UpdateOperation,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let mut launched = None;
        let execution = async {
            launched = Some(self.runtime.launch(&operation, cancellation).await?);
            let runtime = launched.as_ref().expect("helper launched");
            runtime.acknowledged(&operation, cancellation).await?;
            let journal = runtime.journal(&operation, cancellation).await?;
            self.repository.import(&journal).await?;
            self.notifier.changed();
            if self
                .drain
                .drain(&journal.operation, Duration::from_secs(300), cancellation)
                .await?
                != crate::UpdateDrainOutcome::Idle
            {
                return Err(UpdateError::Conflict(
                    "Unfinished work prevents shutdown. Maintenance remains active.".into(),
                ));
            }
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            runtime.authorize_shutdown(&operation, cancellation).await
        }
        .await;
        if execution.is_err() {
            // Recovery is worker-owned even if the caller disconnected or shutdown
            // cancelled preparation. Ambiguous stop evidence fails closed.
            let recovery = CancellationToken::new();
            let current = self.repository.operation(operation.id).await?;
            if let Err(error) = self.runtime.recover_source(&current, &recovery).await {
                // Preserve the independent executor's terminal observation even
                // when ambiguous shutdown prevents automatic source release.
                if let Some(runtime) = launched {
                    let observation = CancellationToken::new();
                    let read = runtime.journal(&operation, &observation);
                    tokio::pin!(read);
                    let observed = tokio::select! {
                        result = &mut read => result,
                        () = tokio::time::sleep(Duration::from_secs(60)) => { observation.cancel(); read.await }
                    };
                    if let Ok(journal) = observed
                        && matches!(
                            journal.operation.phase,
                            crate::UpdatePhase::Failed | crate::UpdatePhase::RecoveryRequired
                        )
                    {
                        self.repository.import(&journal).await?;
                        self.notifier.changed();
                    }
                }
                let latest = self.repository.operation(operation.id).await?;
                if matches!(
                    latest.phase,
                    crate::UpdatePhase::Preparing | crate::UpdatePhase::Draining
                ) {
                    self.repository.interrupt(&latest).await?;
                    self.notifier.changed();
                }
                return Err(error);
            }
            self.repository.abort_source(&current).await?;
            self.notifier.changed();
            self.drain.reopen(current.fence)?;
            return Ok(());
        }
        execution
    }
}

fn check_source(source: &InstallationSnapshot, plan: &UpdatePlan) -> Result<(), UpdateError> {
    if source.instance_id != plan.instance_id
        || source.source != plan.source
        || source.registration_id != plan.registration_id
        || source.configuration_sha256 != plan.configuration_sha256
        || source.checkpoint_directory != plan.checkpoint_directory
        || source.agent_versions != plan.agent_requirements
    {
        return Err(UpdateError::Conflict(
            "The installation changed since review. Review the update again.".into(),
        ));
    }
    Ok(())
}
