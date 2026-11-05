use crate::{
    UpdateDrainOutcome, UpdateDrainRepository, UpdateDrainRuntime, UpdateError, UpdateOperation,
    UpdatePhase,
};
use std::{sync::Arc, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct UpdateDrainService {
    repository: Arc<dyn UpdateDrainRepository>,
    runtime: Arc<dyn UpdateDrainRuntime>,
    owner: Arc<tokio::sync::Mutex<()>>,
}

impl UpdateDrainService {
    pub(crate) fn reopen(&self, fence: uuid::Uuid) -> Result<(), UpdateError> {
        self.runtime.reopen(fence)
    }
    pub fn new(
        repository: Arc<dyn UpdateDrainRepository>,
        runtime: Arc<dyn UpdateDrainRuntime>,
    ) -> Self {
        Self {
            repository,
            runtime,
            owner: Default::default(),
        }
    }

    /// The caller already owns a committed approval and must use a supervised
    /// task, not a browser request, to drive the update. This step performs no
    /// mutation of journal history, no automatic release and no force cancellation.
    pub async fn drain(
        &self,
        operation: &UpdateOperation,
        budget: Duration,
        cancellation: &CancellationToken,
    ) -> Result<UpdateDrainOutcome, UpdateError> {
        if budget.is_zero() || budget > Duration::from_secs(3600) {
            return Err(UpdateError::Validation(
                "Drain budget must be greater than zero and at most one hour.".into(),
            ));
        }
        if !matches!(
            operation.phase,
            UpdatePhase::Preparing | UpdatePhase::Draining
        ) {
            return Err(UpdateError::Conflict(
                "The update is not in preparation or draining.".into(),
            ));
        }
        let _owner = self
            .owner
            .try_lock()
            .map_err(|_| UpdateError::Conflict("An update drain is already running.".into()))?;
        let deadline = Instant::now() + budget;
        // Never close normal admission for an unverified caller-supplied fence.
        self.observe(operation, deadline, cancellation).await?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        self.runtime.close(operation.fence)?;
        loop {
            // Recheck persisted ownership after closing and on every observation.
            // Read failures and stale ownership fail closed; absence of evidence
            // cannot become an empty/ready result.
            // Bracket the database snapshot. A local controller can persist an
            // unresolved reservation and finish during the read; observing only
            // its later zero count would miss that newly persisted blocker.
            let before = self.runtime.blockers(operation.fence)?;
            let persisted = self.observe(operation, deadline, cancellation).await?;
            let after = self.runtime.blockers(operation.fence)?;
            let mut counts = std::collections::BTreeMap::<String, u64>::new();
            for blocker in before.into_iter().chain(persisted).chain(after) {
                counts
                    .entry(blocker.kind)
                    .and_modify(|count| *count = (*count).max(blocker.count))
                    .or_insert(blocker.count);
            }
            let blockers: Vec<_> = counts
                .into_iter()
                .filter(|(_, count)| *count > 0)
                .map(|(kind, count)| crate::UpdateDrainBlocker { kind, count })
                .collect();
            if blockers.is_empty() {
                return Ok(UpdateDrainOutcome::Idle);
            }
            let next = (Instant::now() + Duration::from_secs(1)).min(deadline);
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(UpdateError::Cancelled),
                () = tokio::time::sleep_until(next) => {}
            }
            if Instant::now() >= deadline {
                return Ok(UpdateDrainOutcome::Busy(blockers));
            }
        }
    }

    async fn observe(
        &self,
        operation: &UpdateOperation,
        deadline: Instant,
        cancellation: &CancellationToken,
    ) -> Result<Vec<crate::UpdateDrainBlocker>, UpdateError> {
        tokio::select! {
            biased;
            () = cancellation.cancelled() => Err(UpdateError::Cancelled),
            result = tokio::time::timeout_at(deadline, self.repository.blockers(operation)) =>
                result.map_err(|_| UpdateError::Unavailable("Timed out verifying update ownership and unfinished work.".into()))?,
        }
    }
}
