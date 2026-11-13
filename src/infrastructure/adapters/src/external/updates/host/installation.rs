use super::super::DockerHelperHandoff;
use super::{HostUpdateClient, HostUpdateRequest, HostUpdateResponse};
use citadel_execution::ProcessRunner;
use citadel_updates::{
    CoreIdentity, InstallationSnapshot, UpdateError, UpdateHandoffRuntime,
    UpdateInstallationRuntime, UpdateOperation,
};
use futures_util::future::BoxFuture;
use std::{path::PathBuf, sync::Arc};
use tokio_util::sync::CancellationToken;

pub struct DockerUpdateInstallation {
    client: HostUpdateClient,
    process: Arc<dyn ProcessRunner>,
    state_directory: PathBuf,
    version: String,
    pool: sqlx::PgPool,
    registered: Arc<std::sync::atomic::AtomicBool>,
}

impl DockerUpdateInstallation {
    pub fn new(
        process: Arc<dyn ProcessRunner>,
        state_directory: PathBuf,
        version: String,
        pool: sqlx::PgPool,
    ) -> Self {
        Self {
            client: HostUpdateClient::new(process.clone(), state_directory.clone()),
            process,
            state_directory,
            version,
            pool,
            registered: Arc::new(std::sync::atomic::AtomicBool::new(true)),
        }
    }
}

impl DockerUpdateInstallation {
    pub fn with_registration(mut self, registered: Arc<std::sync::atomic::AtomicBool>) -> Self {
        self.registered = registered;
        self
    }
}

impl UpdateInstallationRuntime for DockerUpdateInstallation {
    fn registered(&self) -> bool {
        self.registered.load(std::sync::atomic::Ordering::Acquire)
    }
    fn database_update<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Option<citadel_updates::DatabaseUpdatePlan>, UpdateError>> {
        Box::pin(async move {
            if !self.registered() {
                return Err(UpdateError::Unavailable("Automatic update setup has not completed. Check Core logs for setup diagnostics.".into()));
            }
            // Startup or the supervised first-install task validates registration.
            // Availability is advisory: never acquire the installation lock or
            // launch preflight helpers for a status read. Review and approval
            // each call source() separately before accepting a database plan.
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            let repository = crate::persistence::postgres::updates::PostgresUpdateRepository::new(
                self.pool.clone(),
            );
            let version =
                citadel_updates::ReleaseCheckRepository::database_version(&repository).await?;
            super::super::compose::database_update(version)
        })
    }

    fn validate_database<'a>(
        &'a self,
        plan: &'a citadel_updates::UpdatePlan,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            let (response, image, architecture) = self
                .client
                .exchange(
                    HostUpdateRequest::CheckDatabase(Box::new(plan.clone())),
                    cancellation,
                )
                .await?;
            if image != plan.source.image || architecture != plan.source.architecture {
                return Err(super::super::docker::failed());
            }
            match response {
                HostUpdateResponse::DatabaseChecked { plan_id, blocker } if plan_id == plan.id => {
                    match blocker {
                        Some(message) => Err(UpdateError::Conflict(message)),
                        None => Ok(()),
                    }
                }
                _ => Err(super::super::docker::failed()),
            }
        })
    }

    fn recover_source<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(async move {
            if self.version != operation.plan.source.version {
                return Err(super::super::docker::failed());
            }
            let (response, image, architecture) = self
                .client
                .exchange(
                    HostUpdateRequest::RecoverSource(Box::new(operation.clone())),
                    cancellation,
                )
                .await?;
            if !matches!(response, HostUpdateResponse::SourceRecovered(ref saved) if **saved == *operation)
                || image != operation.plan.source.image
                || architecture != operation.plan.source.architecture
            {
                return Err(super::super::docker::failed());
            }
            Ok(())
        })
    }

    fn source<'a>(
        &'a self,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<InstallationSnapshot, UpdateError>> {
        Box::pin(async move {
            if !self.registered() {
                return Err(UpdateError::Conflict(
                    "Automatic update setup has not completed.".into(),
                ));
            }
            let (response, image, architecture) = self
                .client
                .exchange(
                    HostUpdateRequest::Check {
                        state_directory: self.state_directory.clone(),
                    },
                    cancellation,
                )
                .await?;
            let HostUpdateResponse::Checked(checked) = response else {
                return Err(super::super::docker::failed());
            };
            let agents =
                crate::persistence::postgres::updates::read_update_agent_versions(&self.pool)
                    .await?;
            Ok(InstallationSnapshot {
                instance_id: checked.database.instance_id,
                source: CoreIdentity {
                    version: self.version.clone(),
                    image,
                    architecture,
                    catalog_sha256: checked.database.catalog_sha256,
                },
                registration_id: checked.registration.id,
                configuration_sha256: checked.registration.configuration_sha256,
                checkpoint_directory: self
                    .state_directory
                    .to_str()
                    .ok_or_else(super::super::docker::failed)?
                    .into(),
                agent_versions: agents,
            })
        })
    }
    fn launch<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<Arc<dyn UpdateHandoffRuntime>, UpdateError>> {
        Box::pin(async move {
            let (response, image, _) = self
                .client
                .exchange(
                    HostUpdateRequest::Launch(Box::new(operation.clone())),
                    cancellation,
                )
                .await?;
            let HostUpdateResponse::Launched(launched) = response else {
                return Err(super::super::docker::failed());
            };
            if launched.intent.operation != *operation || image != operation.plan.source.image {
                return Err(super::super::docker::failed());
            }
            Ok(Arc::new(DockerHelperHandoff::new(
                launched.intent,
                launched.container_id,
                self.process.clone(),
            )) as Arc<dyn UpdateHandoffRuntime>)
        })
    }
}
