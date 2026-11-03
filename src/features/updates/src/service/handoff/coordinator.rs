use crate::{
    UpdateDrainOutcome, UpdateDrainService, UpdateError, UpdateHandoffRuntime, UpdateOperation,
};
use std::{sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;

pub struct UpdateHandoffService {
    drain: UpdateDrainService,
    runtime: Arc<dyn UpdateHandoffRuntime>,
    owner: tokio::sync::Mutex<()>,
}
impl UpdateHandoffService {
    pub fn new(drain: UpdateDrainService, runtime: Arc<dyn UpdateHandoffRuntime>) -> Self {
        Self {
            drain,
            runtime,
            owner: Default::default(),
        }
    }

    /// Run under a supervised owner outside the tracker being drained. Once the
    /// grant is published, a disconnect/cancellation cannot revoke a pending stop.
    /// Failures retain maintenance and must not trigger blind launch or stop retries.
    pub async fn authorize_shutdown(
        &self,
        operation: &UpdateOperation,
        drain_budget: Duration,
        cancellation: &CancellationToken,
    ) -> Result<(), UpdateError> {
        let _owner = self
            .owner
            .try_lock()
            .map_err(|_| UpdateError::Conflict("Update handoff is already running.".into()))?;
        self.runtime.acknowledged(operation, cancellation).await?;
        match self
            .drain
            .drain(operation, drain_budget, cancellation)
            .await?
        {
            UpdateDrainOutcome::Idle => {}
            UpdateDrainOutcome::Busy(_) => {
                return Err(UpdateError::Conflict(
                    "Update shutdown is blocked by unfinished work. Maintenance remains owned."
                        .into(),
                ));
            }
        }
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        self.runtime
            .authorize_shutdown(operation, cancellation)
            .await
    }
}
