use crate::DeploymentStatus;
use citadel_primitives::health::*;

pub fn assess(
    members: Vec<HealthMember>,
    status: DeploymentStatus,
    online: bool,
    progressing: bool,
    now: i64,
) -> WorkloadHealth {
    assess_health(
        HealthAssessmentInput {
            requirements: Some(vec![HealthRequirement {
                service: String::new(),
                replicas: usize::from(status != DeploymentStatus::Created),
                completed_job: false,
            }]),
            members,
            available: online,
            inactive: status == DeploymentStatus::Created,
            progressing,
            operation_failed: status == DeploymentStatus::Failed,
            ownership_conflict: false,
        },
        now,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stopped_lifecycle_does_not_prove_requested_inactivity() {
        let now = 1_000_000;
        let mut member: HealthMember = serde_json::from_value(serde_json::json!({
            "platformId": "00000000-0000-0000-0000-000000000000",
            "name": "api", "service": "", "state": "Exited", "available": true,
            "observation": {"configuration": "Enabled", "result": "Healthy",
                "observedAt": now, "exitCode": 1, "startedAt": "1970-01-01T00:01:00Z"}
        }))
        .unwrap();
        let assess =
            |member| super::assess(vec![member], DeploymentStatus::Stopped, true, false, now);
        assert!(
            assess(member.clone())
                .attention
                .contains(&HealthAttention::UnexpectedExit)
        );
        member.stop_requested_at = Some(now - 100_000);
        let requested = assess(member.clone());
        assert!(
            !requested
                .attention
                .contains(&HealthAttention::UnexpectedExit)
        );
        assert_eq!(requested.reason, "The applied workload is inactive.");
        // Restarting outside Citadel supersedes the old stop request as well.
        member.observation.as_mut().unwrap().started_at = Some("1970-01-01T00:16:00Z".into());
        assert!(
            assess(member)
                .attention
                .contains(&HealthAttention::UnexpectedExit)
        );
    }
}
