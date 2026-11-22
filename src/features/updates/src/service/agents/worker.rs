use super::AgentUpdateService;
use crate::UpdateError;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

impl AgentUpdateService {
    pub fn work_changes(&self) -> tokio::sync::watch::Receiver<()> {
        self.work_changed.subscribe()
    }

    /// Probe only scheduling/ownership columns until durable work needs full state.
    pub async fn poll(&self, cancellation: &CancellationToken) -> Result<Duration, UpdateError> {
        if self.repository.pending_work().await? {
            self.tick(cancellation).await?;
            return Ok(Duration::from_secs(5));
        }
        Ok(Duration::from_secs(30))
    }
}
