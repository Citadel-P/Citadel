//! Core-side Docker transport; registered host files are only accessed by the helper.
use super::super::docker::LocalDocker;
use super::{
    command::{HelperAction, HelperRequest, HelperResponse},
    evidence::{HelperBinding, HelperLaunchIntent, conflict},
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateHandoffRuntime, UpdateOperation, is_sha256};
use futures_util::future::BoxFuture;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

pub struct DockerHelperHandoff {
    intent: HelperLaunchIntent,
    helper_container_id: String,
    process: Arc<dyn ProcessRunner>,
}
impl DockerHelperHandoff {
    /// The launcher supplies the exact confirmed container ID, never a browser identifier.
    pub fn new(
        intent: HelperLaunchIntent,
        helper_container_id: String,
        process: Arc<dyn ProcessRunner>,
    ) -> Self {
        Self {
            intent,
            helper_container_id,
            process,
        }
    }
    async fn exchange(
        &self,
        operation: &UpdateOperation,
        action: HelperAction,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        if operation != &self.intent.operation
            || !is_sha256(&self.helper_container_id)
            || self.intent.nonce.is_nil()
        {
            return Err(conflict());
        }
        operation.plan.validate(operation.created_at)?;
        self.intent.registration.validate()?;
        let binding = HelperBinding {
            nonce: self.intent.nonce,
            helper_container_id: self.helper_container_id.clone(),
        };
        let home = private_home()?;
        let docker = LocalDocker {
            process: self.process.as_ref(),
            cancellation,
            home: home.path(),
            deadline: Instant::now()
                + Duration::from_secs(if matches!(action, HelperAction::Check) {
                    900
                } else {
                    60
                }),
        };
        let observed = verification::helper(&docker, &self.intent, &binding).await?;
        let request = HelperRequest {
            operation: operation.clone(),
            binding: binding.clone(),
            started_at: observed.started_at.clone(),
            action,
        };
        let user = format!(
            "{}:{}",
            self.intent.registration.executor_uid, self.intent.registration.executor_gid
        );
        let HelperResponse::Acknowledged(ack) = request
            .exchange(&docker, &user, Duration::from_secs(45))
            .await?
        else {
            return Err(conflict());
        };
        if ack.binding != binding || ack.started_at != observed.started_at {
            return Err(conflict());
        }
        verification::acknowledged_helper(&docker, &self.intent, &ack).await?;
        Ok(())
    }
}
impl UpdateHandoffRuntime for DockerHelperHandoff {
    fn journal<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<citadel_updates::UpdateJournal, UpdateError>> {
        Box::pin(async move {
            let read = async {
                if operation != &self.intent.operation {
                    return Err(conflict());
                }
                let home = private_home()?;
                let docker = LocalDocker {
                    process: self.process.as_ref(),
                    cancellation,
                    home: home.path(),
                    deadline: Instant::now() + Duration::from_secs(60),
                };
                let binding = HelperBinding {
                    nonce: self.intent.nonce,
                    helper_container_id: self.helper_container_id.clone(),
                };
                let observed = verification::helper(&docker, &self.intent, &binding).await?;
                let request = HelperRequest {
                    operation: operation.clone(),
                    binding,
                    started_at: observed.started_at,
                    action: HelperAction::InspectJournal,
                };
                let user = format!(
                    "{}:{}",
                    self.intent.registration.executor_uid, self.intent.registration.executor_gid
                );
                let HelperResponse::Journal(journal) = request
                    .exchange(&docker, &user, Duration::from_secs(30))
                    .await?
                else {
                    return Err(conflict());
                };
                journal.validate_for(operation)?;
                if journal.helper_container_id != self.helper_container_id {
                    return Err(conflict());
                }
                Ok(*journal)
            }
            .await;
            if read.is_ok() || cancellation.is_cancelled() {
                return read;
            }
            // An exited helper cannot answer docker exec. Read only its durable
            // terminal journal under the host lock; never infer success from exit.
            use super::super::host::{HostUpdateClient, HostUpdateRequest, HostUpdateResponse};
            let client = HostUpdateClient::new(
                self.process.clone(),
                self.intent.registration.state_directory.clone(),
            );
            let (response, image, _) = client
                .exchange(
                    HostUpdateRequest::FailedJournal(Box::new(operation.clone())),
                    cancellation,
                )
                .await?;
            let HostUpdateResponse::FailedJournal(journal) = response else {
                return Err(conflict());
            };
            journal.validate_for(operation)?;
            if image != operation.plan.source.image
                || journal.helper_container_id != self.helper_container_id
            {
                return Err(conflict());
            }
            Ok(*journal)
        })
    }

    fn acknowledged<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.exchange(operation, HelperAction::Check, cancellation))
    }
    fn authorize_shutdown<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.exchange(operation, HelperAction::Grant, cancellation))
    }
}

pub(super) fn private_home() -> Result<tempfile::TempDir, UpdateError> {
    use std::os::unix::fs::PermissionsExt;
    tempfile::Builder::new()
        .prefix("citadel-handoff-")
        .permissions(std::fs::Permissions::from_mode(0o700))
        .tempdir()
        .map_err(|_| conflict())
}
