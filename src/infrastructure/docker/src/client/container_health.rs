//! Health evidence extracted during background observation, without retaining probe output.
use citadel_platforms::container_health::{
    CheckConfiguration, ContainerHealthObservation, HealthStatus,
};
use serde_json::Value;

pub fn observation(document: &Value) -> ContainerHealthObservation {
    citadel_runtime::runtime_metrics::RuntimeWork::ContainerHealthObservation.units(1);
    let check = document
        .pointer("/Config/Healthcheck/Test")
        .and_then(Value::as_array);
    let configuration = match check.and_then(|v| v.first()).and_then(Value::as_str) {
        Some("NONE") => CheckConfiguration::Disabled,
        Some("CMD" | "CMD-SHELL") => CheckConfiguration::Enabled,
        Some(_) => CheckConfiguration::Unknown,
        None if document.get("Config").is_some_and(Value::is_object) => CheckConfiguration::NoCheck,
        _ => CheckConfiguration::Unknown,
    };
    let result = match document
        .pointer("/State/Health/Status")
        .and_then(Value::as_str)
    {
        Some("healthy") => Some(HealthStatus::Healthy),
        Some("unhealthy") => Some(HealthStatus::Unhealthy),
        Some("starting") => Some(HealthStatus::Starting),
        _ => None,
    };
    let time = |path| {
        document
            .pointer(path)
            .and_then(Value::as_str)
            .filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok_and(|t| t.timestamp() > 0))
            .map(str::to_owned)
    };
    let last_check_at = document
        .pointer("/State/Health/Log")
        .and_then(Value::as_array)
        .and_then(|log| log.last())
        .and_then(|v| v.get("End"))
        .and_then(Value::as_str)
        .filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok_and(|t| t.timestamp() > 0))
        .map(str::to_owned);
    ContainerHealthObservation {
        configuration,
        result,
        observed_at: chrono::Utc::now().timestamp_millis(),
        last_check_at,
        failing_streak: document
            .pointer("/State/Health/FailingStreak")
            .and_then(Value::as_i64)
            .filter(|n| *n >= 0),
        exit_code: document.pointer("/State/ExitCode").and_then(Value::as_i64),
        oom_killed: document
            .pointer("/State/OOMKilled")
            .and_then(Value::as_bool),
        restart_count: document
            .get("RestartCount")
            .and_then(Value::as_i64)
            .filter(|n| *n >= 0),
        started_at: time("/State/StartedAt"),
        finished_at: time("/State/FinishedAt"),
    }
}

/// Redact before truncating so a secret crossing the output limit cannot leak a prefix.
pub fn bounded_probe_output(output: &str, environment: &[String]) -> String {
    let mut output = output.to_owned();
    for (name, value) in environment.iter().filter_map(|s| s.split_once('=')) {
        if !value.is_empty() && citadel_primitives::is_sensitive_environment_name(name) {
            output = output.replace(value, "[REDACTED]");
        }
    }
    output.chars().take(4096).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn missing_evidence_is_unknown_and_exit_137_does_not_invent_oom() {
        let value = observation(&json!({"State":{"ExitCode":137}}));
        assert_eq!(value.configuration, CheckConfiguration::Unknown);
        assert_eq!(value.oom_killed, None);
        assert_eq!(value.exit_code, Some(137));
        assert_eq!(
            observation(&json!({"Config":{}})).configuration,
            CheckConfiguration::NoCheck
        );
        assert_eq!(
            observation(&json!({"Config":{"Healthcheck":{"Test":["NONE"]}}})).configuration,
            CheckConfiguration::Disabled
        );
    }
    #[test]
    fn summaries_do_not_include_commands_or_output() {
        let value = observation(
            &json!({"Config":{"Healthcheck":{"Test":["CMD-SHELL","secret command"]}},
            "State":{"Health":{"Status":"unhealthy","FailingStreak":3,"Log":[{"Output":"secret output","End":"2026-10-16T12:00:00Z"}]}}}),
        );
        assert_eq!(value.result, Some(HealthStatus::Unhealthy));
        assert_eq!(value.failing_streak, Some(3));
        assert!(value.last_check_at.is_some());
        assert!(!serde_json::to_string(&value).unwrap().contains("secret"));
    }
}
