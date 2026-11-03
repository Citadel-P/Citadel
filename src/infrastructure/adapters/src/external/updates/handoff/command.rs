//! Helper-side endpoint. Only this process opens or writes the private host state.
use super::super::{JournalDirectory, docker::LocalDocker};
use super::{
    evidence::{HelperAcknowledgement, HelperBinding, HelperLaunchIntent, binding, conflict},
    runtime::private_home,
    verification,
};
use citadel_execution::ProcessRunner;
use citadel_updates::{UpdateError, UpdatePhase};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tokio_util::sync::CancellationToken;

pub const MAX_HANDOFF_REQUEST: usize = 256 * 1024;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub(super) enum HandoffAction {
    Check,
    Grant,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HandoffRequest {
    pub intent: HelperLaunchIntent,
    pub binding: HelperBinding,
    pub started_at: String,
    pub action: HandoffAction,
}

/// Invoked through Docker exec in the verified helper, not through an HTTP route.
/// A cancelled exec may have published a grant; never infer resume or retry safety.
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
    let request: HandoffRequest = serde_json::from_slice(bytes).map_err(|_| conflict())?;
    let state = JournalDirectory::open(request.intent.registration.state_directory.clone())?;
    request.intent.verify_saved(&state)?;
    let expected = binding(&state, &request.intent)?;
    let ack: HelperAcknowledgement = state.read(&request.intent.file("ack"))?;
    if ack.binding != expected
        || ack.binding != request.binding
        || ack.started_at != request.started_at
        || ack.incarnation.is_nil()
    {
        return Err(conflict());
    }
    state.require_lock_held()?;
    let home = private_home()?;
    let docker = LocalDocker {
        process,
        cancellation,
        home: home.path(),
        deadline: Instant::now() + Duration::from_secs(30),
    };
    let observed = verification::helper(&docker, &request.intent, &expected).await?;
    if observed.started_at != ack.started_at {
        return Err(conflict());
    }
    let journal = state.journal(request.intent.operation.id)?;
    journal.validate_for(&request.intent.operation)?;
    if journal.helper_container_id != expected.helper_container_id
        || journal.operation.phase != UpdatePhase::Draining
        || journal.operation.revision != 1
        || journal.operation.checkpoint.is_some()
        || journal.verified_target.is_some()
        || journal.maintenance_released
        || journal.completion_acknowledged
    {
        return Err(conflict());
    }
    request.intent.verify_saved(&state)?;
    state.require_lock_held()?;
    if state.read::<HelperAcknowledgement>(&request.intent.file("ack"))? != ack {
        return Err(conflict());
    }
    if cancellation.is_cancelled() {
        return Err(UpdateError::Cancelled);
    }
    if matches!(request.action, HandoffAction::Grant) {
        state.create(&request.intent.file("shutdown-grant"), &ack)?;
    }
    serde_json::to_vec(&ack).map_err(|_| conflict())
}
