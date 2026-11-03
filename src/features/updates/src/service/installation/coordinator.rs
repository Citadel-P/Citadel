use crate::{
    InstallationSnapshot, ReleaseInspectionService, UpdateActor, UpdateChangeNotifier,
    UpdateDrainService, UpdateError, UpdateInstallationRuntime, UpdateOperation, UpdatePlan,
    UpdateRepository,
};
use std::{sync::Arc, time::Duration};
use tokio::sync::{mpsc, oneshot};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub enum UpdateInstallCommand {
    Review {
        actor: UpdateActor,
        version: String,
        response: oneshot::Sender<Result<UpdatePlan, UpdateError>>,
    },
    Approve {
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
            },
            receiver,
        )
    }

    pub async fn review(
        &self,
        actor: UpdateActor,
        version: &str,
    ) -> Result<UpdatePlan, UpdateError> {
        let (response, receiver) = oneshot::channel();
        self.queue
            .try_send(UpdateInstallCommand::Review {
                actor,
                version: version.into(),
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
        self.repository.save_review(actor, &plan).await?;
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
        let (response, receiver) = oneshot::channel();
        self.queue
            .try_send(UpdateInstallCommand::Approve {
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
        let (actor, plan_id, key, response) = match command {
            UpdateInstallCommand::Review {
                actor,
                version,
                response,
            } => {
                let result = self.prepare_review(actor, &version, cancellation).await;
                let _ = response.send(result);
                return Ok(());
            }
            UpdateInstallCommand::Approve {
                actor,
                plan_id,
                key,
                response,
            } => (actor, plan_id, key, response),
        };
        let claim = async {
            let plan = self.repository.review(actor, plan_id).await?;
            // Exact retries return the stored claim before running source preflight,
            // which intentionally refuses installations already in maintenance.
            if self.repository.previous(actor, key).await?.is_some() {
                return self
                    .repository
                    .claim(actor, &plan, key, chrono::Utc::now())
                    .await;
            }
            let source = self.runtime.source(cancellation).await?;
            check_source(&source, &plan)?;
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
        let runtime = self.runtime.launch(&operation, cancellation).await?;
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
