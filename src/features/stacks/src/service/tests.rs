use super::apply::build_image_override;
use super::*;

use chrono::Utc;

use crate::RecreateStackOnNewImageState;
use crate::StackDriftPolicy;
use crate::StackSource;
use crate::StackUpdateState;

#[test]
fn runtime_output_is_redacted_before_progress_and_failure_persistence() {
    for status in [StackReleaseStatus::Healthy, StackReleaseStatus::Failed] {
        let mut result = StackRuntimeResult {
            status,
            messages: vec![
                StackProgressItem::system("pre-deploy: secret-value"),
                StackProgressItem {
                    event_type: crate::StackApplyEventType::StdErr,
                    message: Some("error: secret-value".into()),
                    exit_code: Some(1),
                    stack_status: None,
                    severity: Some("secret-value".into()),
                },
            ],
        };
        redact_runtime_messages(&mut result, &["secret-value".into()]);
        assert_eq!(result.status, status);
        assert_eq!(result.messages[1].exit_code, Some(1));
        assert!(result.messages.iter().all(|item| {
            !item
                .message
                .as_deref()
                .unwrap_or_default()
                .contains("secret-value")
                && !item
                    .severity
                    .as_deref()
                    .unwrap_or_default()
                    .contains("secret-value")
        }));
        assert!(
            result.messages[0]
                .message
                .as_ref()
                .unwrap()
                .contains("********")
        );
    }
}

fn stack(status: StackReleaseStatus, policy: StackDriftPolicy) -> crate::Stack {
    let id = Uuid::now_v7();
    crate::Stack {
        id,
        name: "demo".to_owned(),
        description: None,
        stack_source: StackSource::WebEditor,
        stack_update_state: StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
        },
        drift_policy: policy,

        control_state: citadel_primitives::ResourceControlState::Idle,
        current_stack_release_id: Uuid::now_v7(),
        row_version: 0,

        audit: citadel_primitives::AuditMetadata {
            created_at: Utc::now(),
            created_by_actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
        },

        status,
        platform_type: citadel_platforms::PlatformKind::Docker,
        platform_id: Some(Uuid::now_v7()),
        version: Some("1".to_owned()),
        spec: Some(StackSpec::WebEditor {
            compose_file: "services:\n  api:\n    image: nginx\n".to_owned(),
            update_behavior: crate::StackUpdateBehavior::Disabled,
            common: Default::default(),
        }),
        source: None,
        resource_bindings: None,
        platform_status: citadel_primitives::PlatformStatus::Online,
        platform_name: Some("local".to_owned()),
        tags: Vec::new(),
        latest_activity: None,
    }
}

#[test]
fn drift_reports_missing_extra_stopped_and_paused_containers() {
    let policy = StackDriftPolicy {
        auto_start_stopped_containers: true,
        auto_resume_paused_containers: true,
        remove_extra_containers: true,
        ..Default::default()
    };
    let stack = stack(StackReleaseStatus::Healthy, policy);
    let report = calculate_drift(
        &stack,
        &StackRuntimeSnapshot {
            containers: vec![crate::StackRuntimeContainer {
                docker_container_id: "old".to_owned(),
                service_name: "old".to_owned(),
                state: "running".to_owned(),
                health: None,
            }],
            services: Vec::new(),
        },
    )
    .unwrap();
    assert!(report.has_structural_drift);
    assert!(report.has_auto_fixable_drift);
    assert_eq!(report.drifts.len(), 2);
}

#[test]
fn one_stopped_service_is_not_reported_as_two_missing_services() {
    let stack = stack(StackReleaseStatus::Healthy, StackDriftPolicy::default());
    let runtime = StackRuntimeSnapshot {
        containers: [("beszel", "running"), ("beszel-agent", "exited")]
            .into_iter()
            .map(|(name, state)| crate::StackRuntimeContainer {
                docker_container_id: format!("{name}-id"),
                service_name: name.into(),
                state: state.into(),
                health: None,
            })
            .collect(),
        services: Vec::new(),
    };
    let report = calculate_drift_from_compose(
        &stack,
        &runtime,
        &[
            "services:\n  beszel:\n    image: beszel\n  beszel-agent:\n    image: beszel-agent\n"
                .into(),
        ],
    )
    .unwrap();
    assert!(!report.has_structural_drift);
    assert!(
        matches!(report.drifts.as_slice(), [StackDrift::ContainerStopped { service_name, .. }] if service_name == "beszel-agent")
    );
}

