//! One-shot Core replacement after an integrity-checked recovery checkpoint.
use super::super::{
    CheckedCoreReplacement, InstallerComposeInputs, JournalDirectory, PreparedUpdateCheckpoint,
    checkpoint::verify_checkpoint_integrity, docker::LocalDocker,
};
use super::ExecutionJournal;
use super::{
    OwnedUpdateHelper,
    evidence::{HelperAcknowledgement, HelperLaunchIntent, conflict},
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateJournal, UpdatePhase};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct CoreReplacementIntent {
    pub(super) configuration: CheckedCoreReplacement,
    pub(super) checkpoint: PreparedUpdateCheckpoint,
    pub(super) target_image_id: String,
    pub(super) retained_runtime_sha256: String,
}

impl OwnedUpdateHelper {
    /// Submit exactly one Core-only Compose replacement. This does not verify
    /// target startup, import completion, release maintenance or resume source.
    /// Await cancellation cleanup; any error retains intent and prohibits replay.
    pub async fn replace_core(
        &self,
        process: &dyn ProcessRunner,
        cancellation: &CancellationToken,
    ) -> Result<UpdateJournal, UpdateError> {
        let mut execution = self.check_grant()?;
        let mut journal = execution.journal.clone();
        if journal.operation.phase != UpdatePhase::BackingUp || journal.operation.revision != 2 {
            return Err(conflict());
        }
        let home = private_home()?;
        let docker = LocalDocker {
            process,
            cancellation,
            home: home.path(),
            deadline: Instant::now() + Duration::from_secs(600),
        };
        let checkpoint =
            verify_checkpoint_integrity(&self.state, &self.owner, &journal.operation, cancellation)
                .await?;
        let checked = execution.prepared_target(&self.intent, false)?.clone();
        let reg = &self.intent.registration;
        let inputs = InstallerComposeInputs {
            directory: reg.directory.clone(),
            project: reg.project.clone(),
            compose_files: reg.compose_files.clone(),
        };
        let candidate = inputs
            .prepare_replacement(
                &self.intent.operation.plan.target.image,
                process,
                cancellation,
            )
            .await?;
        if candidate.checked != checked {
            return Err(conflict());
        }
        let target_image_id =
            verification::release_image(&docker, &self.intent.operation.plan.target).await?;
        let retained_runtime_sha256 = self.check_stopped_runtime(process, cancellation).await?;
        let core = verification::container(&docker, &reg.core_container_id).await?;
        let database = verification::container(&docker, &reg.database_container_id).await?;
        if core["Image"] == target_image_id {
            return Err(conflict());
        }
        if self.check_grant()? != execution || cancellation.is_cancelled() {
            return Err(conflict());
        }
        let intent = CoreReplacementIntent {
            retained_runtime_sha256,
            configuration: checked,
            checkpoint,
            target_image_id,
        };
        if execution.replacement.is_some() {
            return Err(conflict());
        }
        journal.operation.checkpoint = Some(
            intent
                .checkpoint
                .bundle
                .to_str()
                .ok_or_else(conflict)?
                .into(),
        );
        journal
            .operation
            .advance(2, UpdatePhase::ReplacingCore, chrono::Utc::now())?;
        journal.history.push(journal.operation.clone());
        journal.validate_for(&self.intent.operation)?;
        execution.replacement = Some(intent.clone());
        execution.journal = journal.clone();
        execution.save(&self.state, &self.owner)?;
        candidate.publish(&inputs, cancellation)?;

        // The original fingerprint must no longer match after publication. Verify
        // the approved target fingerprint explicitly; do not weaken check_grant.
        self.state.verify_lock(&self.owner)?;
        if reg.fingerprint(std::path::Path::new("/"))?
            != intent.configuration.target_configuration_sha256
            || ExecutionJournal::read_for(&self.state, &self.intent)? != execution
        {
            return Err(conflict());
        }
        verification::acknowledged_helper(&docker, &self.intent, &self.acknowledgement).await?;
        let current = verification::container(&docker, &reg.core_container_id).await?;
        same_runtime(&core, &current)?;
        if current["State"]["Running"] != false
            || current["State"]["Status"] != "exited"
            || current["State"]["ExitCode"] != 0
            || current["State"]["OOMKilled"] != false
            || current["State"]["Error"] != ""
            || current["State"]["Paused"] != false
            || current["State"]["Restarting"] != false
        {
            return Err(conflict());
        }
        let current = verification::container(&docker, &reg.database_container_id).await?;
        same_runtime(&database, &current)?;
        verification::database_running(&current)?;
        let mut args = vec![
            "compose",
            "--project-name",
            &reg.project,
            "--project-directory",
            reg.directory.to_str().ok_or_else(conflict)?,
            "--env-file",
            reg.environment_file.to_str().ok_or_else(conflict)?,
        ];
        for path in &reg.compose_files {
            args.extend(["--file", path.to_str().ok_or_else(conflict)?]);
        }
        args.extend([
            "up",
            "--detach",
            "--no-deps",
            "--no-build",
            "--pull",
            "never",
            "server",
        ]);
        self.state.verify_lock(&self.owner)?;
        if reg.fingerprint(std::path::Path::new("/"))?
            != intent.configuration.target_configuration_sha256
            || ExecutionJournal::read_for(&self.state, &self.intent)? != execution
        {
            return Err(conflict());
        }
        docker
            .run_with_timeout(&args, Duration::from_secs(300))
            .await
            .map_err(|error| match error {
                UpdateError::Cancelled => UpdateError::Cancelled,
                _ => UpdateError::Unavailable("Core replacement was not confirmed. Preserve update state for recovery; do not retry or restart the source.".into()),
            })?;
        self.state.verify_lock(&self.owner)?;
        if ExecutionJournal::read_for(&self.state, &self.intent)? != execution {
            return Err(conflict());
        }
        // Only submission evidence. No old-schema database access after this point.
        execution.submitted = true;
        execution.save(&self.state, &self.owner)?;
        Ok(journal)
    }
}

