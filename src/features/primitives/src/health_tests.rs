use super::*;
const NOW: i64 = 1_000_000;
fn member(service: &str, status: HealthStatus) -> HealthMember {
    HealthMember {
        stop_requested_at: None,
        status: HealthStatus::Unknown,
        container_id: Some(Uuid::now_v7()),
        task_id: None,
        docker_container_id: Some(service.into()),
        platform_id: Uuid::nil(),
        node_id: None,
        name: service.into(),
        service: service.into(),
        state: "Running".into(),
        available: true,
        completed_job: false,
        observation: Some(ContainerHealthObservation {
            configuration: match status {
                HealthStatus::NoCheck => CheckConfiguration::NoCheck,
                HealthStatus::Disabled => CheckConfiguration::Disabled,
                _ => CheckConfiguration::Enabled,
            },
            result: Some(status),
            observed_at: NOW,
            last_check_at: None,
            failing_streak: None,
            exit_code: None,
            oom_killed: None,
            restart_count: None,
            started_at: None,
            finished_at: None,
        }),
    }
}
fn input(members: Vec<HealthMember>) -> HealthAssessmentInput {
    HealthAssessmentInput {
        requirements: Some(vec![HealthRequirement {
            service: "api".into(),
            replicas: 2,
            completed_job: false,
        }]),
        members,
        available: true,
        inactive: false,
        progressing: false,
        operation_failed: false,
        ownership_conflict: false,
    }
}
#[test]
fn mixed_check_and_observation_coverage_is_never_all_green() {
    for (second, expected) in [
        (HealthStatus::Healthy, WorkloadHealthStatus::Healthy),
        (HealthStatus::Unhealthy, WorkloadHealthStatus::Unhealthy),
        (HealthStatus::Starting, WorkloadHealthStatus::Starting),
        (HealthStatus::NoCheck, WorkloadHealthStatus::Partial),
        (HealthStatus::Disabled, WorkloadHealthStatus::Partial),
        (HealthStatus::Unknown, WorkloadHealthStatus::Partial),
    ] {
        let result = assess_health(
            input(vec![
                member("api", HealthStatus::Healthy),
                member("api", second),
            ]),
            NOW,
        );
        assert_eq!(result.status, expected);
        assert_eq!(result.expected, Some(2));
    }
}
#[test]
fn missing_service_cannot_be_hidden_by_an_extra_healthy_replica() {
    let mut value = input(vec![
        member("api", HealthStatus::Healthy),
        member("api", HealthStatus::Healthy),
    ]);
    value.requirements.as_mut().unwrap()[0].replicas = 1;
    value
        .requirements
        .as_mut()
        .unwrap()
        .push(HealthRequirement {
            service: "db".into(),
            replicas: 1,
            completed_job: false,
        });
    let result = assess_health(value, NOW);
    assert_eq!(result.missing, Some(1));
    assert_eq!(result.status, WorkloadHealthStatus::Partial);
    assert!(result.attention.contains(&HealthAttention::RuntimeMissing));
}
#[test]
fn unknown_topology_does_not_invent_missing_counts() {
    let mut value = input(vec![member("api", HealthStatus::Healthy)]);
    value.requirements = None;
    let result = assess_health(value, NOW);
    assert_eq!(result.status, WorkloadHealthStatus::Partial);
    assert_eq!(result.expected, None);
    assert_eq!(result.missing, None);
}
#[test]
fn stale_or_future_evidence_cannot_hide_a_current_failure() {
    for observed in [NOW - HEALTH_MAX_AGE_MS - 1, NOW + 1] {
        let mut stale = member("api", HealthStatus::Healthy);
        stale.observation.as_mut().unwrap().observed_at = observed;
        let result = assess_health(
            input(vec![stale, member("api", HealthStatus::Unhealthy)]),
            NOW,
        );
        assert_eq!(result.status, WorkloadHealthStatus::Unhealthy);
        assert_eq!(result.observed_at, Some(NOW));
        assert_eq!(result.counts.unknown, 1);
        assert!(!result.coverage_complete);
    }
}
#[test]
fn unreachable_runtime_and_missing_observations_are_unknown() {
    let mut value = input(vec![member("api", HealthStatus::Healthy)]);
    value.available = false;
    let result = assess_health(value, NOW);
    assert_eq!(result.status, WorkloadHealthStatus::Unknown);
    assert!(
        result
            .attention
            .contains(&HealthAttention::RuntimeUnavailable)
    );
    let mut value = input(vec![member("api", HealthStatus::Healthy)]);
    value.members[0].observation = None;
    assert_eq!(
        assess_health(value, NOW).status,
        WorkloadHealthStatus::Unknown
    );
}
#[test]
fn stop_and_rollout_suppress_deficits_but_not_current_failures() {
    for (inactive, progressing) in [(true, false), (false, true)] {
        let mut value = input(vec![member("api", HealthStatus::Unhealthy)]);
        value.inactive = inactive;
        value.progressing = progressing;
        let result = assess_health(value, NOW);
        assert!(!result.attention.contains(&HealthAttention::RuntimeMissing));
        assert!(result.attention.contains(&HealthAttention::Unhealthy));
    }
    let mut value = input(vec![]);
    value.inactive = true;
    assert_eq!(
        assess_health(value, NOW).status,
        WorkloadHealthStatus::NotRunning
    );
    assert_ne!(
        assess_health(input(vec![]), NOW).status,
        WorkloadHealthStatus::Healthy
    );
}
#[test]
fn exit_zero_is_only_completion_with_proven_intent_and_fresh_evidence() {
    let mut exited = member("api", HealthStatus::Healthy);
    exited.state = "Exited".into();
    exited.observation.as_mut().unwrap().exit_code = Some(0);
    let mut value = input(vec![exited.clone(), member("api", HealthStatus::Healthy)]);
    assert_eq!(assess_health(value, NOW).counts.completed, 0);
    value = input(vec![exited.clone(), member("api", HealthStatus::Healthy)]);
    value.requirements.as_mut().unwrap()[0].completed_job = true;
    assert_eq!(assess_health(value, NOW).counts.completed, 1);
    exited.available = false;
    value = input(vec![exited]);
    value.requirements.as_mut().unwrap()[0].completed_job = true;
    assert_eq!(assess_health(value, NOW).counts.completed, 0);
}
#[test]
fn bounded_breakdown_preserves_full_counts_and_failures() {
    let mut members = vec![member("api", HealthStatus::Healthy); 300];
    members.push(member("api", HealthStatus::Unhealthy));
    let mut value = input(members);
    value.requirements.as_mut().unwrap()[0].replicas = 301;
    let result = assess_health(value, NOW);
    assert_eq!(result.observed, 301);
    assert_eq!(result.counts.healthy, 300);
    assert_eq!(result.members.len(), 200);
    assert!(result.members_truncated);
    assert_eq!(result.members[0].status, HealthStatus::Unhealthy);
}

