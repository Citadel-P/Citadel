//! Reconcile a committed target after the detached executor exits.
use super::super::{
    JournalDirectory, RegisteredComposeInstallation, bootstrap::observe_replacement,
    docker::LocalDocker,
};
use super::{
    ExecutionJournal, HelperLaunchIntent, TargetUpdateCompletion, evidence::conflict,
    runtime::private_home, verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateJournal, UpdatePhase};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FinalizeUpdateRequest {
    pub intent: HelperLaunchIntent,
    pub completion: TargetUpdateCompletion,
}

/// Runs in the registered host namespace. The caller still owns the database
/// claim and must release it only after this result is verified by the target.
/// Any interrupted cleanup retains maintenance; it never replays replacement.
pub async fn finalize_update(
    request: &FinalizeUpdateRequest,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<UpdateJournal, UpdateError> {
    let intent = &request.intent;
    let state = JournalDirectory::open(intent.registration.state_directory.clone())?;
    let owner = state.existing_lock()?;
    let archive = JournalDirectory::open(
        state
            .path()
            .join(format!("checkpoint-{}", intent.operation.id)),
    )?;
    let pending: Option<FinalizeUpdateRequest> = archive.read_optional("finalization.json")?;
    if pending.as_ref().is_some_and(|saved| saved != request) {
        return Err(conflict());
    }
    let resuming = pending.is_some();
    let journal_name = format!("{}.json", intent.operation.id);
    let active = state
        .read_optional::<ExecutionJournal>(&journal_name)?
        .is_some();
    let mut execution = if resuming {
        ExecutionJournal::read(if active { &state } else { &archive }, intent.operation.id)?
    } else {
        ExecutionJournal::read_replacement(&state, intent)?
    };
    let mut acknowledged = request.completion.journal.clone();
    acknowledged.completion_acknowledged = true;
    request.completion.receipt.validate_for(&acknowledged)?;
    let mut saved = execution.journal.clone();
    saved.maintenance_released = false;
    if saved != acknowledged || acknowledged.operation.phase != UpdatePhase::Succeeded {
        return Err(conflict());
    }
    execution.require_submission()?;
    let replacement = execution.replacement.as_ref().ok_or_else(conflict)?;
    let target = execution.target.as_ref().ok_or_else(conflict)?;
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(120),
    };
    let helper = &execution.acknowledgement.binding.helper_container_id;
    let helpers = docker.containers(&format!("id={helper}")).await?;
    let helper_present = helpers == [helper.clone()];
    if (!helper_present && (!helpers.is_empty() || !resuming))
        || (helper_present
            && !verification::helper_exited(&docker, intent, &execution.acknowledgement).await?)
    {
        return Err(conflict());
    }
    let running_helper = helper_present.then_some(helper.as_str());
    let observed = observe_replacement(
        &docker,
        &intent.registration,
        &replacement.configuration,
        running_helper,
    )
    .await?;
    if observed != *target
        && !(resuming
            && observed.container_id == target.container_id
            && observed.image_id == target.image_id
            && observed.runtime_sha256 == target.runtime_sha256
            && observed.retained_runtime_sha256 == target.retained_runtime_sha256)
    {
        return Err(conflict());
    }
    // Ask target-owned code for the committed receipt; the reconciler never
    // opens the migrated database using source-schema queries.
    let bytes = docker
        .run_with_input(
            &[
                "exec",
                "--interactive",
                "--user",
                "65532:0",
                &target.container_id,
                "/app/citadel-server",
                "update-target-receipt",
            ],
            serde_json::to_vec(&request.completion).map_err(|_| conflict())?,
            Duration::from_secs(30),
        )
        .await?;
    let receipt: citadel_updates::CompletionReceipt =
        serde_json::from_slice(&bytes).map_err(|_| conflict())?;
    if ExecutionJournal::read(if active { &state } else { &archive }, intent.operation.id)?
        != execution
        || receipt != request.completion.receipt
        || (helper_present
            && !verification::helper_exited(&docker, intent, &execution.acknowledgement).await?)
        || observe_replacement(
            &docker,
            &intent.registration,
            &replacement.configuration,
            running_helper,
        )
        .await?
            != observed
    {
        return Err(conflict());
    }
    // Persist proof of clean exit and committed target acceptance before cleanup.
    // A restart may finish these exact file moves, never replay replacement.
    if !resuming {
        archive.create("finalization.json", request)?;
    }
    if helper_present {
        docker.run(&["container", "rm", helper]).await?;
    }
    if observe_replacement(
        &docker,
        &intent.registration,
        &replacement.configuration,
        None,
    )
    .await?
        != observed
        || cancellation.is_cancelled()
    {
        return Err(conflict());
    }
    let mut registration = intent.registration.clone();
    registration.core_container_id = target.container_id.clone();
    registration.configuration_sha256 = replacement
        .configuration
        .target_configuration_sha256
        .clone();
    registration.runtime_sha256 = target.runtime_sha256.clone();
    registration.validate()?;
    drop(home);
    execution.journal.maintenance_released = true;
    if active {
        execution.save(&state, &owner)?;
    } else if ExecutionJournal::read(&archive, intent.operation.id)? != execution {
        return Err(conflict());
    }
    // Each move is resumable only when its destination already contains the
    // retained record; a collision never overwrites either copy.
    for (name, expected) in [
        (intent.file("launch"), serde_json::to_value(intent)),
        (
            intent.file("binding"),
            serde_json::to_value(&execution.acknowledgement.binding),
        ),
        (
            intent.file("shutdown-grant"),
            serde_json::to_value(&execution.acknowledgement),
        ),
        (
            intent.file("target-completion"),
            serde_json::to_value(&request.completion),
        ),
        (journal_name, serde_json::to_value(&execution)),
    ] {
        let expected = expected.map_err(|_| conflict())?;
        if let Some(saved) = state.read_optional::<serde_json::Value>(&name)? {
            if saved != expected {
                return Err(conflict());
            }
            state.archive(&owner, &archive, &name)?;
        } else if archive.read::<serde_json::Value>(&name)? != expected {
            return Err(conflict());
        }
    }
    let current: RegisteredComposeInstallation = state.read("installation.json")?;
    if current != intent.registration && current != registration {
        return Err(conflict());
    }
    state.write("installation.json", &registration)?;
    let completed = CompletedUpdate {
        journal: execution.journal.clone(),
        registration,
    };
    match archive.read_optional::<CompletedUpdate>("completed.json")? {
        Some(saved) if saved != completed => return Err(conflict()),
        Some(_) => {}
        None => archive.create("completed.json", &completed)?,
    }
    state.require_registration_only()?;
    Ok(execution.journal)
}

#[derive(PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(in crate::external::updates) struct CompletedUpdate {
    pub journal: UpdateJournal,
    pub registration: RegisteredComposeInstallation,
}

impl CompletedUpdate {
    pub(in crate::external::updates) fn validate(
        &self,
        archive: &JournalDirectory,
    ) -> Result<(), UpdateError> {
        let operation = &self.journal.operation;
        self.journal.validate_for(operation)?;
        self.registration.validate()?;
        if operation.phase != UpdatePhase::Succeeded
            || !self.journal.maintenance_released
            || !self.journal.completion_acknowledged
            || self.registration.instance_id != operation.instance_id
            || self.registration.id != operation.plan.registration_id
            || archive.path()
                != self
                    .registration
                    .state_directory
                    .join(format!("checkpoint-{}", operation.id))
            || archive.journal(operation.id)? != self.journal
        {
            return Err(conflict());
        }
        Ok(())
    }
}
