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

fn stack(status: StackReleaseStatus, policy: StackDriftPolicy) -> StackDetails {
    let id = Uuid::now_v7();
    StackDetails {
        stack: crate::Stack {
            id,
            name: "demo".to_owned(),
            description: None,
            stack_source: StackSource::WebEditor,
            stack_update_state: StackUpdateState::WebEditor {
                recreate_stack_on_new_image_state: RecreateStackOnNewImageState::default(),
            },
            drift_policy: policy,
            created_at: Utc::now(),
            created_by_actor_id: Uuid::now_v7(),
            control_state: "Idle".to_owned(),
            current_stack_release_id: Uuid::now_v7(),
            row_version: 0,
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
        platform_status: "Online".to_owned(),
        platform_name: Some("local".to_owned()),
        tags: Vec::new(),
        latest_activity: None,
        effective_permission: citadel_primitives::EffectivePermission::Administrator,
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
fn daemon_drift_repair_only_targets_idle_healthy_or_degraded_auto_fix_stacks() {
    let mut policy = StackDriftPolicy {
        mode: crate::StackDriftMode::AutoFix,
        ..Default::default()
    };
    for status in [StackReleaseStatus::Healthy, StackReleaseStatus::Degraded] {
        let mut value = stack(status, policy.clone());
        assert!(event_drift_eligible(&value));
        value.stack.control_state = "Processing".into();
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
    stack.latest_activity = Some(serde_json::json!({"info": info}));
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
    stack.stack.drift_policy.mark_degraded = false;
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
    stack.stack.control_state = "Processing".into();
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
