use std::collections::{BTreeMap, BTreeSet};

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    ImageUpdateState, RecreateStackOnNewImageState, StackDetails, StackError, StackReleaseStatus,
    StackSpec, StackUpdateBehavior, StackUpdateState, parse_compose, validation,
};

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
    stack: &StackDetails,
    scheduled: bool,
) -> Result<Vec<ManualStackImageCheck>, StackError> {
    if stack.control_state == "Processing" {
        return Err(StackError::Conflict(
            "The Stack is currently processing another operation.".to_owned(),
        ));
    }
    let spec = stack.spec.as_ref().ok_or(StackError::NotFound)?;
    let StackSpec::WebEditor {
        compose_file,
        update_behavior,
        common,
    } = spec
    else {
        return Err(validation(
            "Only Web Editor Stacks support image update checks.",
        ));
    };
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
    let model = parse_compose(std::slice::from_ref(compose_file))?;
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
    if checks.is_empty() {
        return Err(validation(
            "The Stack has no supported tagged Service images to check.",
        ));
    }
    Ok(checks)
}

pub fn evaluate_manual_stack_updates(
    previous: &StackUpdateState,
    checks: &[ManualStackImageCheck],
    remote_digests: &BTreeMap<StackImageKey, String>,
    checked_at: DateTime<Utc>,
    deployed_digests: Option<&BTreeMap<String, String>>,
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
        let update_available = !current.eq_ignore_ascii_case(remote);
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
                        .is_some_and(|value| value.eq_ignore_ascii_case(remote))
            }) {
                newly_detected.push(item.clone());
            }
            available.push(item);
        }
    }
    ManualStackUpdateEvaluation {
        state: StackUpdateState::WebEditor {
            recreate_stack_on_new_image_state: RecreateStackOnNewImageState {
                auto_update_states: next,
            },
        },
        available_updates: available,
        newly_detected_updates: newly_detected,
        baselines_created: baselines,
    }
}

pub fn state_key(service: &str, image: &str) -> String {
    format!("{service}\n{image}")
}

fn split_tagged_image(image: &str) -> Option<(String, String)> {
    if image.contains('@') {
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
    use crate::{StackDriftPolicy, StackSource, TagSummary};

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
        );
        assert_eq!(baseline.baselines_created, 1);
        assert!(baseline.available_updates.is_empty());
        let second = BTreeMap::from([(checks[0].key.clone(), "sha256:second".to_owned())]);
        let changed =
            evaluate_manual_stack_updates(&baseline.state, &checks, &second, Utc::now(), None);
        assert_eq!(changed.baselines_created, 0);
        assert_eq!(changed.available_updates.len(), 1);
        assert_eq!(changed.newly_detected_updates.len(), 1);
        let repeated =
            evaluate_manual_stack_updates(&changed.state, &checks, &second, Utc::now(), None);
        assert_eq!(repeated.available_updates.len(), 1);
        assert!(repeated.newly_detected_updates.is_empty());
    }

    fn stack(update_behavior: StackUpdateBehavior, compose: &str, build_api: bool) -> StackDetails {
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
        StackDetails {
            stack: crate::Stack {
                id,
                name: "stack".to_owned(),
                description: None,
                stack_source: StackSource::WebEditor,
                stack_update_state: StackUpdateState::new(&spec),
                drift_policy: StackDriftPolicy::default(),
                created_at: Utc::now(),
                created_by_actor_id: Uuid::now_v7(),
                control_state: "Idle".to_owned(),
                current_stack_release_id: Uuid::now_v7(),
                row_version: 0,
            },
            status: StackReleaseStatus::Healthy,
            platform_type: "Docker".to_owned(),
            platform_id: Some(Uuid::now_v7()),
            version: Some("1".to_owned()),
            spec: Some(spec),
            source: None,
            resource_bindings: None,
            platform_status: "Online".to_owned(),
            platform_name: Some("local".to_owned()),
            tags: Vec::<TagSummary>::new(),
            latest_activity: None,
            effective_permission: citadel_domain::EffectivePermission::Administrator,
        }
    }
}
