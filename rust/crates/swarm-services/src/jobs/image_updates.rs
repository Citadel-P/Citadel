use super::*;

impl ManagedSwarmServiceService {
    /// Keyset paging bounds memory without starving later resources when one
    /// platform is unavailable. Each check uses the same lease and policy as a
    /// webhook; a failed candidate never aborts the rest of the sweep.
    pub async fn run_scheduled_update_checks(
        &self,
        cancel: &CancellationToken,
    ) -> Result<usize, SwarmServiceError> {
        let mut after = None;
        let mut checked = 0;
        loop {
            if cancel.is_cancelled() {
                return Err(SwarmServiceError::Cancelled);
            }
            let ids = self.store.update_check_candidates(after, 25).await?;
            if ids.is_empty() {
                return Ok(checked);
            }
            for id in ids {
                after = Some(id);
                if cancel.is_cancelled() {
                    return Err(SwarmServiceError::Cancelled);
                }
                let snapshot = match self
                    .store
                    .get_authorized(ActorId::new(Uuid::from_u128(1)), true, id)
                    .await
                {
                    Ok(snapshot) => snapshot,
                    Err(SwarmServiceError::NotFound) => continue,
                    Err(error) => return Err(error),
                };
                match self
                    .check_automated_updates_mode(snapshot, true, cancel)
                    .await
                {
                    Ok(_) => checked += 1,
                    Err(SwarmServiceError::Cancelled) => return Err(SwarmServiceError::Cancelled),
                    Err(error) => tracing::warn!(%id, %error, "Service image update check failed"),
                }
            }
        }
    }
}
