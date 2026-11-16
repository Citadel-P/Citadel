//! Small projection for periodic health collection; full UI inspection stays separate.
use super::{DockerClient, DockerError};
use citadel_platforms::container_health::{
    CheckConfiguration, ContainerHealthObservation, HealthStatus,
};
use serde::Deserialize;

pub struct ContainerHealthInspection {
    pub id: Option<String>,
    pub state: String,
    pub health: ContainerHealthObservation,
}

impl DockerClient {
    pub async fn inspect_container_health(
        &self,
        id: &str,
    ) -> Result<ContainerHealthInspection, DockerError> {
        super::transport::validate_identifier(id)?;
        let bytes = self.health_inspection_bytes(id).await?;
        parse(&bytes).map_err(Into::into)
    }
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Inspection {
    id: Option<String>,
    config: Option<Config>,
    state: Option<State>,
    restart_count: Option<i64>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Config {
    healthcheck: Option<Healthcheck>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Healthcheck {
    test: Option<Vec<String>>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct State {
    status: Option<String>,
    health: Option<Health>,
    exit_code: Option<i64>,
    #[serde(rename = "OOMKilled")]
    oom_killed: Option<bool>,
    started_at: Option<String>,
    finished_at: Option<String>,
}
#[derive(Default, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Health {
    status: Option<String>,
    failing_streak: Option<i64>,
    log: Option<Vec<Log>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Log {
    end: Option<String>,
}

fn valid_time(value: Option<String>) -> Option<String> {
    value.filter(|s| chrono::DateTime::parse_from_rfc3339(s).is_ok_and(|t| t.timestamp() > 0))
}

fn parse(bytes: &[u8]) -> Result<ContainerHealthInspection, serde_json::Error> {
    let doc = match serde_json::from_slice::<Inspection>(bytes) {
        Ok(doc) => doc,
        // Preserve the existing permissive treatment of unexpected field types.
        // Only unusual documents pay for the full tree; invalid JSON still fails.
        Err(_) => {
            let doc: serde_json::Value = serde_json::from_slice(bytes)?;
            return Ok(ContainerHealthInspection {
                id: doc["Id"].as_str().map(str::to_owned),
                state: doc["State"]["Status"].as_str().unwrap_or("unknown").into(),
                health: super::container_health::observation(&doc),
            });
        }
    };
    let configuration = match doc.config {
        None => CheckConfiguration::Unknown,
        Some(config) => match config
            .healthcheck
            .and_then(|c| c.test)
            .and_then(|t| t.into_iter().next())
            .as_deref()
        {
            Some("NONE") => CheckConfiguration::Disabled,
            Some("CMD" | "CMD-SHELL") => CheckConfiguration::Enabled,
            Some(_) => CheckConfiguration::Unknown,
            None => CheckConfiguration::NoCheck,
        },
    };
    let state = doc.state.unwrap_or_default();
    let health = state.health.unwrap_or_default();
    let result = match health.status.as_deref() {
        Some("healthy") => Some(HealthStatus::Healthy),
        Some("unhealthy") => Some(HealthStatus::Unhealthy),
        Some("starting") => Some(HealthStatus::Starting),
        _ => None,
    };
    citadel_runtime::runtime_metrics::RuntimeWork::ContainerHealthObservation.units(1);
    Ok(ContainerHealthInspection {
        id: doc.id,
        state: state.status.unwrap_or_else(|| "unknown".into()),
        health: ContainerHealthObservation {
            configuration,
            result,
            observed_at: chrono::Utc::now().timestamp_millis(),
            last_check_at: valid_time(
                health
                    .log
                    .and_then(|mut log| log.pop())
                    .and_then(|log| log.end),
            ),
            failing_streak: health.failing_streak.filter(|n| *n >= 0),
            exit_code: state.exit_code,
            oom_killed: state.oom_killed,
            restart_count: doc.restart_count.filter(|n| *n >= 0),
            started_at: valid_time(state.started_at),
            finished_at: valid_time(state.finished_at),
        },
    })
}

#[cfg(test)]
#[path = "health_inspection_tests.rs"]
mod tests;