#[test]
fn drift_recognizes_projected_container_states_and_serializes_frontend_fields() {
    for (state, kind) in [
        ("Exited", "ContainerStopped"),
        ("Created", "ContainerStopped"),
        ("Paused", "ContainerPaused"),
    ] {
        let report = calculate_drift(
            &stack(StackReleaseStatus::Healthy, StackDriftPolicy::default()),
            &StackRuntimeSnapshot {
                containers: vec![crate::StackRuntimeContainer {
                    docker_container_id: "docker-api".into(),
                    service_name: "api".into(),
                    state: state.into(),
                    health: None,
                }],
                services: vec![],
            },
        )
        .unwrap();
        assert!(report.has_drift, "{state} must not be healthy");
        let drift = serde_json::to_value(&report.drifts[0]).unwrap();
        assert_eq!(drift["$type"], kind);
        assert_eq!(drift["serviceName"], "api");
        assert_eq!(drift["containerId"], "docker-api");
        assert!(!report.has_structural_drift);
    }
    let drift = serde_json::to_value(StackDrift::MissingContainer {
        service_name: "api".into(),
    })
    .unwrap();
    assert_eq!(drift["serviceName"], "api");
}

// Port: StackDriftMonitorJobTests die on AutoFix vs intentional stop.
#[test]
fn daemon_drift_detection_targets_idle_active_stacks_with_an_enabled_policy() {
    let mut policy = StackDriftPolicy {
        mode: crate::StackDriftMode::AutoFix,
        ..Default::default()
    };
    for status in [StackReleaseStatus::Healthy, StackReleaseStatus::Degraded] {
        let mut value = stack(status, policy.clone());
        assert!(event_drift_eligible(&value));
        value.control_state = citadel_primitives::ResourceControlState::Processing;
        assert!(!event_drift_eligible(&value));
    }
    for status in [
        StackReleaseStatus::Stopped,
        StackReleaseStatus::Paused,
        StackReleaseStatus::Applying,
        StackReleaseStatus::Created,
    ] {
        assert!(!event_drift_eligible(&stack(status, policy.clone())));
    }
    policy.mode = crate::StackDriftMode::DetectOnly;
    assert!(event_drift_eligible(&stack(
        StackReleaseStatus::Healthy,
        policy.clone()
    )));
    policy.mode = crate::StackDriftMode::Disabled;
    assert!(!event_drift_eligible(&stack(
        StackReleaseStatus::Healthy,
        policy
    )));
}

#[test]
fn drift_status_respects_policy_deduplication_and_recovery_origin() {
    let mut stack = stack(StackReleaseStatus::Healthy, StackDriftPolicy::default());
    let mut report = calculate_drift(
        &stack,
        &StackRuntimeSnapshot {
            containers: vec![],
            services: vec![],
        },
    )
    .unwrap();
    let (status, info) = drift_status_update(&stack, &report).unwrap();
    assert_eq!(status, StackReleaseStatus::Degraded);
    stack.status = status;
    stack.latest_activity = Some(citadel_activities::ActivitySummary {
        id: uuid::Uuid::now_v7(),
        resource_type: citadel_activities::ActivityResourceType::Stack,
        event_type: citadel_activities::ActivityEventType::StackDriftDetected,
        status: citadel_activities::ActivityStatus::Warning,
        created_at: chrono::Utc::now(),
        info: serde_json::to_value(info).unwrap(),
    });
    assert!(drift_status_update(&stack, &report).is_none());
    report.has_drift = false;
    report.drifts.clear();
    assert!(matches!(
        drift_status_update(&stack, &report),
        Some((
            StackReleaseStatus::Healthy,
            ActivityEventInfo::StackDriftResolved { .. }
        ))
    ));
    stack.latest_activity = None;
    assert!(
        drift_status_update(&stack, &report).is_none(),
        "unrelated degradation must not be healed"
    );
    report.has_drift = true;
    stack.status = StackReleaseStatus::Healthy;
    stack.drift_policy.mark_degraded = false;
    assert_eq!(
        drift_status_update(&stack, &report).unwrap().0,
        StackReleaseStatus::Healthy
    );
    for status in [
        StackReleaseStatus::Stopped,
        StackReleaseStatus::Paused,
        StackReleaseStatus::Applying,
    ] {
        stack.status = status;
        assert!(drift_status_update(&stack, &report).is_none());
    }
    stack.status = StackReleaseStatus::Healthy;
    stack.control_state = citadel_primitives::ResourceControlState::Processing;
    assert!(drift_status_update(&stack, &report).is_none());
}

