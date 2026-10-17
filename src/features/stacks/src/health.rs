use crate::StackSpec;
use citadel_primitives::health::HealthRequirement;
use serde_yaml_ng::Value;
use std::collections::BTreeSet;

/// Only an immutable applied Web Editor definition can establish Compose membership.
/// Git source text/interpolation that was not retained must remain unknown on reads.
pub fn compose_requirements(applied: Option<&StackSpec>) -> Option<Vec<HealthRequirement>> {
    let text = applied?.compose_file()?;
    if text.contains("${") || text.len() > 2 * 1024 * 1024 {
        return None;
    }
    let doc: Value = serde_yaml_ng::from_str(text).ok()?;
    if doc.get("include").is_some() {
        return None;
    }
    let services = doc.get("services")?.as_mapping()?;
    if applied?.common().env_file_path.is_some()
        && services.values().any(|s| s.get("profiles").is_some())
    {
        return None;
    }
    let mut jobs = BTreeSet::new();
    for service in services.values() {
        if let Some(dependencies) = service.get("depends_on").and_then(Value::as_mapping) {
            for (name, condition) in dependencies {
                if condition.get("condition").and_then(Value::as_str)
                    == Some("service_completed_successfully")
                {
                    jobs.insert(name.as_str()?.to_owned());
                }
            }
        }
    }
    services
        .iter()
        .filter(|(_, service)| {
            service
                .get("profiles")
                .and_then(Value::as_sequence)
                .is_none_or(|p| p.is_empty())
        })
        .map(|(name, service)| {
            if service.get("extends").is_some() || service.get("<<").is_some() {
                return None;
            }
            let name = name.as_str()?;
            let replicas = match service
                .get("deploy")
                .and_then(|d| d.get("replicas"))
                .or_else(|| service.get("scale"))
            {
                None => 1,
                Some(value) => usize::try_from(value.as_u64()?).ok()?,
            };
            Some(HealthRequirement {
                service: name.into(),
                replicas,
                completed_job: jobs.contains(name),
            })
        })
        .collect()
}

pub fn assess(
    members: Vec<citadel_primitives::health::HealthMember>,
    applied: Option<&StackSpec>,
    services: Option<Vec<citadel_primitives::health::AppliedServiceHealth>>,
    status: crate::StackReleaseStatus,
    online: bool,
    progressing: bool,
    now: i64,
) -> citadel_primitives::health::WorkloadHealth {
    use citadel_primitives::health::*;
    let requirements = if let Some(services) = &services {
        if services.is_empty()
            || services.iter().any(|s| {
                s.stale
                    || s.ownership == "OwnershipConflict"
                    || s.stack_service_count
                        .as_ref()
                        .and_then(|v| v.parse::<usize>().ok())
                        != Some(services.len())
                    || s.release_id.is_none()
                    || s.release_id != services[0].release_id
                    || !matches!(
                        s.mode.to_ascii_lowercase().as_str(),
                        "replicated" | "global"
                    )
                    || (s.desired == 0 && !s.mode.eq_ignore_ascii_case("replicated"))
            })
        {
            None
        } else {
            Some(
                services
                    .iter()
                    .map(|s| HealthRequirement {
                        service: s.service.clone(),
                        replicas: s.desired,
                        completed_job: false,
                    })
                    .collect(),
            )
        }
    } else {
        compose_requirements(applied)
    };
    let zero_scaled = requirements
        .as_ref()
        .is_some_and(|rs| !rs.is_empty() && rs.iter().all(|r| r.replicas == 0))
        && members.is_empty()
        && services
            .as_ref()
            .is_none_or(|ss| ss.iter().all(|s| s.running == 0));
    let never_applied = status == crate::StackReleaseStatus::Created;
    assess_health(
        HealthAssessmentInput {
            requirements: if never_applied {
                Some(vec![])
            } else {
                requirements
            },
            members,
            available: online,
            inactive: zero_scaled || never_applied,
            progressing: progressing
                || services.as_ref().is_some_and(|ss| {
                    ss.iter()
                        .any(|s| matches!(s.update_state.as_str(), "updating" | "rollback_started"))
                }),
            operation_failed: matches!(
                status,
                crate::StackReleaseStatus::Failed | crate::StackReleaseStatus::TimedOut
            ) || services.as_ref().is_some_and(|ss| {
                ss.iter()
                    .any(|s| matches!(s.update_state.as_str(), "paused" | "rollback_paused"))
            }),
            ownership_conflict: services
                .as_ref()
                .is_some_and(|ss| ss.iter().any(|s| s.ownership == "OwnershipConflict")),
        },
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use citadel_primitives::health::*;
    fn spec(text: &str) -> StackSpec {
        StackSpec::WebEditor {
            compose_file: text.into(),
            update_behavior: crate::StackUpdateBehavior::Disabled,
            common: Default::default(),
        }
    }
    #[test]
    fn applied_compose_distinguishes_jobs_profiles_and_replicas() {
        let applied = spec(
            "services:\n  api:\n    image: api\n    deploy: {replicas: 3}\n    depends_on:\n      migration: {condition: service_completed_successfully}\n  migration: {image: migration}\n  debug: {image: debug, profiles: [debug]}\n",
        );
        let requirements = compose_requirements(Some(&applied)).unwrap();
        assert_eq!(requirements.len(), 2);
        assert_eq!(
            requirements
                .iter()
                .find(|r| r.service == "api")
                .unwrap()
                .replicas,
            3
        );
        assert!(
            requirements
                .iter()
                .find(|r| r.service == "migration")
                .unwrap()
                .completed_job
        );
    }
    #[test]
    fn unresolved_or_unretained_topology_stays_unknown() {
        assert!(compose_requirements(None).is_none());
        assert!(
            compose_requirements(Some(&spec("services: {api: {image: '${IMAGE}'}}"))).is_none()
        );
        assert!(compose_requirements(Some(&spec("services: {api: {extends: base}}"))).is_none());
    }
    #[test]
    fn missing_swarm_service_and_mixed_release_never_establish_complete_topology() {
        let service = AppliedServiceHealth {
            service: "api".into(),
            mode: "replicated".into(),
            desired: 1,
            running: 0,
            stale: false,
            update_state: String::new(),
            ownership: String::new(),
            stack_service_count: Some("2".into()),
            release_id: Some("release".into()),
        };
        let result = assess(
            vec![],
            None,
            Some(vec![service.clone()]),
            crate::StackReleaseStatus::Healthy,
            true,
            false,
            1_000,
        );
        assert_eq!(result.expected, None);
        let mut other = service.clone();
        other.service = "db".into();
        other.release_id = Some("old".into());
        assert_eq!(
            assess(
                vec![],
                None,
                Some(vec![service.clone(), other.clone()]),
                crate::StackReleaseStatus::Healthy,
                true,
                false,
                1_000
            )
            .expected,
            None
        );
        other.release_id = service.release_id.clone();
        assert_eq!(
            assess(
                vec![],
                None,
                Some(vec![service, other]),
                crate::StackReleaseStatus::Healthy,
                true,
                false,
                1_000
            )
            .missing,
            Some(2)
        );
    }
}
