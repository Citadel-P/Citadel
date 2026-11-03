//! One private helper protocol for source authorization and target completion.
use super::super::{JournalDirectory, docker::LocalDocker};
use super::ExecutionJournal;
use super::{
    completion::{TargetUpdateCompletion, validate_completion},
    evidence::{HelperAcknowledgement, HelperBinding, HelperLaunchIntent, binding, conflict},
    runtime::private_home,
    target::{TargetUpdateEvidence, target_evidence},
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdateOperation, UpdatePhase};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub const MAX_HANDOFF_REQUEST: usize = 256 * 1024;

#[derive(Serialize, Deserialize)]
#[serde(
    tag = "action",
    content = "completion",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(super) enum HelperAction {
    Check,
    Grant,
    InspectTarget,
    CompleteTarget(Box<TargetUpdateCompletion>),
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HelperRequest {
    pub operation: UpdateOperation,
    pub binding: HelperBinding,
    pub started_at: String,
    pub action: HelperAction,
}

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "status",
    content = "evidence",
    rename_all = "camelCase",
    deny_unknown_fields
)]
pub(super) enum HelperResponse {
    Acknowledged(HelperAcknowledgement),
    Pending,
    Ready(Box<TargetUpdateEvidence>),
}

impl HelperRequest {
    pub async fn exchange(
        &self,
        docker: &LocalDocker<'_>,
        id: &str,
        user: &str,
        timeout: Duration,
    ) -> Result<HelperResponse, UpdateError> {
        let input = serde_json::to_vec(self).map_err(|_| conflict())?;
        if input.len() > MAX_HANDOFF_REQUEST {
            return Err(conflict());
        }
        let temporary = format!("TMPDIR={}", self.operation.plan.checkpoint_directory);
        loop {
            let output = docker
                .run_with_input(
                    &[
                        "exec",
                        "--interactive",
                        "--env",
                        &temporary,
                        "--user",
                        user,
                        id,
                        "/app/citadel-server",
                        "update-helper-handoff",
                    ],
                    input.clone(),
                    timeout,
                )
                .await?;
            let response: HelperResponse =
                serde_json::from_slice(&output).map_err(|_| conflict())?;
            if response != HelperResponse::Pending {
                return Ok(response);
            }
            // Only read actions may wait. Never retry grants or completion writes.
            if !matches!(
                self.action,
                HelperAction::Check | HelperAction::InspectTarget
            ) {
                return Err(conflict());
            }
            let helper = verification::container(docker, id).await?;
            if helper["State"]["Running"] != true
                || helper["State"]["StartedAt"] != self.started_at
                || helper["RestartCount"] != 0
                || Instant::now() >= docker.deadline
            {
                return Err(conflict());
            }
            tokio::select! {
                biased;
                () = docker.cancellation.cancelled() => return Err(UpdateError::Cancelled),
                () = tokio::time::sleep(Duration::from_millis(250)) => (),
            }
        }
    }
}

/// Only the helper opens host state. The driver owns journal writes; requests
/// may publish a shutdown grant or completion mailbox, never execute a phase.
pub async fn execute_helper_handoff(
    bytes: &[u8],
    process: &dyn ProcessRunner,
    cancellation: &CancellationToken,
) -> Result<Vec<u8>, UpdateError> {
    if bytes.len() > MAX_HANDOFF_REQUEST {
        return Err(conflict());
    }
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    let request: HelperRequest = serde_json::from_slice(bytes).map_err(|_| conflict())?;
    let state = JournalDirectory::open(request.operation.plan.checkpoint_directory.clone().into())?;
    let intent: HelperLaunchIntent =
        state.read(&format!("launch-{}.json", request.operation.id))?;
    intent.verify_saved_identity(&state)?;
    state.require_lock_held()?;
    let execution = ExecutionJournal::read(&state, intent.operation.id)?;
    let journal = execution.journal;
    journal.validate_for(&request.operation)?;
    journal.validate_for(&intent.operation)?;
    let ack = execution.acknowledgement;
    if ack.binding != binding(&state, &intent)?
        || ack.binding != request.binding
        || ack.started_at != request.started_at
        || journal.helper_container_id != ack.binding.helper_container_id
    {
        return Err(conflict());
    }
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(30),
    };
    if verification::helper(&docker, &intent, &ack.binding)
        .await?
        .started_at
        != ack.started_at
    {
        return Err(conflict());
    }
    let response = match request.action {
        HelperAction::Check | HelperAction::Grant => {
            intent.verify_saved(&state)?;
            if request.operation != intent.operation
                || journal.operation.phase != UpdatePhase::Draining
                || journal.operation.revision != 1
                || journal.operation.checkpoint.is_some()
                || journal.verified_target.is_some()
                || journal.maintenance_released
                || journal.completion_acknowledged
            {
                return Err(conflict());
            }
            state.require_lock_held()?;
            if ExecutionJournal::read(&state, intent.operation.id)?.acknowledgement != ack
                || state.journal(intent.operation.id)? != journal
            {
                return Err(conflict());
            }
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            if matches!(request.action, HelperAction::Check)
                && ExecutionJournal::read(&state, intent.operation.id)?
                    .prepared
                    .is_none()
            {
                HelperResponse::Pending
            } else {
                super::preparation::check_prepared_target(&state, &intent, &ack, true)?;
                if matches!(request.action, HelperAction::Grant) {
                    state.create(&intent.file("shutdown-grant"), &ack)?;
                }
                HelperResponse::Acknowledged(ack)
            }
        }
        action => {
            let evidence = target_evidence(&state, &intent, process, cancellation).await?;
            match (action, evidence) {
                (HelperAction::InspectTarget, None) => HelperResponse::Pending,
                (HelperAction::InspectTarget, Some(evidence)) => {
                    HelperResponse::Ready(Box::new(evidence))
                }
                (HelperAction::CompleteTarget(completion), Some(evidence)) => {
                    validate_completion(&evidence.journal, &completion)?;
                    let name = intent.file("target-completion");
                    match state.read_optional::<TargetUpdateCompletion>(&name)? {
                        Some(saved) if saved == *completion => state.sync()?,
                        Some(_) => return Err(conflict()),
                        None => state.create(&name, &completion)?,
                    }
                    HelperResponse::Ready(Box::new(evidence))
                }
                _ => return Err(conflict()),
            }
        }
    };
    serde_json::to_vec(&response).map_err(|_| conflict())
}