#[test]
fn pre_upgrade_unknown_service_identity_does_not_invent_a_missing_instance() {
    let mut value = input(vec![member("", HealthStatus::Healthy)]);
    value.requirements.as_mut().unwrap()[0].replicas = 1;
    let result = assess_health(value, NOW);
    assert_eq!(result.expected, Some(1));
    assert_eq!(result.missing, None);
    assert_eq!(result.status, WorkloadHealthStatus::Partial);
    assert!(!result.attention.contains(&HealthAttention::RuntimeMissing));
}

#[test]
fn priority_expiry_covers_all_members_without_extending_coverage() {
    let mut old = member("api", HealthStatus::Healthy);
    old.observation.as_mut().unwrap().observed_at = NOW - 170_000;
    let mut members = vec![old; 250];
    members.push(member("api", HealthStatus::Unhealthy));
    let mut value = input(members);
    value.requirements.as_mut().unwrap()[0].replicas = 251;
    let result = assess_health(value, NOW);
    assert!(result.members_truncated);
    assert_eq!(result.valid_until, Some(NOW + 10_000));
    assert_eq!(result.unhealthy_until, Some(NOW + HEALTH_MAX_AGE_MS));
    assert_eq!(result.starting_until, None);
}

#[test]
fn completed_job_evidence_also_limits_complete_coverage() {
    let mut job = member("migration", HealthStatus::Healthy);
    job.state = "Exited".into();
    let observation = job.observation.as_mut().unwrap();
    observation.exit_code = Some(0);
    observation.observed_at = NOW - 170_000;
    let mut value = input(vec![member("api", HealthStatus::Healthy), job]);
    value.requirements = Some(vec![
        HealthRequirement {
            service: "api".into(),
            replicas: 1,
            completed_job: false,
        },
        HealthRequirement {
            service: "migration".into(),
            replicas: 1,
            completed_job: true,
        },
    ]);
    let result = assess_health(value, NOW);
    assert_eq!(result.status, WorkloadHealthStatus::Healthy);
    assert_eq!(result.valid_until, Some(NOW + 10_000));
}
