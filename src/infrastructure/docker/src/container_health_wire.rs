use citadel_contracts::citadel::shared_models::v1::ContainerHealthObservation as Wire;
use citadel_platforms::container_health::{
    CheckConfiguration, ContainerHealthObservation, HealthStatus,
};

pub fn to_wire(value: ContainerHealthObservation) -> Wire {
    Wire {
        configuration: format!("{:?}", value.configuration),
        result: value.result.map(|v| format!("{v:?}")),
        observed_at: value.observed_at,
        last_check_at: value.last_check_at,
        failing_streak: value.failing_streak,
        exit_code: value.exit_code,
        oom_killed: value.oom_killed,
        restart_count: value.restart_count,
        started_at: value.started_at,
        finished_at: value.finished_at,
    }
}

pub fn from_wire(value: Wire) -> ContainerHealthObservation {
    ContainerHealthObservation {
        configuration: match value.configuration.as_str() {
            "Enabled" => CheckConfiguration::Enabled,
            "Disabled" => CheckConfiguration::Disabled,
            "NoCheck" => CheckConfiguration::NoCheck,
            _ => CheckConfiguration::Unknown,
        },
        result: match value.result.as_deref() {
            Some("Healthy") => Some(HealthStatus::Healthy),
            Some("Unhealthy") => Some(HealthStatus::Unhealthy),
            Some("Starting") => Some(HealthStatus::Starting),
            _ => None,
        },
        observed_at: value.observed_at,
        last_check_at: value.last_check_at,
        failing_streak: value.failing_streak.filter(|v| *v >= 0),
        exit_code: value.exit_code,
        oom_killed: value.oom_killed,
        restart_count: value.restart_count.filter(|v| *v >= 0),
        started_at: value.started_at,
        finished_at: value.finished_at,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoding_does_not_renew_replayed_health_evidence() {
        let value = Wire {
            configuration: "Enabled".into(),
            result: Some("Healthy".into()),
            observed_at: 1000,
            ..Default::default()
        };
        assert_eq!(from_wire(value.clone()).observed_at, 1000);
        assert_eq!(from_wire(value).observed_at, 1000);
    }
}