#[test]
fn intentionally_stopped_stack_has_no_drift() {
    let stack = stack(StackReleaseStatus::Stopped, StackDriftPolicy::default());
    let report = calculate_drift(
        &stack,
        &StackRuntimeSnapshot {
            containers: Vec::new(),
            services: Vec::new(),
        },
    )
    .unwrap();
    assert!(!report.has_drift);
}

#[test]
fn stale_state_recovery_only_accepts_stable_container_aggregates() {
    let snapshot = |states: &[(&str, Option<&str>)]| StackRuntimeSnapshot {
        containers: states
            .iter()
            .enumerate()
            .map(|(index, (state, health))| crate::StackRuntimeContainer {
                docker_container_id: format!("container-{index}"),
                service_name: format!("service-{index}"),
                state: (*state).to_owned(),
                health: health.map(str::to_owned),
            })
            .collect(),
        services: Vec::new(),
    };

    assert_eq!(
        stable_runtime_status(&snapshot(&[("running", Some("healthy"))])),
        Some(StackReleaseStatus::Healthy)
    );
    assert_eq!(
        stable_runtime_status(&snapshot(&[("paused", None), ("paused", None)])),
        Some(StackReleaseStatus::Paused)
    );
    assert_eq!(
        stable_runtime_status(&snapshot(&[("exited", None), ("offline", None)])),
        Some(StackReleaseStatus::Stopped)
    );
    assert_eq!(
        stable_runtime_status(&snapshot(&[("running", None), ("exited", None)])),
        None
    );
    assert_eq!(
        stable_runtime_status(&snapshot(&[("running", Some("unhealthy"))])),
        None
    );
    assert_eq!(
        stable_runtime_status(&StackRuntimeSnapshot {
            containers: Vec::new(),
            services: Vec::new(),
        }),
        None
    );
}

#[test]
fn import_fingerprint_changes_with_authoritative_runtime_state() {
    let platform_id = Uuid::now_v7();
    let base = StackImportClaim {
        orphaned_owner_id: None,
        platform_id,
        platform_name: "swarm".to_owned(),
        project_name: "demo".to_owned(),
        import_kind: crate::StackImportKind::SwarmStack,
        runtime_fingerprint: "sha256:first".to_owned(),
        service_names: vec!["api".to_owned()],
        container_ids: Vec::new(),
        container_names: Vec::new(),
        services: vec![crate::ComposeProjectRuntimeService {
            name: "api".to_owned(),
            image: Some("nginx:1".to_owned()),
            container_count: 1,
            states: vec!["completed".to_owned(), "running:1".to_owned()],
        }],
    };
    let mut changed = base.clone();
    changed.runtime_fingerprint = "sha256:second".to_owned();
    assert_ne!(
        import_runtime_fingerprint(&base),
        import_runtime_fingerprint(&changed)
    );
}

#[test]
fn build_image_override_quotes_service_names_and_uses_resolved_images() {
    let output = build_image_override(&[ResolvedStackBuildImageBinding {
        service_name: "api-worker".to_owned(),
        build_project_id: Uuid::now_v7(),
        image_reference: "registry.test:5000/team/api@sha256:abc".to_owned(),
        digest: Some("sha256:abc".to_owned()),
        build_run_id: Some(Uuid::now_v7()),
    }]);
    assert_eq!(
        output,
        "services:\n  \"api-worker\":\n    image: \"registry.test:5000/team/api@sha256:abc\"\n"
    );
}

