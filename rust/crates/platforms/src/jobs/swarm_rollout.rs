//! Shared observation rules from SwarmReconciliationJob. Transport acknowledgement
//! and task counts alone do not establish that a rollout has completed.
use crate::{RuntimeSwarmService, RuntimeSwarmTask};

pub fn rollout_complete_state(state: &str) -> bool {
    state.trim().is_empty()
        || state.eq_ignore_ascii_case("none")
        || state.eq_ignore_ascii_case("completed")
}

pub fn rollout_paused(state: &str) -> bool {
    matches!(
        state.to_ascii_lowercase().replace('_', "").as_str(),
        "paused" | "rollbackpaused" | "rollbackcompleted"
    )
}

pub fn stack_observed_status(
    services: &[&RuntimeSwarmService],
    tasks: &[RuntimeSwarmTask],
) -> &'static str {
    if services.iter().any(|service| {
        rollout_paused(&service.update_state)
            || (rollout_complete_state(&service.update_state)
                && service.running_task_count < service.desired_task_count
                && tasks.iter().any(|task| {
                    task.service_id == service.id
                        // A pending task's scheduling diagnostic is recoverable
                        // when node capacity returns; it is not an execution failure.
                        && !task.state.eq_ignore_ascii_case("pending")
                        && !task.desired_state.eq_ignore_ascii_case("shutdown")
                        && !task.desired_state.eq_ignore_ascii_case("remove")
                        && task
                            .error
                            .as_ref()
                            .is_some_and(|error| !error.trim().is_empty())
                }))
    }) {
        "Failed"
    } else if services
        .iter()
        .all(|service| service.desired_task_count == 0)
    {
        "Stopped"
    } else if services.iter().all(|service| {
        rollout_complete_state(&service.update_state)
            && service.running_task_count >= service.desired_task_count
    }) {
        "Healthy"
    } else if services
        .iter()
        .any(|service| service.running_task_count < service.desired_task_count)
    {
        "Degraded"
    } else {
        "Pending"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn updating_rollout_is_not_complete_despite_matching_task_counts() {
        let mut service = RuntimeSwarmService {
            desired_task_count: 1,
            running_task_count: 1,
            update_state: "updating".into(),
            ..Default::default()
        };
        assert_eq!(stack_observed_status(&[&service], &[]), "Pending");
        service.update_state = "Completed".into();
        assert_eq!(stack_observed_status(&[&service], &[]), "Healthy");
        service.running_task_count = 0;
        assert_eq!(stack_observed_status(&[&service], &[]), "Degraded");
        service.desired_task_count = 0;
        assert_eq!(stack_observed_status(&[&service], &[]), "Stopped");
    }

    #[test]
    fn pending_capacity_shortage_degrades_and_can_recover() {
        let mut service = RuntimeSwarmService {
            id: "web".into(),
            desired_task_count: 2,
            running_task_count: 1,
            ..Default::default()
        };
        let task = RuntimeSwarmTask {
            service_id: "web".into(),
            desired_state: "running".into(),
            state: "pending".into(),
            error: Some("no suitable node".into()),
            ..Default::default()
        };
        assert_eq!(stack_observed_status(&[&service], &[task]), "Degraded");
        service.running_task_count = 2;
        assert_eq!(stack_observed_status(&[&service], &[]), "Healthy");
    }

    #[test]
    fn paused_rollouts_and_current_task_failures_fail_the_stack() {
        let mut service = RuntimeSwarmService {
            id: "service".into(),
            desired_task_count: 1,
            update_state: "completed".into(),
            ..Default::default()
        };
        let mut task = RuntimeSwarmTask {
            service_id: service.id.clone(),
            error: Some("image unavailable".into()),
            desired_state: "running".into(),
            ..Default::default()
        };
        assert_eq!(
            stack_observed_status(&[&service], &[task.clone()]),
            "Failed"
        );
        task.desired_state = "shutdown".into();
        assert_eq!(stack_observed_status(&[&service], &[task]), "Degraded");
        for state in [
            "Paused",
            "RollbackPaused",
            "RollbackCompleted",
            "rollback_paused",
            "rollback_completed",
        ] {
            service.update_state = state.into();
            assert_eq!(stack_observed_status(&[&service], &[]), "Failed");
        }
    }
}
