//! Core-side Docker transport; registered host files are only accessed by the helper.
use super::super::docker::LocalDocker;
use super::{
    command::{HandoffAction, HandoffRequest, MAX_HANDOFF_REQUEST},
    evidence::{HelperAcknowledgement, HelperBinding, HelperLaunchIntent, conflict},
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
        action: HandoffAction,
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
            deadline: Instant::now() + Duration::from_secs(60),
        };
        let observed = verification::helper(&docker, &self.intent, &binding).await?;
        let request = HandoffRequest {
            intent: self.intent.clone(),
            binding: binding.clone(),
            started_at: observed.started_at.clone(),
            action,
        };
        let input = serde_json::to_vec(&request).map_err(|_| conflict())?;
        if input.len() > MAX_HANDOFF_REQUEST {
            return Err(conflict());
        }
        let user = format!(
            "{}:{}",
            self.intent.registration.executor_uid, self.intent.registration.executor_gid
        );
        let temporary = format!(
            "TMPDIR={}",
            self.intent.registration.state_directory.display()
        );
        let output = docker
            .run_with_input(
                &[
                    "exec",
                    "--interactive",
                    "--env",
                    &temporary,
                    "--user",
                    &user,
                    &self.helper_container_id,
                    "/app/citadel-server",
                    "update-helper-handoff",
                ],
                input,
                Duration::from_secs(45),
            )
            .await?;
        let ack: HelperAcknowledgement = serde_json::from_slice(&output).map_err(|_| conflict())?;
        if ack.binding != binding
            || ack.incarnation.is_nil()
            || ack.started_at != observed.started_at
        {
            return Err(conflict());
        }
        let after = verification::helper(&docker, &self.intent, &binding).await?;
        if after.started_at != observed.started_at {
            return Err(conflict());
        }
        Ok(())
    }
}
impl UpdateHandoffRuntime for DockerHelperHandoff {
    fn acknowledged<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.exchange(operation, HandoffAction::Check, cancellation))
    }
    fn authorize_shutdown<'a>(
        &'a self,
        operation: &'a UpdateOperation,
        cancellation: &'a CancellationToken,
    ) -> BoxFuture<'a, Result<(), UpdateError>> {
        Box::pin(self.exchange(operation, HandoffAction::Grant, cancellation))
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
