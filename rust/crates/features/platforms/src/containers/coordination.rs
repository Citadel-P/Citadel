//! Live hints only. PostgreSQL claims and read-only recovery remain authoritative.
use super::*;
use std::{collections::BTreeMap, sync::Mutex};
use tokio::sync::watch;

pub const EVENT_CONFIRMATION_WINDOW: Duration = Duration::from_millis(150);
const MAX_LIVE_OPERATIONS: usize = 32;

#[derive(Default)]
pub struct ContainerOperationCoordinator {
    operations: Mutex<BTreeMap<Uuid, Operation>>,
}
struct Operation {
    started_at_millis: i64,
    action: ContainerAction,
    targets: BTreeMap<Uuid, (ContainerTarget, bool)>,
    changed: watch::Sender<u64>,
}

/// Dropping the task, timing out, or completing removes its bounded live hints.
pub struct OperationRegistration {
    coordinator: Arc<ContainerOperationCoordinator>,
    id: Uuid,
    changed: watch::Receiver<u64>,
}
impl ContainerOperationCoordinator {
    pub fn register(
        self: &Arc<Self>,
        claim: &ContainerClaim,
        action: ContainerAction,
    ) -> Option<OperationRegistration> {
        let mut operations = self.operations.lock().unwrap_or_else(|e| e.into_inner());
        if operations.len() >= MAX_LIVE_OPERATIONS
            || claim.targets.len() > MAX_CONTAINER_BATCH
            || operations.contains_key(&claim.operation_id)
        {
            return None;
        }
        let (changed, receiver) = watch::channel(0);
        operations.insert(
            claim.operation_id,
            Operation {
                // Registration precedes the runtime mutation. Millisecond event
                // stamps reject delayed pre-command events in the same Unix second.
                started_at_millis: chrono::Utc::now().timestamp_millis(),
                action,
                targets: claim
                    .targets
                    .iter()
                    .map(|target| (target.id, (target.clone(), false)))
                    .collect(),
                changed,
            },
        );
        Some(OperationRegistration {
            coordinator: self.clone(),
            id: claim.operation_id,
            changed: receiver,
        })
    }

    /// Call only after committing a fresh observation with this durable operation ID.
    /// Unexpected later states revoke a previous confirmation.
    pub fn committed(
        &self,
        operation: Uuid,
        target: &ContainerTarget,
        state: Option<&str>,
        observed_at_millis: i64,
    ) {
        let mut operations = self.operations.lock().unwrap_or_else(|e| e.into_inner());
        let Some(entry) = operations.get_mut(&operation) else {
            return;
        };
        if observed_at_millis < entry.started_at_millis {
            return;
        }
        let Some((expected, confirmed)) = entry.targets.get_mut(&target.id) else {
            return;
        };
        if expected.platform_id != target.platform_id
            || expected.node_id != target.node_id
            || expected.docker_id != target.docker_id
        {
            return;
        }
        let matches = match entry.action {
            ContainerAction::Start | ContainerAction::Unpause | ContainerAction::Restart => {
                state.is_some_and(|s| s.eq_ignore_ascii_case("running"))
            }
            ContainerAction::Stop => state.is_some_and(|s| s.eq_ignore_ascii_case("exited")),
            ContainerAction::Pause => state.is_some_and(|s| s.eq_ignore_ascii_case("paused")),
            ContainerAction::Delete(_) => state.is_none(),
        };
        if *confirmed != matches {
            *confirmed = matches;
            entry
                .changed
                .send_modify(|version| *version = version.wrapping_add(1));
        }
    }
}
impl OperationRegistration {
    fn missing(&self) -> Vec<ContainerTarget> {
        self.coordinator
            .operations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&self.id)
            .expect("registration owns live entry")
            .targets
            .values()
            .filter(|(_, confirmed)| !confirmed)
            .map(|(target, _)| target.clone())
            .collect()
    }
    pub async fn wait_missing(
        &mut self,
        cancellation: &CancellationToken,
    ) -> Result<Vec<ContainerTarget>, RuntimeCapabilityError> {
        let deadline = tokio::time::sleep(EVENT_CONFIRMATION_WINDOW);
        tokio::pin!(deadline);
        loop {
            if cancellation.is_cancelled() {
                return Err(error(
                    RuntimeErrorKind::Unavailable,
                    "Container operation canceled; durable recovery retains its claim.",
                ));
            }
            let missing = self.missing();
            if missing.is_empty() {
                return Ok(missing);
            }
            tokio::select! {
                biased;
                () = cancellation.cancelled() => return Err(error(RuntimeErrorKind::Unavailable, "Container operation canceled; durable recovery retains its claim.")),
                () = &mut deadline => return Ok(self.missing()),
                _ = self.changed.changed() => {}
            }
        }
    }
}
impl Drop for OperationRegistration {
    fn drop(&mut self) {
        self.coordinator
            .operations
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&self.id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim() -> ContainerClaim {
        let target = ContainerTarget {
            id: Uuid::now_v7(),
            platform_id: Uuid::now_v7(),
            docker_id: "fixture".into(),
            node_id: Some("worker-1".into()),
        };
        ContainerClaim {
            operation_id: Uuid::now_v7(),
            started_at: chrono::Utc::now().timestamp(),
            targets: vec![target],
            deployment_ids: vec![],
            stack_ids: vec![],
        }
    }

    #[test]
    fn old_or_wrong_identity_observations_cannot_confirm_a_command() {
        let coordinator = Arc::new(ContainerOperationCoordinator::default());
        let claim = claim();
        let registration = coordinator.register(&claim, ContainerAction::Stop).unwrap();
        let target = &claim.targets[0];
        let before_registration = coordinator
            .operations
            .lock()
            .unwrap()
            .get(&claim.operation_id)
            .unwrap()
            .started_at_millis
            - 1;
        coordinator.committed(
            claim.operation_id,
            target,
            Some("exited"),
            before_registration,
        );
        assert_eq!(registration.missing().len(), 1);
        let mut wrong = target.clone();
        wrong.node_id = Some("other-worker".into());
        coordinator.committed(
            claim.operation_id,
            &wrong,
            Some("exited"),
            chrono::Utc::now().timestamp_millis() + 10,
        );
        assert_eq!(registration.missing().len(), 1);
        coordinator.committed(
            claim.operation_id,
            target,
            Some("running"),
            chrono::Utc::now().timestamp_millis() + 20,
        );
        assert_eq!(registration.missing().len(), 1);
        coordinator.committed(
            claim.operation_id,
            target,
            Some("exited"),
            chrono::Utc::now().timestamp_millis() + 30,
        );
        assert!(registration.missing().is_empty());
    }

    #[test]
    fn unexpected_later_state_revokes_a_previous_confirmation() {
        let coordinator = Arc::new(ContainerOperationCoordinator::default());
        let claim = claim();
        let registration = coordinator.register(&claim, ContainerAction::Stop).unwrap();
        let target = &claim.targets[0];
        coordinator.committed(
            claim.operation_id,
            target,
            Some("exited"),
            chrono::Utc::now().timestamp_millis() + 10,
        );
        assert!(registration.missing().is_empty());
        coordinator.committed(
            claim.operation_id,
            target,
            Some("running"),
            chrono::Utc::now().timestamp_millis() + 20,
        );
        assert_eq!(registration.missing().len(), 1);
    }
}
