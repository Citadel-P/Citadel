//! One private helper protocol for source authorization and target completion.
use super::super::{JournalDirectory, docker::LocalDocker};
use super::ExecutionJournal;
use super::{
    completion::{TargetUpdateCompletion, validate_completion},
    evidence::{HelperAcknowledgement, HelperBinding, HelperLaunchIntent, conflict},
    runtime::private_home,
    target::TargetUpdateEvidence,
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
    InspectJournal,
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
    Journal(Box<citadel_updates::UpdateJournal>),
}

impl HelperRequest {
    pub async fn exchange(
        &self,
        docker: &LocalDocker<'_>,
        user: &str,
        timeout: Duration,
    ) -> Result<HelperResponse, UpdateError> {
        let id = &self.binding.helper_container_id;
        let input = serde_json::to_vec(self).map_err(|_| conflict())?;
        if input.len() > MAX_HANDOFF_REQUEST {
            return Err(conflict());
        }
        loop {
            let output = docker
                .run_with_input(
                    &[
                        "exec",
                        "--interactive",
                        "--env",
                        "TMPDIR=/tmp",
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
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(30),
    };
    // Docker start confirms the process, not publication of its journal. Only
    // the read-only readiness action may wait for that initial acknowledgement.
    if matches!(request.action, HelperAction::Check)
        && state
            .read_optional::<ExecutionJournal>(&format!("{}.json", intent.operation.id))?
            .is_none()
    {
        intent.verify_saved(&state)?;
        if request.operation != intent.operation
            || super::evidence::binding(&state, &intent)? != request.binding
        {
            return Err(conflict());
        }
        verification::acknowledged_helper(
            &docker,
            &intent,
            &HelperAcknowledgement {
                binding: request.binding.clone(),
                started_at: request.started_at.clone(),
            },
        )
        .await?;
        return serde_json::to_vec(&HelperResponse::Pending).map_err(|_| conflict());
    }
    let execution = ExecutionJournal::read_for(&state, &intent)?;
    let journal = &execution.journal;
    journal.validate_for(&request.operation)?;
    let ack = &execution.acknowledgement;
    if ack.binding != request.binding
        || ack.started_at != request.started_at
        || journal.helper_container_id != ack.binding.helper_container_id
    {
        return Err(conflict());
    }
    verification::acknowledged_helper(&docker, &intent, ack).await?;
    let response = match request.action {
        HelperAction::InspectJournal => HelperResponse::Journal(Box::new(execution.journal)),
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
            // The driver may finish preparation while Core checks readiness.
            // Read that result together; Grant still requires its reviewed snapshot.
            let current = ExecutionJournal::read_for(&state, &intent)?;
            if current.acknowledgement != *ack
                || current.journal != *journal
                || (matches!(request.action, HelperAction::Grant) && current != execution)
            {
                return Err(conflict());
            }
            if cancellation.is_cancelled() {
                return Err(UpdateError::Cancelled);
            }
            if matches!(request.action, HelperAction::Check) && current.prepared.is_none() {
                HelperResponse::Pending
            } else {
                current.prepared_target(&intent, true)?;
                if matches!(request.action, HelperAction::Grant) {
                    state.create(&intent.file("shutdown-grant"), ack)?;
                }
                HelperResponse::Acknowledged(ack.clone())
            }
        }
        action => {
            let evidence = ExecutionJournal::read_replacement(&state, &intent)?
                .target_evidence(&state, &intent, process, cancellation)
                .await?;
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