fn same_runtime(before: &Value, after: &Value) -> Result<(), UpdateError> {
    if [
        "Id",
        "Image",
        "Config",
        "HostConfig",
        "Mounts",
        "RestartCount",
    ]
    .iter()
    .any(|key| before[key] != after[key])
        || before["State"]["StartedAt"] != after["State"]["StartedAt"]
    {
        return Err(conflict());
    }
    Ok(())
}

impl ExecutionJournal {
    /// Validate replacement evidence from one snapshot. Reading it never retries
    /// a mutation or assumes submission has completed.
    pub(super) fn read_replacement(
        state: &JournalDirectory,
        intent: &HelperLaunchIntent,
    ) -> Result<Self, UpdateError> {
        let execution = Self::read_for(state, intent)?;
        let acknowledgement = &execution.acknowledgement;
        if state.read::<HelperAcknowledgement>(&intent.file("shutdown-grant"))? != *acknowledgement
            || execution.stop.is_none()
        {
            return Err(conflict());
        }
        let replacement = execution.replacement.as_ref().ok_or_else(conflict)?;
        let checked = execution.prepared_target(intent, false)?;
        let journal = &execution.journal;
        let completed = journal.operation.phase == UpdatePhase::Succeeded
            && journal.operation.revision == 5
            && journal.verified_target.as_ref() == Some(&intent.operation.plan.target)
            && journal.completion_acknowledged;
        let checkpoint = &replacement.checkpoint;
        let stage = JournalDirectory::open(
            state
                .path()
                .join(format!("checkpoint-{}", journal.operation.id)),
        )?;
        if replacement.configuration != *checked
            || !citadel_updates::is_sha256(&replacement.retained_runtime_sha256)
            || !replacement
                .target_image_id
                .strip_prefix("sha256:")
                .is_some_and(citadel_updates::is_sha256)
            || intent.registration.fingerprint(std::path::Path::new("/"))?
                != checked.target_configuration_sha256
            || !(completed
                || matches!(
                    (journal.operation.phase, journal.operation.revision),
                    (UpdatePhase::ReplacingCore, 3) | (UpdatePhase::Verifying, 4)
                ))
            || journal.helper_container_id != acknowledgement.binding.helper_container_id
            || (!completed
                && (journal.verified_target.is_some() || journal.completion_acknowledged))
            || journal.maintenance_released
            || checkpoint.operation_id != journal.operation.id
            || checkpoint.fence != journal.operation.fence
            || checkpoint.instance_id != journal.operation.instance_id
            || checkpoint.source != journal.operation.plan.source
            || checkpoint.configuration_sha256 != checked.source_configuration_sha256
            || checkpoint.bundle != stage.path().join(journal.operation.id.simple().to_string())
            || checkpoint.bundle.to_str() != journal.operation.checkpoint.as_deref()
            || stage.read::<PreparedUpdateCheckpoint>("checkpoint.json")? != *checkpoint
        {
            return Err(conflict());
        }
        Ok(execution)
    }

    pub(super) fn require_submission(&self) -> Result<(), UpdateError> {
        if !self.submitted {
            return Err(conflict());
        }
        Ok(())
    }
}
