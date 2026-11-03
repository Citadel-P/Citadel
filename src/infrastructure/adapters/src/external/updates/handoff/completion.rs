//! Target-owned completion; the source helper never reads target-schema SQL.
use super::super::docker::LocalDocker;
use super::{
    OwnedUpdateHelper, evidence::conflict, runtime::private_home, target::target_evidence,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{CompletionReceipt, UpdateError, UpdateJournal};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetUpdateCompletion {
    pub journal: UpdateJournal,
    pub receipt: CompletionReceipt,
}

pub(super) fn validate_completion(
    journal: &UpdateJournal,
    completion: &TargetUpdateCompletion,
) -> Result<(), UpdateError> {
    let expected = journal.completed(
        &journal.operation.plan.target,
        completion.journal.operation.updated_at,
    )?;
    if completion.journal != expected {
        return Err(conflict());
    }
    let mut released = expected;
    released.completion_acknowledged = true;
    released.maintenance_released = true;
    completion
        .receipt
        .permits_resume(&released, &journal.operation.plan.target)
}

impl OwnedUpdateHelper {
    /// Only a target-committed receipt can complete the helper. The caller then
    /// exits; Core still has to observe that exact helper exit before DB release.
    pub async fn accept_target_completion(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<bool, UpdateError> {
        self.state.verify_lock(&self.owner)?;
        let Some(completion) = self
            .state
            .read_optional::<TargetUpdateCompletion>(&self.intent.file("target-completion"))?
        else {
            return Ok(false);
        };
        let evidence = target_evidence(&self.state, &self.intent, process, cancellation)
            .await?
            .ok_or_else(conflict)?;
        validate_completion(&evidence.journal, &completion)?;
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(45),
        };
        let output = docker
            .run_with_input(
                &[
                    "exec",
                    "--interactive",
                    "--user",
                    "65532:0",
                    &evidence.container_id,
                    "/app/citadel-server",
                    "update-target-receipt",
                ],
                serde_json::to_vec(&completion).map_err(|_| conflict())?,
                Duration::from_secs(30),
            )
            .await?;
        let committed: CompletionReceipt =
            serde_json::from_slice(&output).map_err(|_| conflict())?;
        if committed != completion.receipt
            || target_evidence(&self.state, &self.intent, process, cancellation)
                .await?
                .as_ref()
                != Some(&evidence)
        {
            return Err(conflict());
        }
        let mut journal = completion.journal;
        journal.completion_acknowledged = true;
        // Runtime cleanup has not happened yet. The target must not interpret
        // helper exit or this acknowledgement as database admission permission.
        self.save_journal(journal.clone())?;
        Ok(true)
    }
}
