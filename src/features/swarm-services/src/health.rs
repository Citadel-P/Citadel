use citadel_primitives::health::*;

pub fn requirements(
    service_id: &str,
    mode: &str,
    desired: usize,
) -> Option<Vec<HealthRequirement>> {
    // A global service with no eligible nodes does not express an intentional stop.
    if mode.eq_ignore_ascii_case("global") && desired == 0 {
        return None;
    }
    if !mode.eq_ignore_ascii_case("replicated") && !mode.eq_ignore_ascii_case("global") {
        return None;
    }
    Some(vec![HealthRequirement {
        service: service_id.into(),
        replicas: desired,
        completed_job: false,
    }])
}

pub fn assess(
    members: Vec<HealthMember>,
    service: Option<AppliedServiceHealth>,
    online: bool,
    never_applied: bool,
    operation: Option<crate::SwarmServiceOperationState>,
    now: i64,
) -> WorkloadHealth {
    use crate::SwarmServiceOperationState as Operation;
    let requirements = service
        .as_ref()
        .filter(|s| !s.stale && s.ownership != "OwnershipConflict")
        .and_then(|s| requirements(&s.service, &s.mode, s.desired));
    let inactive = never_applied
        || service.as_ref().is_some_and(|s| {
            !s.stale
                && s.mode.eq_ignore_ascii_case("replicated")
                && s.desired == 0
                && s.running == 0
        });
    let update = service
        .as_ref()
        .map(|s| s.update_state.as_str())
        .unwrap_or("");
    assess_health(
        HealthAssessmentInput {
            requirements: if never_applied {
                Some(vec![])
            } else {
                requirements
            },
            members,
            available: online
                && service
                    .as_ref()
                    .is_none_or(|s| !s.stale && s.ownership != "OwnershipConflict"),
            inactive,
            progressing: matches!(update, "updating" | "rollback_started")
                || matches!(
                    operation,
                    Some(Operation::Prepared | Operation::PendingAcceptance | Operation::Accepted)
                ),
            operation_failed: matches!(update, "paused" | "rollback_paused")
                || matches!(
                    operation,
                    Some(Operation::Rejected | Operation::NotAccepted)
                ),
            ownership_conflict: service
                .as_ref()
                .is_some_and(|s| s.ownership == "OwnershipConflict")
                || operation == Some(Operation::OwnershipConflict),
        },
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_applied_replicated_zero_is_intentionally_inactive() {
        for (mode, expected) in [
            ("replicated", WorkloadHealthStatus::NotRunning),
            ("global", WorkloadHealthStatus::Unknown),
        ] {
            let service = AppliedServiceHealth {
                service: "s".into(),
                mode: mode.into(),
                desired: 0,
                running: 0,
                stale: false,
                update_state: String::new(),
                ownership: String::new(),
                stack_service_count: None,
                release_id: None,
            };
            assert_eq!(
                assess(vec![], Some(service.clone()), true, false, None, 1_000).status,
                expected
            );
            let mut draining = service;
            draining.running = 1;
            assert_ne!(
                assess(vec![], Some(draining), true, false, None, 1_000).status,
                WorkloadHealthStatus::NotRunning
            );
        }
        assert!(requirements("s", "unsupported", 1).is_none());
        assert_eq!(
            assess(vec![], None, true, true, None, 1_000).status,
            WorkloadHealthStatus::NotRunning
        );
        assert_eq!(
            assess(vec![], None, true, false, None, 1_000).status,
            WorkloadHealthStatus::Unknown
        );
    }
}
