//! Bind the replacement process before target-owned startup acceptance.
use super::super::{
    JournalDirectory,
    bootstrap::{ObservedReplacement, observe_replacement},
    docker::LocalDocker,
};
use super::ExecutionJournal;
use super::{
    OwnedUpdateHelper,
    evidence::{HelperLaunchIntent, conflict},
    replacement::ReplacementEvidence,
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateJournal, UpdatePhase};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

impl OwnedUpdateHelper {
    /// Read-only daemon reconciliation after a confirmed Compose submission.
    /// Repeating an observation never repeats replacement. A saved binding pins
    /// one target incarnation; a restart or different container requires recovery.
    pub async fn observe_replacement(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<UpdateJournal, UpdateError> {
        self.state.verify_lock(&self.owner)?;
        let evidence = ReplacementEvidence::read(&self.state, &self.intent)?;
        evidence.require_submission(&self.state, &self.intent)?;
        if evidence.acknowledgement != self.acknowledgement {
            return Err(conflict());
        }
        let mut journal = evidence.journal.clone();
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(120),
        };
        let target = evidence.observe(&docker, &self.intent).await?;
        let mut execution = ExecutionJournal::read(&self.state, self.intent.operation.id)?;
        let existing = &execution.target;
        if existing.as_ref().is_some_and(|saved| saved != &target) {
            return Err(conflict());
        }
        // Repeat the live read before publication to fence an incarnation change
        // during a multi-resource observation. Health samples are not identity.
        if evidence.observe(&docker, &self.intent).await? != target
            || ReplacementEvidence::read(&self.state, &self.intent)? != evidence
        {
            return Err(conflict());
        }
        evidence.require_submission(&self.state, &self.intent)?;
        self.state.verify_lock(&self.owner)?;
        if cancellation.is_cancelled() {
            return Err(UpdateError::Cancelled);
        }
        if journal.operation.phase == UpdatePhase::ReplacingCore {
            journal
                .operation
                .advance(3, UpdatePhase::Verifying, chrono::Utc::now())?;
            journal.history.push(journal.operation.clone());
            journal.validate_for(&self.intent.operation)?;
            execution.target = Some(target);
            execution.journal = journal.clone();
            execution.save(&self.state, &self.owner)?;
        }
        // No verified_target or completion flags: only the target may establish
        // catalog/readiness and commit completion. No source-schema SQL runs here.
        Ok(journal)
    }
}

impl ReplacementEvidence {
    async fn observe(
        &self,
        docker: &LocalDocker<'_>,
        intent: &HelperLaunchIntent,
    ) -> Result<ObservedReplacement, UpdateError> {
        let observed = observe_replacement(
            docker,
            &intent.registration,
            &self.replacement.configuration,
            &self.acknowledgement.binding.helper_container_id,
        )
        .await?;
        let helper = verification::helper(docker, intent, &self.acknowledgement.binding).await?;
        if helper.started_at != self.acknowledgement.started_at
            || verification::release_image(docker, &intent.operation.plan.target).await?
                != self.replacement.target_image_id
            || observed.image_id != self.replacement.target_image_id
            || observed.retained_runtime_sha256 != self.replacement.retained_runtime_sha256
        {
            return Err(conflict());
        }
        Ok(observed)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TargetUpdateEvidence {
    pub intent: HelperLaunchIntent,
    pub journal: UpdateJournal,
    pub container_id: String,
    pub started_at: String,
    pub source_catalog: String,
    pub recovery_keys: super::keys::TargetRecoveryKeys,
}

/// A replacing journal may wait for Compose submission and target binding. All
/// later phases require the exact saved binding and the unchanged live runtime.
pub(super) async fn target_evidence(
    state: &JournalDirectory,
    intent: &HelperLaunchIntent,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<Option<TargetUpdateEvidence>, UpdateError> {
    let evidence = ReplacementEvidence::read(state, intent)?;
    if evidence.journal.operation.phase == UpdatePhase::ReplacingCore {
        return Ok(None);
    }
    evidence.require_submission(state, intent)?;
    let target = ExecutionJournal::read(state, intent.operation.id)?
        .target
        .ok_or_else(conflict)?;
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(120),
    };
    if evidence.observe(&docker, intent).await? != target
        || ReplacementEvidence::read(state, intent)? != evidence
        || cancellation.is_cancelled()
    {
        return Err(conflict());
    }
    evidence.require_submission(state, intent)?;
    Ok(Some(TargetUpdateEvidence {
        intent: intent.clone(),
        journal: evidence.journal,
        container_id: target.container_id,
        started_at: target.started_at,
        source_catalog: citadel_database::MIGRATION_MANIFEST_JSON.into(),
        recovery_keys: super::keys::TargetRecoveryKeys::from_checkpoint(
            state,
            &evidence.replacement.checkpoint,
        )?,
    }))
}
