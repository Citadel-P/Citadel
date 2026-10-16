//! Read-only evidence from the owning daemon. No probe commands or output belong here.
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct ContainerHealthTransition {
    pub observed_at: i64,
    pub first_observed_at: i64,
    pub sample_count: i64,
    pub state: String,
    pub health: Option<ContainerHealthObservation>,
    pub origin: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CheckConfiguration {
    Enabled,
    Disabled,
    NoCheck,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    Healthy,
    Unhealthy,
    Starting,
    NoCheck,
    Disabled,
    NotRunning,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContainerHealthObservation {
    pub configuration: CheckConfiguration,
    pub result: Option<HealthStatus>,
    /// Time of an actual inspection, never a heartbeat or statistics timestamp.
    pub observed_at: i64,
    pub last_check_at: Option<String>,
    pub failing_streak: Option<i64>,
    pub exit_code: Option<i64>,
    pub oom_killed: Option<bool>,
    pub restart_count: Option<i64>,
    pub started_at: Option<String>,
    pub finished_at: Option<String>,
}

impl ContainerHealthObservation {
    pub fn status(&self, state: &str, available: bool) -> HealthStatus {
        if !available {
            return HealthStatus::Unknown;
        }
        match state.to_ascii_lowercase().as_str() {
            "created" | "exited" | "dead" | "paused" | "restarting" | "removing" => {
                return HealthStatus::NotRunning;
            }
            "running" => {}
            _ => return HealthStatus::Unknown,
        }
        match self.configuration {
            CheckConfiguration::NoCheck => HealthStatus::NoCheck,
            CheckConfiguration::Disabled => HealthStatus::Disabled,
            CheckConfiguration::Unknown => HealthStatus::Unknown,
            CheckConfiguration::Enabled => match self.result {
                Some(HealthStatus::Healthy | HealthStatus::Unhealthy | HealthStatus::Starting) => {
                    self.result.unwrap()
                }
                _ => HealthStatus::Unknown,
            },
        }
    }
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
