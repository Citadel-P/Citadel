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
        let evidence = ExecutionJournal::read_replacement(&self.state, &self.intent)?;
        evidence.require_submission()?;
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
        let existing = &evidence.target;
        if existing.as_ref().is_some_and(|saved| saved != &target) {
            return Err(conflict());
        }
        // Repeat the live read before publication to fence an incarnation change
        // during a multi-resource observation. Health samples are not identity.
        if evidence.observe(&docker, &self.intent).await? != target
            || ExecutionJournal::read_replacement(&self.state, &self.intent)? != evidence
        {
            return Err(conflict());
        }
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
            let mut execution = evidence;
            execution.target = Some(target);
            execution.journal = journal.clone();
            execution.save(&self.state, &self.owner)?;
        }
        // No verified_target or completion flags: only the target may establish
        // catalog/readiness and commit completion. No source-schema SQL runs here.
        Ok(journal)
    }
}

impl ExecutionJournal {
    async fn observe(
        &self,
        docker: &LocalDocker<'_>,
        intent: &HelperLaunchIntent,
    ) -> Result<ObservedReplacement, UpdateError> {
        let replacement = self.replacement.as_ref().ok_or_else(conflict)?;
        let observed = observe_replacement(
            docker,
            &self.target_registration(intent)?,
            &replacement.configuration,
            Some(&self.acknowledgement.binding.helper_container_id),
        )
        .await?;
        verification::acknowledged_helper(docker, intent, &self.acknowledgement).await?;
        if verification::release_image(docker, &intent.operation.plan.target).await?
            != replacement.target_image_id
            || observed.image_id != replacement.target_image_id
            || (intent.operation.plan.database.is_none()
                && observed.retained_runtime_sha256 != replacement.retained_runtime_sha256)
        {
            return Err(conflict());
        }
        if let Some(database) = &replacement.database {
            database.verify(docker, intent).await?;
        }
        Ok(observed)
    }

    /// A replacing journal may wait for Compose submission and target binding. All
    /// later phases require the exact saved binding and the unchanged live runtime.
    pub(super) async fn target_evidence(
        &self,
        state: &JournalDirectory,
        intent: &HelperLaunchIntent,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<Option<TargetUpdateEvidence>, UpdateError> {
        if self.journal.operation.phase == UpdatePhase::ReplacingCore {
            return Ok(None);
        }
        if self.journal.operation.phase != UpdatePhase::Verifying {
            return Err(conflict());
        }
        self.require_submission()?;
        let target = self.target.as_ref().ok_or_else(conflict)?;
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(120),
        };
        if self.observe(&docker, intent).await? != *target
            || ExecutionJournal::read_replacement(state, intent)? != *self
            || cancellation.is_cancelled()
        {
            return Err(conflict());
        }
        let recovery_keys = super::keys::TargetRecoveryKeys::from_checkpoint(
            state,
            &self.replacement.as_ref().ok_or_else(conflict)?.checkpoint,
        )?;
        Ok(Some(TargetUpdateEvidence {
            intent: intent.clone(),
            journal: self.journal.clone(),
            container_id: target.container_id.clone(),
            started_at: target.started_at.clone(),
            source_catalog: citadel_database::MIGRATION_MANIFEST_JSON.into(),
            recovery_keys,
        }))
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
