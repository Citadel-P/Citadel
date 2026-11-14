use super::*;

/// A durable claim and its execution capacity must travel together until cleanup
/// and result persistence complete. The dispatcher always hands this to an owned task.
#[must_use = "a claimed Script must be executed, including during shutdown"]
pub struct ClaimedScriptRun {
    claim: ScriptRunClaim,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl ScriptService {
    async fn claim_next(
        &self,
        stale_before: DateTime<Utc>,
        remote: bool,
    ) -> Result<Option<ScriptRunClaim>, ScriptError> {
        let result = self.store.claim_next(stale_before, remote).await?;
        for script_id in result.recovered_scripts {
            self.changed(Some(script_id));
        }
        Ok(result.claim)
    }

    pub async fn claim_execution(
        &self,
        remote_first: bool,
    ) -> Result<Option<ClaimedScriptRun>, ScriptError> {
        if !self.options.enabled {
            return Ok(None);
        }
        let stale_before = Utc::now()
            - chrono::Duration::from_std(self.stale_after)
                .map_err(|error| ScriptError::Storage(error.to_string()))?;
        for remote in [remote_first, !remote_first] {
            let slots = if remote {
                &self.remote_slots
            } else {
                &self.slots
            };
            if let Ok(permit) = slots.clone().try_acquire_owned()
                && let Some(claim) = self.claim_next(stale_before, remote).await?
            {
                return Ok(Some(ClaimedScriptRun {
                    claim,
                    _permit: permit,
                }));
            }
        }
        Ok(None)
    }

    pub async fn execute_claimed(
        &self,
        execution: ClaimedScriptRun,
        cancellation: &CancellationToken,
    ) -> Result<(), ScriptError> {
        self.execute_claim(&execution.claim, cancellation, Some(&self.progress))
            .await
    }

    pub async fn process_one(&self, cancellation: &CancellationToken) -> Result<bool, ScriptError> {
        let Some(execution) = self.claim_execution(false).await? else {
            return Ok(false);
        };
        self.execute_claimed(execution, cancellation).await?;
        Ok(true)
    }
}
