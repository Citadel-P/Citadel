//! Reconcile a completed host archive when database release was interrupted.
use super::super::{
    JournalDirectory, RegisteredComposeInstallation, bootstrap::observe_accepted_target,
    docker::LocalDocker,
};
use super::{
    CompletedUpdate, ExecutionJournal, TargetRecoveryKeys, TargetUpdateCompletion,
    evidence::HelperLaunchIntent, evidence::conflict, runtime::private_home,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateJournal, UpdateOperation, UpdatePhase};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CompletedUpdateTarget {
    pub journal: UpdateJournal,
    pub completion: TargetUpdateCompletion,
    pub recovery_keys: TargetRecoveryKeys,
    pub container_id: String,
}

pub async fn completed_update_target(
    operation: &UpdateOperation,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<CompletedUpdateTarget, UpdateError> {
    if operation.phase != UpdatePhase::Succeeded {
        return Err(conflict());
    }
    let state = JournalDirectory::open(operation.plan.checkpoint_directory.clone().into())?;
    let archive =
        JournalDirectory::open(state.path().join(format!("checkpoint-{}", operation.id)))?;
    if let Some(pending) =
        archive.read_optional::<super::FinalizeUpdateRequest>("finalization.json")?
    {
        if pending.completion.journal.operation != *operation {
            return Err(conflict());
        }
        super::finalize_update(&pending, process, cancellation).await?;
    } else if archive
        .read_optional::<CompletedUpdate>("completed.json")?
        .is_none()
    {
        let intent: HelperLaunchIntent = state.read(&format!("launch-{}.json", operation.id))?;
        let execution = ExecutionJournal::read(&state, operation.id)?;
        let completion = execution.receipt_completion(operation)?;
        quiesce_accepted_helper(
            &state,
            &intent,
            &execution,
            &completion,
            process,
            cancellation,
        )
        .await?;
        super::finalize_update(
            &super::FinalizeUpdateRequest {
                intent,
                completion,
                receipt_recovery: true,
            },
            process,
            cancellation,
        )
        .await?;
    }
    let owner = state.existing_lock()?;
    state.require_registration_only()?;
    let archive =
        JournalDirectory::open(state.path().join(format!("checkpoint-{}", operation.id)))?;
    let completed: CompletedUpdate = archive.read("completed.json")?;
    completed.validate(&archive)?;
    let registration: RegisteredComposeInstallation = state.read("installation.json")?;
    let intent: HelperLaunchIntent = archive.read(&format!("launch-{}.json", operation.id))?;
    let completion: TargetUpdateCompletion = archive.read(&intent.file("target-completion"))?;
    let execution = ExecutionJournal::read(&archive, operation.id)?;
    let replacement = execution.replacement.as_ref().ok_or_else(conflict)?;
    let target = execution.target.as_ref().ok_or_else(conflict)?;
    let mut released = completion.journal.clone();
    released.completion_acknowledged = true;
    released.maintenance_released = true;
    completion
        .receipt
        .permits_resume(&released, &operation.plan.target)?;
    if completed.journal.operation != *operation
        || completed.journal != released
        || execution.receipt_completion(operation)? != completion
        || registration != completed.registration
        || execution.journal.history.first() != Some(&intent.operation)
        || registration.core_container_id != target.container_id
    {
        return Err(conflict());
    }
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(120),
    };
    if !docker
        .containers(&format!(
            "label=com.citadel.update-operation={}",
            operation.id
        ))
        .await?
        .is_empty()
    {
        return Err(conflict());
    }
    let observed = observe_accepted_target(
        &docker,
        &execution.target_registration(&intent)?,
        &replacement.configuration,
        None,
    )
    .await?;
    // The completed archive already proves the original incarnation's acceptance.
    // A restart of that same container may change StartedAt, but not its image,
    // configuration, persistent storage or the retained PostgreSQL incarnation.
    if observed.container_id != target.container_id
        || observed.image_id != target.image_id
        || observed.runtime_sha256 != target.runtime_sha256
        || observed.retained_runtime_sha256 != target.retained_runtime_sha256
    {
        return Err(conflict());
    }
    let recovery_keys = TargetRecoveryKeys::from_checkpoint(&state, &replacement.checkpoint)?;
    state.verify_lock(&owner)?;
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    Ok(CompletedUpdateTarget {
        journal: released,
        completion,
        recovery_keys,
        container_id: target.container_id.clone(),
    })
}

/// The target receipt, not process health, authorizes retirement of this exact
/// executor. Stopping the helper cannot rerun its replacement or migration work.
async fn quiesce_accepted_helper(
    state: &JournalDirectory,
    intent: &HelperLaunchIntent,
    execution: &ExecutionJournal,
    completion: &TargetUpdateCompletion,
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    intent.verify_saved_identity(state)?;
    execution.journal.validate_for(&intent.operation)?;
    if super::evidence::binding(state, intent)? != execution.acknowledgement.binding {
        return Err(conflict());
    }
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
    let actual =
        super::verification::helper_container(&docker, intent, &execution.acknowledgement.binding)
            .await?;
    if actual["State"]["StartedAt"] != execution.acknowledgement.started_at {
        return Err(conflict());
    }
    let observed = observe_accepted_target(
        &docker,
        &execution.target_registration(&intent)?,
        &replacement.configuration,
        Some(helper),
    )
    .await?;
    if observed.container_id != target.container_id
        || observed.image_id != target.image_id
        || observed.runtime_sha256 != target.runtime_sha256
        || observed.retained_runtime_sha256 != target.retained_runtime_sha256
    {
        return Err(conflict());
    }
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
            serde_json::to_vec(completion).map_err(|_| conflict())?,
            Duration::from_secs(30),
        )
        .await?;
    let receipt: citadel_updates::CompletionReceipt =
        serde_json::from_slice(&bytes).map_err(|_| conflict())?;
    if receipt != completion.receipt {
        return Err(conflict());
    }
    // Preserve daemon exit evidence too; a nonzero helper exit is never relabelled clean.
    if actual["State"]["Running"] == true {
        docker
            .run(&["container", "stop", "--time", "30", helper])
            .await?;
    }
    let exited =
        super::verification::helper_container(&docker, intent, &execution.acknowledgement.binding)
            .await?;
    if exited["State"]["Running"] != false
        || exited["State"]["Status"] != "exited"
        || exited["State"]["StartedAt"] != execution.acknowledgement.started_at
    {
        return Err(conflict());
    }
    let archive = JournalDirectory::open(
        state
            .path()
            .join(format!("checkpoint-{}", intent.operation.id)),
    )?;
    match archive.read_optional::<serde_json::Value>("executor-exit.json")? {
        Some(saved) if saved != exited["State"] => return Err(conflict()),
        Some(_) => {}
        None => archive.create("executor-exit.json", &exited["State"])?,
    }
    Ok(())
}
