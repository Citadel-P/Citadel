//! Read-only evidence from the owning daemon. No probe commands or output belong here.
pub use citadel_primitives::health::{
    CheckConfiguration, ContainerHealthObservation, HealthStatus,
};

#[derive(Debug, Clone)]
pub struct ContainerHealthTransition {
    pub observed_at: i64,
    pub first_observed_at: i64,
    pub sample_count: i64,
    pub state: String,
    pub health: Option<ContainerHealthObservation>,
    pub origin: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inactive_and_unavailable_targets_never_reuse_green_results() {
        let mut evidence = ContainerHealthObservation {
            configuration: CheckConfiguration::Enabled,
            result: Some(HealthStatus::Healthy),
            observed_at: 1,
            last_check_at: None,
            failing_streak: None,
            exit_code: None,
            oom_killed: None,
            restart_count: None,
            started_at: None,
            finished_at: None,
        };
        assert_eq!(evidence.status("Running", true), HealthStatus::Healthy);
        for state in ["Exited", "Paused", "Created", "Restarting"] {
            assert_eq!(evidence.status(state, true), HealthStatus::NotRunning);
        }
        assert_eq!(evidence.status("Running", false), HealthStatus::Unknown);
        evidence.configuration = CheckConfiguration::Unknown;
        assert_eq!(evidence.status("Running", true), HealthStatus::Unknown);
        evidence.configuration = CheckConfiguration::NoCheck;
        assert_eq!(evidence.status("Running", true), HealthStatus::NoCheck);
        evidence.configuration = CheckConfiguration::Disabled;
        assert_eq!(evidence.status("Running", true), HealthStatus::Disabled);
    }
}
