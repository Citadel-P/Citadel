//! Owned detached execution. No phase is resumed by replaying a daemon mutation.
use super::super::{CheckpointRestoreBudget, JournalDirectory};
use super::ExecutionJournal;
use super::{
    OwnedUpdateHelper,
    evidence::{HelperAcknowledgement, HelperLaunchIntent, binding, conflict},
};
use citadel_execution::ProcessRunner;
use citadel_updates::{ReleaseInspectionService, UpdateError, UpdatePhase};
use std::{path::PathBuf, time::Duration};
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

pub struct UpdateHelperBudget {
    pub handoff: Duration,
    pub shutdown: Duration,
    pub checkpoint: CheckpointRestoreBudget,
    pub target: Duration,
}

impl Default for UpdateHelperBudget {
    fn default() -> Self {
        Self {
            handoff: Duration::from_secs(900),
            shutdown: Duration::from_secs(600),
            checkpoint: CheckpointRestoreBudget {
                timeout: Duration::from_secs(3600),
                maximum_bytes: 4 * 1024 * 1024 * 1024,
            },
            target: Duration::from_secs(900),
        }
    }
}

/// Invoked only by the launcher in the exact source-image helper. Owns all
/// futures through cancellation, including checkpoint sandbox cleanup. A restart
/// cannot acquire a fresh acknowledgement over an existing journal.
pub async fn run_update_helper(
    state_directory: PathBuf,
    operation_id: Uuid,
    fence: Uuid,
    inspector: &ReleaseInspectionService,
    process: &dyn ProcessRunner,
    budget: &UpdateHelperBudget,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    if operation_id.is_nil()
        || fence.is_nil()
        || budget.handoff.is_zero()
        || budget.handoff > Duration::from_secs(3600)
        || budget.target.is_zero()
        || budget.target > Duration::from_secs(3600)
    {
        return Err(conflict());
    }
    let state = JournalDirectory::open(state_directory)?;
    let intent: HelperLaunchIntent = state.read(&format!("launch-{operation_id}.json"))?;
    if intent.operation.id != operation_id || intent.operation.fence != fence {
        return Err(conflict());
    }
    let bound = binding(&state, &intent)?;
    let hostname = std::fs::read_to_string("/proc/sys/kernel/hostname").map_err(|_| conflict())?;
    let hostname = hostname.trim();
    if hostname != bound.helper_container_id && hostname != &bound.helper_container_id[..12] {
        return Err(conflict());
    }
    let helper = OwnedUpdateHelper::acknowledge(
        state,
        intent,
        &bound.helper_container_id,
        process,
        cancellation,
    )
    .await?;
    let result = execute(&helper, inspector, process, budget, cancellation).await;
    if result.is_err() {
        // Keep only a fixed safe message. Process and SQL errors may contain
        // credentials. Persist ambiguity even when the cancellation token is set.
        record_failure(&helper)?;
    }
    result
}

async fn execute(
    helper: &OwnedUpdateHelper,
    inspector: &ReleaseInspectionService,
    process: &dyn ProcessRunner,
    budget: &UpdateHelperBudget,
    cancellation: &CancellationToken,
) -> Result<(), UpdateError> {
    helper
        .prepare_target(inspector, process, cancellation)
        .await?;
    let deadline = Instant::now() + budget.handoff;
    loop {
        helper.state.verify_lock(&helper.owner)?;
        if helper
            .state
            .read_optional::<HelperAcknowledgement>(&helper.intent.file("shutdown-grant"))?
            .is_some()
        {
            helper.check_grant()?;
            break;
        }
        pause(deadline, cancellation).await?;
    }
    helper
        .stop_source(process, budget.shutdown, cancellation)
        .await?;
    helper
        .prepare_checkpoint(process, &budget.checkpoint, cancellation)
        .await?;
    helper.replace_core(process, cancellation).await?;
    helper.observe_replacement(process, cancellation).await?;
    let deadline = Instant::now() + budget.target;
    loop {
        if helper
            .accept_target_completion(process, cancellation)
            .await?
        {
            return Ok(());
        }
        pause(deadline, cancellation).await?;
    }
}

async fn pause(deadline: Instant, cancellation: &CancellationToken) -> Result<(), UpdateError> {
    if Instant::now() >= deadline {
        return Err(UpdateError::Unavailable(
            "Update handoff timed out; preserve the operation state for recovery.".into(),
        ));
    }
    tokio::select! {
        biased;
        () = cancellation.cancelled() => Err(UpdateError::Cancelled),
        () = tokio::time::sleep_until(deadline.min(Instant::now() + Duration::from_millis(250))) => Ok(()),
    }
}

pub(super) fn record_failure(helper: &OwnedUpdateHelper) -> Result<(), UpdateError> {
    helper.state.verify_lock(&helper.owner)?;
    helper.intent.verify_saved_identity(&helper.state)?;
    let mut journal = helper.state.journal(helper.intent.operation.id)?;
    journal.validate_for(&helper.intent.operation)?;
    if journal.operation.phase.terminal() {
        return Ok(());
    }
    let stop = ExecutionJournal::read(&helper.state, helper.intent.operation.id)?.stop;
    let phase = if stop.is_none() && journal.operation.phase == UpdatePhase::Draining {
        UpdatePhase::Failed
    } else {
        UpdatePhase::RecoveryRequired
    };
    journal
        .operation
        .advance(journal.operation.revision, phase, chrono::Utc::now())?;
    journal.operation.message = Some(if phase == UpdatePhase::Failed {
        "Update preparation failed before shutdown. Source verification is required before maintenance can end."
    } else {
        "Update execution was interrupted. Preserve the checkpoint and operation state; explicit recovery is required."
    }.into());
    journal.history.push(journal.operation.clone());
    journal.validate_for(&helper.intent.operation)?;
    helper.save_journal(journal)
}