#[test]
fn git_drift_is_pinned_to_the_deployed_commit_and_requires_a_baseline() {
    let mut stack = stack(StackReleaseStatus::Healthy, StackDriftPolicy::default());
    stack.stack_source = StackSource::Git;
    stack.spec = Some(serde_json::from_value(serde_json::json!({"$type":"Git", "gitRepoId":Uuid::now_v7(), "branch":"main", "commitSha":"unapplied", "composePaths":["compose.yml","override.yml"]})).unwrap());
    assert!(matches!(
        drift_source_claim(&stack, stack.audit.created_by_actor_id),
        Err(StackError::Conflict(_))
    ));
    stack.source = Some(
        serde_json::from_value(
            serde_json::json!({"sourceType":"Git", "resolvedCommitSha":"abc123"}),
        )
        .unwrap(),
    );
    let claim = drift_source_claim(&stack, stack.audit.created_by_actor_id).unwrap();
    let StackSpec::Git {
        commit_sha,
        compose_paths,
        ..
    } = claim.spec
    else {
        panic!("Git spec expected")
    };
    assert_eq!(commit_sha.as_deref(), Some("abc123"));
    assert_eq!(compose_paths, ["compose.yml", "override.yml"]);
    assert_eq!(claim.release_id, stack.current_stack_release_id);
    assert_eq!(claim.operation, "Drift");
}

#[test]
fn git_drift_compares_all_compose_files_and_still_detects_real_drift() {
    let stack = stack(StackReleaseStatus::Healthy, StackDriftPolicy::default());
    let files = vec![
        "services:\n  api:\n    image: nginx\n".into(),
        "services:\n  worker:\n    image: alpine\n".into(),
    ];
    let mut runtime = StackRuntimeSnapshot {
        containers: vec![],
        services: vec![],
    };
    for name in ["api", "worker"] {
        runtime.containers.push(crate::StackRuntimeContainer {
            docker_container_id: name.into(),
            service_name: name.into(),
            state: "Running".into(),
            health: None,
        });
    }
    assert!(
        !calculate_drift_from_compose(&stack, &runtime, &files)
            .unwrap()
            .has_drift
    );
    runtime.containers.pop();
    let report = calculate_drift_from_compose(&stack, &runtime, &files).unwrap();
    assert!(report.has_structural_drift);
    assert!(
        matches!(&report.drifts[0], StackDrift::MissingContainer { service_name } if service_name == "worker")
    );
}

#[test]
fn failed_apply_summary_keeps_redacted_error_before_empty_completion() {
    let mut error = StackProgressItem::system(
        "Error response from daemon: port 8080 is already allocated; secret=private-value",
    );
    error.event_type = StackApplyEventType::StdErr;
    let mut completed =
        StackProgressItem::completed(StackReleaseStatus::Failed, "Stack deployment failed.");
    completed.message = None;
    let mut result = StackRuntimeResult {
        status: StackReleaseStatus::Failed,
        messages: vec![error, completed],
    };
    redact_runtime_messages(&mut result, &["private-value".into()]);
    let message = super::apply::apply_failure_message(&result);
    assert!(message.contains("exit code 1"));
    assert!(message.contains("port 8080 is already allocated"));
    assert!(message.contains("********"));
    assert!(!message.contains("private-value"));
    result.messages[0].event_type = StackApplyEventType::StdOut;
    assert!(super::apply::apply_failure_message(&result).contains("port 8080"));
    result.messages.remove(0);
    assert_eq!(
        super::apply::apply_failure_message(&result),
        "Stack deployment failed (exit code 1)."
    );
}

#[test]
fn compose_failure_summary_does_not_replay_progress_or_duplicate_errors() {
    let error = "Error response from daemon: Conflict. The container name is already in use.";
    let mut messages: Vec<_> = [
        "beszel Pulled",
        "beszel-agent Pulled",
        "Network beszel-copy_default Created",
        "Container beszel Creating",
        &format!("Container beszel-agent {error}"),
        error,
    ]
    .into_iter()
    .map(|text| {
        let mut item = StackProgressItem::system(text);
        item.event_type = StackApplyEventType::StdErr;
        item
    })
    .collect();
    for item in &messages[..4] {
        assert!(item.error_detail().is_none());
    }
    assert_eq!(messages[4].error_detail(), Some(error));
    assert_eq!(messages[5].error_detail(), Some(error));
    messages.push(StackProgressItem::completed(
        StackReleaseStatus::Failed,
        "Stack deployment failed.",
    ));
    let result = StackRuntimeResult {
        status: StackReleaseStatus::Failed,
        messages,
    };
    assert_eq!(
        super::apply::apply_failure_message(&result),
        format!("Stack deployment failed (exit code 1).\n{error}")
    );
}
