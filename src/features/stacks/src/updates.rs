use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::ImageUpdateState;
use crate::RecreateStackOnNewImageState;
use crate::StackError;
use crate::StackReleaseStatus;
use crate::StackSpec;
use crate::StackUpdateBehavior;
use crate::StackUpdateState;
use crate::parse_compose;
use crate::validation;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct StackImageKey {
    pub registry_id: Uuid,
    pub repository: String,
    pub tag: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualStackImageCheck {
    pub service_name: String,
    pub image_name: String,
    pub key: StackImageKey,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StackImageUpdateItem {
    pub service_name: String,
    pub image_name: String,
    pub current_digest: String,
    pub remote_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManualStackUpdateEvaluation {
    pub state: StackUpdateState,
    pub available_updates: Vec<StackImageUpdateItem>,
    pub newly_detected_updates: Vec<StackImageUpdateItem>,
    pub baselines_created: usize,
}

pub fn build_manual_stack_checks(
    stack: &crate::Stack,
    scheduled: bool,
) -> Result<Vec<ManualStackImageCheck>, StackError> {
    let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
    let StackSpec::WebEditor { compose_file, .. } = spec else {
        return Err(validation(
            "Git Stack image checks require the applied Compose files.",
        ));
    };
    build_stack_image_checks(stack, scheduled, std::slice::from_ref(compose_file))
}

/// Builds image checks from the applied Compose definition, regardless of source.
pub fn build_stack_image_checks(
    stack: &crate::Stack,
    scheduled: bool,
    compose_files: &[String],
) -> Result<Vec<ManualStackImageCheck>, StackError> {
    if stack.control_state == citadel_primitives::ResourceControlState::Processing {
        return Err(StackError::Conflict(
            "The Stack is currently processing another operation.".into(),
        ));
    }
    let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
    let (StackSpec::Git {
        update_behavior, ..
    }
    | StackSpec::WebEditor {
        update_behavior, ..
    }) = spec;
    let common = spec.common();
    if scheduled && *update_behavior == StackUpdateBehavior::Disabled {
        return Err(validation(
            "Scheduled update checks are disabled for this Stack.",
        ));
    }
    let allowed = if scheduled {
        matches!(
            stack.status,
            StackReleaseStatus::Healthy | StackReleaseStatus::Degraded
        )
    } else {
        matches!(
            stack.status,
            StackReleaseStatus::Healthy
                | StackReleaseStatus::Degraded
                | StackReleaseStatus::Stopped
                | StackReleaseStatus::Paused
        )
    };
    if !allowed {
        return Err(StackError::Conflict(
            "The Stack release state does not support update checks.".to_owned(),
        ));
    }
    let registry_id = common
        .registry_id
        .filter(|id| !id.is_nil())
        .ok_or_else(|| validation("The Stack has no Registry configured."))?;
    let model = parse_compose(compose_files)?;
    let build_services = common
        .build_image_bindings
        .iter()
        .map(|binding| binding.service_name.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let checks = model
        .services
        .into_iter()
        .filter(|service| !build_services.contains(&service.name.to_ascii_lowercase()))
        .filter_map(|service| {
            let image = service.image?;
            let (repository, tag) = split_tagged_image(&image)?;
            Some(ManualStackImageCheck {
                service_name: service.name,
                image_name: image,
                key: StackImageKey {
                    registry_id,
                    repository,
                    tag,
                },
            })
        })
        .collect::<Vec<_>>();
    Ok(checks)
}

pub fn evaluate_manual_stack_updates(
    previous: &StackUpdateState,
    checks: &[ManualStackImageCheck],
    remote_digests: &BTreeMap<StackImageKey, String>,
    checked_at: DateTime<Utc>,
    deployed_digests: Option<&BTreeMap<String, String>>,
    content_changes: Option<&BTreeMap<String, bool>>,
) -> ManualStackUpdateEvaluation {
    let prior = match previous {
        StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state,
        }
        | StackUpdateState::Git {
            recreate_stack_on_new_image_state,
            ..
        } => &recreate_stack_on_new_image_state.auto_update_states,
    }
    .iter()
    .map(|state| (state_key(&state.service_name, &state.image_name), state))
    .collect::<BTreeMap<_, _>>();
    let mut next = Vec::new();
    let mut available = Vec::new();
    let mut newly_detected = Vec::new();
    let mut baselines = 0;
    for check in checks {
        let Some(remote) = remote_digests
            .get(&check.key)
            .filter(|value| !value.trim().is_empty())
        else {
            continue;
        };
        let key = state_key(&check.service_name, &check.image_name);
        let previous = prior.get(&key).copied();
        let deployed = deployed_digests
            .and_then(|values| values.get(&key))
            .filter(|value| !value.trim().is_empty());
        let current = if let Some(deployed) = deployed {
            deployed.clone()
        } else if let Some(previous) =
            previous.filter(|state| !state.current_digest.trim().is_empty())
        {
            previous.current_digest.clone()
        } else {
            baselines += 1;
            remote.clone()
        };
        let update_available = match content_changes {
            Some(changes) => match changes.get(&key) {
                Some(changed) => *changed,
                None => continue,
            },
            None => !citadel_primitives::image_digests_equal(&current, remote),
        };
        next.push(ImageUpdateState {
            service_name: check.service_name.clone(),
            image_name: check.image_name.clone(),
            current_digest: current.clone(),
            remote_digest: Some(remote.clone()),
            last_checked_at: checked_at,
            update_available,
        });
        if update_available {
            let item = StackImageUpdateItem {
                service_name: check.service_name.clone(),
                image_name: check.image_name.clone(),
                current_digest: current,
                remote_digest: remote.clone(),
            };
            if !previous.is_some_and(|state| {
                state.update_available
                    && state
                        .remote_digest
                        .as_deref()
                        .is_some_and(|value| citadel_primitives::image_digests_equal(value, remote))
            }) {
                newly_detected.push(item.clone());
            }
            available.push(item);
        }
    }
    let mut state = previous.clone();
    let (StackUpdateState::Git {
        recreate_stack_on_new_image_state,
        ..
    }
    | StackUpdateState::WebEditor {
        recreate_stack_on_new_image_state,
    }) = &mut state;
    *recreate_stack_on_new_image_state = RecreateStackOnNewImageState {
        auto_update_states: next,
    };
    ManualStackUpdateEvaluation {
        state,
        available_updates: available,
        newly_detected_updates: newly_detected,
        baselines_created: baselines,
    }
}

pub fn state_key(service: &str, image: &str) -> String {
    format!("{service}\n{image}")
}

fn split_tagged_image(image: &str) -> Option<(String, String)> {
    if image.contains('@') || image.contains('$') {
        return None;
    }
    let last_slash = image.rfind('/');
    let colon = image.rfind(':');
    let (repository, tag) =
        match colon.filter(|colon| last_slash.is_none_or(|slash| *colon > slash)) {
            Some(colon) => (&image[..colon], &image[colon + 1..]),
            None => (image, "latest"),
        };
    (!repository.trim().is_empty() && !tag.trim().is_empty())
        .then(|| (repository.to_owned(), tag.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StackDriftPolicy, StackSource};
    use citadel_tags::TagSummary;

    #[test]
    fn update_behavior_support_matches_platform_and_source() {
        use citadel_platforms::PlatformKind;
        for behavior in [
            StackUpdateBehavior::Disabled,
            StackUpdateBehavior::Notify,
            StackUpdateBehavior::StackAutoDeploy,
            StackUpdateBehavior::ServiceAutoDeploy,
        ] {
            let mut value = stack(
                behavior,
                "services:\n  web:\n    image: nginx:alpine\n",
                false,
            );
            let spec = value.spec.as_ref().unwrap();
            assert!(spec.validate_update_behavior(PlatformKind::Docker).is_ok());
            assert_eq!(
                spec.validate_update_behavior(PlatformKind::DockerSwarm)
                    .is_ok(),
                behavior != StackUpdateBehavior::ServiceAutoDeploy
            );
            value.platform_type = PlatformKind::DockerSwarm;
            assert!(build_manual_stack_checks(&value, false).is_ok());
            assert_eq!(
                build_manual_stack_checks(&value, true).is_ok(),
                behavior != StackUpdateBehavior::Disabled
            );
            let git: StackSpec = serde_json::from_value(serde_json::json!({"$type":"Git", "gitRepoId":Uuid::now_v7(), "branch":"main", "composePaths":["compose.yml"], "updateBehavior":behavior})).unwrap();
            assert_eq!(
                git.validate_update_behavior(PlatformKind::Docker).is_ok(),
                behavior != StackUpdateBehavior::ServiceAutoDeploy
            );
            assert_eq!(
                git.validate_update_behavior(PlatformKind::DockerSwarm)
                    .is_ok(),
                behavior != StackUpdateBehavior::ServiceAutoDeploy
            );
        }
    }

    #[test]
    fn check_builder_allows_disabled_on_demand_and_excludes_pinned_and_build_images() {
        let stack = stack(
            StackUpdateBehavior::Disabled,
            "services:\n  api:\n    image: example/api:latest\n  worker:\n    image: example/worker\n  pinned:\n    image: redis@sha256:abc\n",
            true,
        );
        assert!(build_manual_stack_checks(&stack, true).is_err());
        let checks = build_manual_stack_checks(&stack, false).unwrap();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].service_name, "worker");
        assert_eq!(checks[0].key.tag, "latest");
    }

    #[test]
    fn evaluator_creates_a_baseline_then_reports_only_a_changed_digest_once() {
        let stack = stack(
            StackUpdateBehavior::Notify,
            "services:\n  api:\n    image: nginx:latest\n",
            false,
        );
        let checks = build_manual_stack_checks(&stack, false).unwrap();
        let first = BTreeMap::from([(checks[0].key.clone(), "sha256:first".to_owned())]);
        let baseline = evaluate_manual_stack_updates(
            &stack.stack_update_state,
            &checks,
            &first,
            Utc::now(),
            None,
            None,
        );
        assert_eq!(baseline.baselines_created, 1);
        assert!(baseline.available_updates.is_empty());
        let second = BTreeMap::from([(checks[0].key.clone(), "sha256:second".to_owned())]);
        let changed = evaluate_manual_stack_updates(
            &baseline.state,
            &checks,
            &second,
            Utc::now(),
            None,
            None,
        );
        assert_eq!(changed.baselines_created, 0);
        assert_eq!(changed.available_updates.len(), 1);
        assert_eq!(changed.newly_detected_updates.len(), 1);
        let repeated =
            evaluate_manual_stack_updates(&changed.state, &checks, &second, Utc::now(), None, None);
        assert_eq!(repeated.available_updates.len(), 1);
        assert!(repeated.newly_detected_updates.is_empty());
    }

    #[test]
    fn equivalent_digest_forms_do_not_trigger_stack_updates_or_repeat_notifications() {
        let stack = stack(
            StackUpdateBehavior::Notify,
            "services:\n  api:\n    image: nginx:latest\n",
            false,
        );
        let checks = build_manual_stack_checks(&stack, false).unwrap();
        let deployed = BTreeMap::from([(
            state_key(&checks[0].service_name, &checks[0].image_name),
            "nginx@sha256:ABC".to_owned(),
        )]);
        let remote = BTreeMap::from([(checks[0].key.clone(), "sha256:abc".to_owned())]);
        let equal = evaluate_manual_stack_updates(
            &stack.stack_update_state,
            &checks,
            &remote,
            Utc::now(),
            Some(&deployed),
            None,
        );
        assert!(equal.available_updates.is_empty());
        assert!(equal.newly_detected_updates.is_empty());
        assert_eq!(equal.baselines_created, 0);

        let changed_remote =
            BTreeMap::from([(checks[0].key.clone(), "nginx@sha256:new".to_owned())]);
        let changed = evaluate_manual_stack_updates(
            &equal.state,
            &checks,
            &changed_remote,
            Utc::now(),
            Some(&deployed),
            None,
        );
        assert_eq!(changed.newly_detected_updates.len(), 1);
        let bare_remote = BTreeMap::from([(checks[0].key.clone(), "sha256:NEW".to_owned())]);
        let repeated = evaluate_manual_stack_updates(
            &changed.state,
            &checks,
            &bare_remote,
            Utc::now(),
            Some(&deployed),
            None,
        );
        assert_eq!(repeated.available_updates.len(), 1);
        assert!(repeated.newly_detected_updates.is_empty());
    }

    #[test]
    fn equivalent_platform_content_preserves_real_digests_without_update_notifications() {
        let stack = stack(
            StackUpdateBehavior::Notify,
            "services:\n  api:\n    image: nginx:latest\n",
            false,
        );
        let checks = build_manual_stack_checks(&stack, false).unwrap();
        let key = state_key(&checks[0].service_name, &checks[0].image_name);
        let deployed = BTreeMap::from([(key.clone(), "sha256:old-index".into())]);
        let remote = BTreeMap::from([(checks[0].key.clone(), "sha256:new-index".into())]);
        let content = BTreeMap::from([(key, false)]);
        let result = evaluate_manual_stack_updates(
            &stack.stack_update_state,
            &checks,
            &remote,
            Utc::now(),
            Some(&deployed),
            Some(&content),
        );
        assert!(result.available_updates.is_empty() && result.newly_detected_updates.is_empty());
        assert_eq!(result.baselines_created, 0);
        let StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state,
        } = result.state
        else {
            unreachable!()
        };
        let image = &recreate_stack_on_new_image_state.auto_update_states[0];
        assert_eq!(image.current_digest, "sha256:old-index");
        assert_eq!(image.remote_digest.as_deref(), Some("sha256:new-index"));
        assert!(!image.update_available);
    }

    #[test]
    fn git_images_use_supplied_compose_and_preserve_commit_observation() {
        let mut stack = stack(StackUpdateBehavior::Notify, "services: {}", true);
        let common = stack.spec.as_ref().unwrap().common().clone();
        let mut spec: StackSpec = serde_json::from_value(serde_json::json!({
            "$type":"Git", "gitRepoId":Uuid::now_v7(), "branch":"main",
            "commitSha":"a".repeat(40), "composePaths":["compose.yml"], "updateBehavior":"Notify"
        }))
        .unwrap();
        *spec.common_mut() = common;
        stack.stack_source = StackSource::Git;
        stack.stack_update_state = StackUpdateState::new(&spec);
        stack
            .stack_update_state
            .record_applied_commit(&"a".repeat(40), Utc::now());
        stack.spec = Some(spec);
        let original = stack.stack_update_state.clone();
        let checks = build_stack_image_checks(&stack, true, &[
            "services:\n  api:\n    image: app:latest\n  web:\n    image: nginx:old\n  pinned:\n    image: redis@sha256:abc\n  variable:\n    image: ${IMAGE}\n".into(),
            "services:\n  web:\n    image: nginx:alpine\n".into(),
        ]).unwrap();
        assert_eq!(checks.len(), 1);
        assert_eq!(checks[0].image_name, "nginx:alpine");
        let remote = BTreeMap::from([(checks[0].key.clone(), "sha256:new".into())]);
        let deployed = BTreeMap::from([(state_key("web", "nginx:alpine"), "sha256:old".into())]);
        let result = evaluate_manual_stack_updates(
            &original,
            &checks,
            &remote,
            Utc::now(),
            Some(&deployed),
            None,
        );
        let StackUpdateState::Git {
            recreate_stack_on_new_commit_state,
            recreate_stack_on_new_image_state,
        } = result.state
        else {
            panic!("must retain Git state")
        };
        let StackUpdateState::Git {
            recreate_stack_on_new_commit_state: expected,
            ..
        } = original
        else {
            unreachable!()
        };
        assert_eq!(recreate_stack_on_new_commit_state, expected);
        assert!(recreate_stack_on_new_image_state.auto_update_states[0].update_available);
        assert_eq!(result.baselines_created, 0);
    }

    #[test]
    fn image_only_reapply_preserves_pending_git_update() {
        let now = Utc::now();
        let mut state = StackUpdateState::Git {
            recreate_stack_on_new_image_state: Default::default(),
            recreate_stack_on_new_commit_state: crate::RecreateStackOnNewCommitState {
                current_commit_sha: "old".into(),
                remote_commit_sha: Some("new".into()),
                last_checked_at: now,
            },
        };
        let expected = state.clone();
        assert!(!state.record_applied_commit("old", now + chrono::Duration::hours(2)));
        assert_eq!(state, expected);
    }

    fn stack(update_behavior: StackUpdateBehavior, compose: &str, build_api: bool) -> crate::Stack {
        let id = Uuid::now_v7();
        let registry_id = Uuid::now_v7();
        let spec = StackSpec::WebEditor {
            compose_file: compose.to_owned(),
            update_behavior,
            common: crate::StackSpecCommon {
                registry_id: Some(registry_id),
                destroy_before_deploy: false,
                build_image_bindings: if build_api {
                    vec![crate::StackBuildImageBinding {
                        service_name: "api".to_owned(),
                        build_project_id: Uuid::now_v7(),
                        redeploy_on_build: false,
                        resolved_image_reference: None,
                        resolved_digest: None,
                        resolved_build_run_id: None,
                        applied_image_reference: None,
                        applied_digest: None,
                        applied_build_run_id: None,
                        applied_at: None,
                    }]
                } else {
                    Vec::new()
                },
                ..Default::default()
            },
        };
        crate::Stack {
            id,
            name: "stack".to_owned(),
            description: None,
            stack_source: StackSource::WebEditor,
            stack_update_state: StackUpdateState::new(&spec),
            drift_policy: StackDriftPolicy::default(),

            control_state: citadel_primitives::ResourceControlState::Idle,
            current_stack_release_id: Uuid::now_v7(),
            row_version: 0,

            audit: citadel_primitives::AuditMetadata {
                created_at: Utc::now(),
                created_by_actor_id: citadel_primitives::ActorId::new(Uuid::now_v7()),
            },

            status: StackReleaseStatus::Healthy,
            platform_type: citadel_platforms::PlatformKind::Docker,
            platform_id: Some(Uuid::now_v7()),
            has_copied_storage: false,
            version: Some("1".to_owned()),
            spec: Some(spec),
            source: None,
            resource_bindings: None,
            platform_status: citadel_primitives::PlatformStatus::Online,
            platform_name: Some("local".to_owned()),
            tags: Vec::<TagSummary>::new(),
            latest_activity: None,
        }
    }
}
